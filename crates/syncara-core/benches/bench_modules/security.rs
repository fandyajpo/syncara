use std::net::IpAddr;
use std::time::Duration;

use criterion::{black_box, Criterion};
use http_body_util::Full;
use bytes::Bytes;

use syncara_core::security::{RateLimiter, SecurityConfig, SecurityLayer, validator};

fn bench_validate_valid(c: &mut Criterion) {
    let mut group = c.benchmark_group("security/validate");

    let req = http::Request::builder()
        .uri("/api/health")
        .header("host", "example.com")
        .header("content-type", "application/json")
        .header("accept", "*/*")
        .body(Full::new(Bytes::new()))
        .unwrap();

    group.bench_function("valid", |b| {
        b.iter(|| {
            let result = validator::validate_request(black_box(&req));
            black_box(result)
        });
    });

    group.finish();
}

fn bench_validate_long_uri(c: &mut Criterion) {
    let mut group = c.benchmark_group("security/validate");

    let long_uri = format!("/{}", "a".repeat(8000));
    let req = http::Request::builder()
        .uri(&long_uri)
        .header("host", "x")
        .body(Full::new(Bytes::new()))
        .unwrap();

    group.bench_function("near_limit_uri", |b| {
        b.iter(|| {
            let result = validator::validate_request(black_box(&req));
            black_box(result)
        });
    });

    group.finish();
}

fn bench_validate_many_headers(c: &mut Criterion) {
    let mut group = c.benchmark_group("security/validate");

    let mut builder = http::Request::builder().uri("/").header("host", "x");
    for i in 0..64 {
        builder = builder.header(format!("x-header-{}", i), "value");
    }
    let req = builder.body(Full::new(Bytes::new())).unwrap();

    group.bench_function("many_headers", |b| {
        b.iter(|| {
            let result = validator::validate_request(black_box(&req));
            black_box(result)
        });
    });

    group.finish();
}

fn bench_security_layer(c: &mut Criterion) {
    let mut group = c.benchmark_group("security/layer");

    // Build a SecurityLayer with rate limiting enabled.
    let mut cfg = SecurityConfig::default();
    cfg.rate_limit = Some(syncara_core::security::RateLimitConfig {
        enabled: true,
        requests_per_minute: 10_000,
    });
    let layer = SecurityLayer::new(&cfg);

    let req = http::Request::builder()
        .uri("/api/test")
        .header("host", "example.com")
        .body(Full::new(Bytes::new()))
        .unwrap();

    let client_ip: IpAddr = "10.0.0.55".parse().unwrap();

    group.bench_function("validate_and_rate_check", |b| {
        b.iter(|| {
            let v = layer.validate(black_box(&req));
            let r = layer.check_rate_limit(black_box(client_ip));
            black_box((v, r))
        });
    });

    group.finish();
}

fn bench_rate_limiter_cold(c: &mut Criterion) {
    let mut group = c.benchmark_group("security/rate_limiter");

    let rl = RateLimiter::new(1000, Duration::from_secs(60));
    let ips: Vec<IpAddr> = (0..1000)
        .map(|i| format!("10.0.0.{}", i % 256).parse().unwrap())
        .collect();

    group.bench_function("cold_distinct_ips", |b| {
        b.iter(|| {
            for ip in &ips {
                let ok = rl.check(black_box(*ip));
                black_box(ok);
            }
        });
    });

    group.finish();
}

fn bench_rate_limiter_warm(c: &mut Criterion) {
    let mut group = c.benchmark_group("security/rate_limiter");

    let rl = RateLimiter::new(1000, Duration::from_secs(60));
    let ip: IpAddr = "10.0.0.1".parse().unwrap();
    // Warm the cache.
    for _ in 0..10 {
        rl.check(ip);
    }

    group.bench_function("warm_single_ip", |b| {
        b.iter(|| {
            let ok = rl.check(black_box(ip));
            black_box(ok)
        });
    });

    group.finish();
}

pub fn group(c: &mut Criterion) {
    bench_validate_valid(c);
    bench_validate_long_uri(c);
    bench_validate_many_headers(c);
    bench_security_layer(c);
    bench_rate_limiter_cold(c);
    bench_rate_limiter_warm(c);
}
