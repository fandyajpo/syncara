use criterion::{black_box, Criterion};

use syncara_core::config::Config;
use syncara_core::routing::Router;

/// Build a config with `n` routes for benchmarking.
fn bench_config(n: usize) -> Config {
    let mut cfg = Config::default();
    for i in 0..n {
        cfg.routes.push(syncara_core::config::RouteConfig {
            host: Some(format!("host-{}.example.com", i)),
            path: Some(format!("/api/v{}/users", i)),
            pool: format!("pool-{}", i),
            proxy: None,
            websocket: false,
        });
    }
    cfg
}

/// Build a config with `n` wildcard host routes.
fn bench_config_wildcard(n: usize) -> Config {
    let mut cfg = Config::default();
    for i in 0..n {
        cfg.routes.push(syncara_core::config::RouteConfig {
            host: Some(format!("*.example.com")),
            path: Some(format!("/api/v{}/items", i)),
            pool: format!("pool-{}", i),
            proxy: None,
            websocket: false,
        });
    }
    cfg
}

fn bench_route_match_exact(c: &mut Criterion) {
    let mut group = c.benchmark_group("router/exact_host");

    for &n in &[10, 100, 1000] {
        let cfg = bench_config(n);
        let router = Router::new(&cfg);

        group.bench_with_input(
            criterion::BenchmarkId::new("match", n),
            &n,
            |b, _| {
                b.iter(|| {
                    let route = router.route(
                        Some("host-5.example.com"),
                        "/api/v5/users",
                    );
                    black_box(route)
                });
            },
        );
    }
    group.finish();
}

fn bench_route_wildcard(c: &mut Criterion) {
    let mut group = c.benchmark_group("router/wildcard_host");

    for &n in &[10, 100, 1000] {
        let cfg = bench_config_wildcard(n);
        let router = Router::new(&cfg);

        group.bench_with_input(
            criterion::BenchmarkId::new("match", n),
            &n,
            |b, _| {
                b.iter(|| {
                    let route = router.route(
                        Some("app.example.com"),
                        "/api/v5/items",
                    );
                    black_box(route)
                });
            },
        );
    }
    group.finish();
}

fn bench_route_path_prefix(c: &mut Criterion) {
    let mut group = c.benchmark_group("router/path_prefix");

    let mut cfg = Config::default();
    // Add a deep path prefix route
    cfg.routes.push(syncara_core::config::RouteConfig {
        host: None,
        path: Some("/api/v2/very/deep/path/users".into()),
        pool: "deep-pool".into(),
        proxy: None,
        websocket: false,
    });
    let router = Router::new(&cfg);

    group.bench_function("short", |b| {
        b.iter(|| {
            let route = router.route(None, "/api/v2/very/deep/path/users/profile");
            black_box(route)
        });
    });

    group.finish();
}

fn bench_route_no_match(c: &mut Criterion) {
    let mut group = c.benchmark_group("router/no_match");

    for &n in &[10, 100, 1000] {
        let cfg = bench_config(n);
        let router = Router::new(&cfg);

        group.bench_with_input(
            criterion::BenchmarkId::new("scan_all", n),
            &n,
            |b, _| {
                b.iter(|| {
                    let route = router.route(
                        Some("unknown.example.com"),
                        "/no/match/path",
                    );
                    black_box(route)
                });
            },
        );
    }
    group.finish();
}

fn bench_route_single_host_multi_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("router/single_host_multi_path");

    let mut cfg = Config::default();
    let host = "app.example.com";
    for i in 0..50 {
        cfg.routes.push(syncara_core::config::RouteConfig {
            host: Some(host.into()),
            path: Some(format!("/api/v{}/", i)),
            pool: format!("pool-{}", i),
            proxy: None,
            websocket: false,
        });
    }
    let router = Router::new(&cfg);

    group.bench_function("match_end", |b| {
        b.iter(|| {
            let route = router.route(
                Some("app.example.com"),
                "/api/v49/users",
            );
            black_box(route)
        });
    });

    group.finish();
}

pub fn group(c: &mut Criterion) {
    bench_route_match_exact(c);
    bench_route_wildcard(c);
    bench_route_path_prefix(c);
    bench_route_no_match(c);
    bench_route_single_host_multi_path(c);
}
