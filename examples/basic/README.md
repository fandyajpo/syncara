# Basic Reverse Proxy

## With Docker

```sh
docker compose up
```

## Without Docker

```sh
# Terminal 1: start two backends
docker run -d --rm -p 9001:80 nginx:alpine
docker run -d --rm -p 9002:80 nginx:alpine

# Terminal 2: start syncara
cargo run --release -p syncara -- start -c examples/basic/syncara.yml

# Terminal 3: test
curl http://localhost:8080/
curl http://localhost:9090/metrics
```
