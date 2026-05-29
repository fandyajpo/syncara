use criterion::{black_box, Criterion};

use syncara_core::balancer::{RequestContext, UpstreamPool};
use syncara_core::brain::tracker;
use syncara_core::config::brain::BrainConfig;
use syncara_core::config::upstream::{Strategy, UpstreamConfig, UpstreamPoolConfig};

fn make_pool(n: usize, strategy: Strategy) -> UpstreamPool {
    let cfg = UpstreamPoolConfig {
        name: "bench".into(),
        strategy,
        connections: None,
        upstreams: (0..n)
            .map(|i| UpstreamConfig {
                addr: format!("10.0.0.{}:3000", i + 1),
                weight: if i % 3 == 0 { 3 } else { 1 },
                max_connections: None,
            })
            .collect(),
        health: None,
        session: None,
        brain: Some(BrainConfig {
            latency_aware: true,
            health_aware: true,
            websocket_pressure_aware: false,
        }),
    };
    UpstreamPool::from_config(&cfg)
}

fn bench_round_robin(c: &mut Criterion) {
    let mut group = c.benchmark_group("balancer/round_robin");

    for &n in &[10, 100, 1000] {
        let pool = make_pool(n, Strategy::RoundRobin);
        let ctx = RequestContext::new("10.0.0.1".parse().unwrap());

        group.bench_with_input(
            criterion::BenchmarkId::new("select", n),
            &n,
            |b, _| {
                b.iter(|| {
                    let acquired = pool.acquire(black_box(&ctx));
                    black_box(acquired)
                });
            },
        );
    }
    group.finish();
}

fn bench_least_connections(c: &mut Criterion) {
    let mut group = c.benchmark_group("balancer/least_connections");

    for &n in &[10, 100, 1000] {
        let pool = make_pool(n, Strategy::LeastConnections);
        let ctx = RequestContext::new("10.0.0.1".parse().unwrap());

        group.bench_with_input(
            criterion::BenchmarkId::new("select", n),
            &n,
            |b, _| {
                b.iter(|| {
                    let acquired = pool.acquire(black_box(&ctx));
                    black_box(acquired)
                });
            },
        );
    }
    group.finish();
}

fn bench_weighted(c: &mut Criterion) {
    let mut group = c.benchmark_group("balancer/weighted");

    for &n in &[10, 100, 1000] {
        let pool = make_pool(n, Strategy::Weighted);
        let ctx = RequestContext::new("10.0.0.1".parse().unwrap());

        group.bench_with_input(
            criterion::BenchmarkId::new("select", n),
            &n,
            |b, _| {
                b.iter(|| {
                    let acquired = pool.acquire(black_box(&ctx));
                    black_box(acquired)
                });
            },
        );
    }
    group.finish();
}

fn bench_ip_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("balancer/ip_hash");

    for &n in &[10, 100, 1000] {
        let pool = make_pool(n, Strategy::IpHash);
        let ctx = RequestContext::new("10.0.0.1".parse().unwrap());

        group.bench_with_input(
            criterion::BenchmarkId::new("select", n),
            &n,
            |b, _| {
                b.iter(|| {
                    let acquired = pool.acquire(black_box(&ctx));
                    black_box(acquired)
                });
            },
        );
    }
    group.finish();
}

fn bench_brain(c: &mut Criterion) {
    let mut group = c.benchmark_group("balancer/brain");

    for &n in &[10, 100, 1000] {
        let pool = make_pool(n, Strategy::Brain);
        let ctx = RequestContext::new("10.0.0.1".parse().unwrap());

        group.bench_with_input(
            criterion::BenchmarkId::new("select", n),
            &n,
            |b, _| {
                b.iter(|| {
                    let acquired = pool.acquire(black_box(&ctx));
                    black_box(acquired)
                });
            },
        );
    }
    group.finish();
}

pub fn group(c: &mut Criterion) {
    tracker::init();

    bench_round_robin(c);
    bench_least_connections(c);
    bench_weighted(c);
    bench_ip_hash(c);
    bench_brain(c);
}
