/// WebSocket upgrade detection and tunneling.
///
/// Intercepts HTTP requests with `Upgrade: websocket`, performs the
/// upgrade handshake with the upstream, then pipes raw TCP frames
/// bidirectionally between client and upstream.
use std::convert::Infallible;

use bytes::{Bytes, BytesMut};
use http::HeaderMap;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::{info, warn};

#[cfg(unix)]
use std::os::unix::io::{FromRawFd, IntoRawFd};
#[cfg(windows)]
use std::os::windows::io::{FromRawSocket, IntoRawSocket};

use socket2::{Socket, TcpKeepalive};

use crate::balancer::BusyGuard;
use crate::dns;
use crate::observability::metrics;
use crate::security::WsPermit;

/// Check whether `req` is a WebSocket upgrade request.
pub fn is_websocket_upgrade<B>(req: &Request<B>) -> bool {
    req.headers()
        .get("upgrade")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(',')
                .any(|part| part.trim().eq_ignore_ascii_case("websocket"))
        })
}

/// Perform a WebSocket upgrade proxying.
///
/// Takes ownership of the request, establishes a raw TCP connection to
/// the upstream, forwards the request bytes, reads the upstream's 101
/// response, then uses hyper's built-in upgrade mechanism to hand the
/// client connection off to a background task that pipes frames
/// bidirectionally.
pub async fn proxy_websocket(
    mut req: Request<Incoming>,
    upstream: &str,
    _busy_guard: BusyGuard,
    _ws_permit: WsPermit,
) -> Result<Response<http_body_util::combinators::BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    // 1. Connect to the upstream server via raw TCP (with DNS caching).
    let mut upstream_stream = match dns::connect(upstream).await {
        Ok(s) => s,
        Err(e) => {
            warn!(%upstream, error = %e, "websocket upstream connection failed");
            return Ok(error_response(
                StatusCode::BAD_GATEWAY,
                &format!("502 Bad Gateway — upstream connection failed: {e}"),
            ));
        }
    };

    // Set TCP keepalive if configured.
    if let Some(ka) = crate::security::get().keepalive {
        upstream_stream = apply_keepalive(upstream_stream, ka);
    }

    // 2. Serialize the incoming request as raw HTTP/1.1 bytes and send
    //    it to the upstream.  WebSocket upgrade requests have no body,
    //    so we only need to send the request line + headers.
    let request_bytes = serialize_request(&req);
    if let Err(e) = upstream_stream.write_all(&request_bytes).await {
        warn!(%upstream, error = %e, "websocket upstream write failed");
        return Ok(error_response(
            StatusCode::BAD_GATEWAY,
            &format!("502 Bad Gateway — upstream write failed: {e}"),
        ));
    }

    // 3. Read the upstream's HTTP response headers.
    let (status, headers) = match read_upstream_response(&mut upstream_stream).await {
        Ok(r) => r,
        Err(e) => {
            warn!(%upstream, error = %e, "websocket upstream response failed");
            return Ok(error_response(
                StatusCode::BAD_GATEWAY,
                &format!("502 Bad Gateway — upstream response failed: {e}"),
            ));
        }
    };

    if status != StatusCode::SWITCHING_PROTOCOLS {
        warn!(%upstream, %status, "upstream did not return 101 Switching Protocols");
        return Ok(error_response(
            StatusCode::BAD_GATEWAY,
            "502 Bad Gateway — upstream did not switch protocols",
        ));
    }

    // 4. Register the hyper upgrade.  This call inserts state into the
    //    request that causes hyper to resolve the returned `OnUpgrade`
    //    future once the 101 response we return from this service
    //    function has been sent to the client.
    let on_upgrade = hyper::upgrade::on(&mut req);

    // 5. Spawn a background task that awaits the upgrade and then pipes
    //    raw TCP frames in both directions.  The BusyGuard is moved into
    //    the task so it stays alive for the tunnel's full lifetime.  The
    //    WS active connections gauge is incremented before the spawn and
    //    decremented when the tunnel closes.
    let m = metrics::get();
    m.ws_connections_active.inc();
    let ws_timeout = crate::security::get().timeouts.websocket;

    tokio::spawn(async move {
        let _guard = _busy_guard;
        let _ws = _ws_permit;
        match on_upgrade.await {
            Ok(upgraded) => {
                let mut client_io = TokioIo::new(upgraded);
                let mut upstream_io = upstream_stream;
                info!("websocket tunnel established, piping bidirectionally");
                match tokio::time::timeout(ws_timeout, tokio::io::copy_bidirectional(&mut client_io, &mut upstream_io)).await {
                    Ok(Ok((from_client, from_server))) => info!("websocket tunnel closed (client->upstream: {from_client} bytes, upstream->client: {from_server} bytes)"),
                    Ok(Err(e)) => info!("websocket tunnel ended: {}", e),
                    Err(_) => info!("websocket tunnel timed out after {ws_timeout:?}"),
                }
            }
            Err(e) => {
                warn!("websocket upgrade failed: {}", e);
            }
        }
        metrics::get().ws_connections_active.dec();
    });

    // 6. Build and return the 101 Switching Protocols response with the
    //    upstream's upgrade headers forwarded to the client.
    let mut response = Response::new(
        Full::new(Bytes::new())
            .map_err(|_: Infallible| unreachable!())
            .boxed(),
    );
    *response.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    *response.headers_mut() = headers;
    Ok(response)
}

