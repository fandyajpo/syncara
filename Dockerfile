FROM rust:1.85 AS builder

WORKDIR /app
COPY . .
RUN cargo build --release -p syncara

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/syncara /usr/local/bin/syncara
EXPOSE 8080 9090
ENTRYPOINT ["syncara"]
CMD ["start"]
