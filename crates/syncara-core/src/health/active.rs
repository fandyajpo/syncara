use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::dns;

/// Result of a single active health check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckResult {
    Pass,
    Fail(String),
}

/// The type of health check to perform.
#[derive(Debug, Clone)]
pub enum CheckMode {
    /// TCP connect only — checks if the port is accepting connections.
    Tcp,
    /// HTTP GET to a specific path — checks for a 2xx or 3xx response.
    Http { path: String },
}

/// Run a single active health check against an upstream.
///
/// * `Tcp` — attempts a TCP connect within the timeout.
/// * `Http` — connects, sends `GET {path} HTTP/1.0`, and checks for a
///   successful status code (200–399).
pub async fn run_check(addr: &str, mode: &CheckMode, timeout: Duration) -> CheckResult {
    let connect = dns::connect(addr);
    match tokio::time::timeout(timeout, connect).await {
        Ok(Ok(stream)) => match mode {
            CheckMode::Tcp => CheckResult::Pass,
            CheckMode::Http { path } => check_http_health(stream, path, timeout).await,
        },
        Ok(Err(e)) => CheckResult::Fail(format!("tcp connect failed: {e}")),
        Err(_) => CheckResult::Fail("tcp connect timed out".to_string()),
    }
}

/// Send an HTTP GET request to the upstream and validate the response.
async fn check_http_health(
    mut stream: TcpStream,
    path: &str,
    timeout: Duration,
) -> CheckResult {
    let request = format!(
        "GET {path} HTTP/1.0\r\nHost: upstream\r\nConnection: close\r\n\r\n"
    );

    let timed = tokio::time::timeout(timeout, async {
        stream.write_all(request.as_bytes()).await.map_err(|e| format!("write error: {e}"))?;

        let mut buf = vec![0u8; 4096];
        let n = stream.read(&mut buf).await.map_err(|e| format!("read error: {e}"))?;

        if n == 0 {
            return Err("empty response".to_string());
        }

        let response = String::from_utf8_lossy(&buf[..n]);
        let status_line = response.lines().next().unwrap_or("");
        let status_code = status_line
            .split(' ')
            .nth(1)
            .and_then(|s| s.parse::<u16>().ok());

        match status_code {
            Some(code) if (200..400).contains(&code) => Ok(CheckResult::Pass),
            Some(code) => Ok(CheckResult::Fail(format!("HTTP status {code}"))),
            None => Err("could not parse HTTP status line".to_string()),
        }
    })
    .await;

    match timed {
        Ok(result) => result.unwrap_or_else(|e| CheckResult::Fail(e)),
        Err(_) => CheckResult::Fail("http check timed out".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tcp_check_refused_is_fail() {
        let result = run_check(
            "127.0.0.1:19999",
            &CheckMode::Tcp,
            Duration::from_secs(2),
        )
        .await;
        assert!(matches!(result, CheckResult::Fail(_)));
    }

    #[tokio::test]
    async fn tcp_check_timeout_is_fail() {
        // 10.255.255.1 is a non-routable address that will time out
        let result = run_check(
            "10.255.255.1:80",
            &CheckMode::Tcp,
            Duration::from_millis(100),
        )
        .await;
        assert!(matches!(result, CheckResult::Fail(_)));
    }
}