/// Apply TCP keepalive to a tokio TcpStream, consuming and returning it.
#[cfg(unix)]
fn apply_keepalive(stream: tokio::net::TcpStream, ka: std::time::Duration) -> tokio::net::TcpStream {
    match stream.into_std() {
        Ok(std_stream) => {
            let socket = Socket::from(std_stream);
            let _ = socket.set_tcp_keepalive(&TcpKeepalive::new().with_time(ka));
            let fd = socket.into_raw_fd();
            let std_stream = unsafe { std::net::TcpStream::from_raw_fd(fd) };
            tokio::net::TcpStream::from_std(std_stream)
                .expect("failed to reclaim TcpStream after keepalive config")
        }
        Err(e) => {
            warn!("failed to convert to std TcpStream for keepalive: {e}");
            panic!("TcpStream::into_std failed: {e}");
        }
    }
}

/// Windows-compatible keepalive using socket2 directly.
#[cfg(windows)]
fn apply_keepalive(stream: tokio::net::TcpStream, ka: std::time::Duration) -> tokio::net::TcpStream {
    match stream.into_std() {
        Ok(std_stream) => {
            let socket = Socket::from(std_stream);
            let _ = socket.set_tcp_keepalive(&TcpKeepalive::new().with_time(ka));
            let sock = socket.into_raw_socket();
            let std_stream = unsafe { std::net::TcpStream::from_raw_socket(sock) };
            tokio::net::TcpStream::from_std(std_stream)
                .expect("failed to reclaim TcpStream after keepalive config")
        }
        Err(e) => {
            warn!("failed to convert to std TcpStream for keepalive: {e}");
            panic!("TcpStream::into_std failed: {e}");
        }
    }
}

/// Reconstruct an HTTP/1.1 request as raw bytes from the parsed hyper
/// request, preserving the original headers.
fn serialize_request<B>(req: &Request<B>) -> Bytes {
    let method = req.method();
    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");

    let mut buf = BytesMut::new();
    buf.extend_from_slice(format!("{method} {path} HTTP/1.1\r\n").as_bytes());
    for (name, value) in req.headers() {
        buf.extend_from_slice(name.as_str().as_bytes());
        buf.extend_from_slice(b": ");
        buf.extend_from_slice(value.as_bytes());
        buf.extend_from_slice(b"\r\n");
    }
    buf.extend_from_slice(b"\r\n");
    buf.freeze()
}

/// Read the HTTP status line and headers from the upstream's response.
///
/// Reads byte-by-byte until `\r\n\r\n` is found, then parses the status
/// line and headers.  Returns the [`StatusCode`] and [`HeaderMap`].
///
/// This deliberately reads *only* the header section so that the stream
/// is positioned at the first byte of the WebSocket frame data when it
/// returns.
async fn read_upstream_response(
    stream: &mut TcpStream,
) -> Result<(StatusCode, HeaderMap), String> {
    use tokio::io::AsyncReadExt;

    let mut buf = Vec::with_capacity(512);
    let mut byte = [0u8; 1];

    loop {
        stream
            .read_exact(&mut byte)
            .await
            .map_err(|e| format!("failed to read upstream response: {e}"))?;
        buf.push(byte[0]);
        if buf.len() >= 4 && buf[buf.len() - 4..] == *b"\r\n\r\n" {
            break;
        }
    }

    let header_str =
        String::from_utf8(buf).map_err(|e| format!("invalid upstream response UTF-8: {e}"))?;
    let mut lines = header_str.lines();

    // Parse status line: "HTTP/1.1 101 Switching Protocols"
    let status_line = lines.next().ok_or("empty response from upstream")?;
    let status_code: u16 = status_line
        .split(' ')
        .nth(1)
        .ok_or_else(|| format!("malformed status line: {status_line}"))?
        .parse()
        .map_err(|e| format!("invalid status code in upstream response: {e}"))?;
    let status =
        StatusCode::from_u16(status_code).map_err(|e| format!("invalid status code: {e}"))?;

    // Parse headers
    let mut headers = HeaderMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim();
            let value = value.trim();
            if let (Ok(name), Ok(value)) = (
                http::HeaderName::from_bytes(name.as_bytes()),
                http::HeaderValue::from_str(value),
            ) {
                headers.append(name, value);
            }
        }
    }

    Ok((status, headers))
}

/// Build a static error response with the given status and body text.
fn error_response(
    status: StatusCode,
    body: &str,
) -> Response<http_body_util::combinators::BoxBody<Bytes, hyper::Error>> {
    let boxed_body = Full::new(Bytes::from(body.to_owned()))
        .map_err(|_: Infallible| unreachable!())
        .boxed();
    Response::builder()
        .status(status)
        .body(boxed_body)
        .expect("static error response is valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::Full;
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

    #[test]
    fn test_is_websocket_upgrade_true() {
        let req = Request::builder()
            .header("upgrade", "websocket")
            .header("connection", "upgrade")
            .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
            .header("sec-websocket-version", "13")
            .body(Full::new(Bytes::new()))
            .unwrap();
        assert!(is_websocket_upgrade(&req));
    }

    #[test]
    fn test_is_websocket_upgrade_case_insensitive() {
        let req = Request::builder()
            .header("Upgrade", "WebSocket")
            .header("Connection", "Upgrade")
            .body(Full::new(Bytes::new()))
            .unwrap();
        assert!(is_websocket_upgrade(&req));
    }

    #[test]
    fn test_is_websocket_upgrade_false() {
        let req = Request::builder()
            .header("content-type", "application/json")
            .body(Full::new(Bytes::new()))
            .unwrap();
        assert!(!is_websocket_upgrade(&req));
    }

    #[test]
    fn test_serialize_request() {
        let req = Request::builder()
            .method("GET")
            .uri("/chat")
            .header("host", "example.com")
            .header("upgrade", "websocket")
            .header("connection", "upgrade")
            .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
            .header("sec-websocket-version", "13")
            .body(Full::new(Bytes::new()))
            .unwrap();

        let bytes = serialize_request(&req);
        let as_str = String::from_utf8(bytes.to_vec()).unwrap();

        assert!(as_str.starts_with("GET /chat HTTP/1.1\r\n"));
        assert!(as_str.contains("host: example.com\r\n"));
        assert!(as_str.contains("upgrade: websocket\r\n"));
        assert!(as_str.ends_with("\r\n\r\n"));
    }

    #[tokio::test]
    async fn test_read_upstream_response_101() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let resp = b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\n";
            stream.write_all(resp).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        });

        let mut client = TcpStream::connect(addr).await.unwrap();
        let (status, headers) = read_upstream_response(&mut client).await.unwrap();

        assert_eq!(status, StatusCode::SWITCHING_PROTOCOLS);
        assert_eq!(
            headers.get("upgrade").unwrap().to_str().unwrap(),
            "websocket"
        );
        assert_eq!(
            headers
                .get("sec-websocket-accept")
                .unwrap()
                .to_str()
                .unwrap(),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_read_upstream_response_200() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let resp = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 12\r\n\r\nHello World!";
            stream.write_all(resp).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        });

        let mut client = TcpStream::connect(addr).await.unwrap();
        let (status, _headers) = read_upstream_response(&mut client).await.unwrap();

        assert_eq!(status, StatusCode::OK);

        server_handle.await.unwrap();
    }
}
