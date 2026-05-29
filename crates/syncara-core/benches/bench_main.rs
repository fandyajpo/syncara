//! Syncara micro-benchmark suite.
//!
//! Measures the cost of the core request-processing pipeline
//! (routing, load-balancer selection, brain scoring, and security
//! validation) across a range of configuration sizes.
//!
//! Run:
//!   cargo bench -p syncara-core
//!
//! For flamegraph profiling:
//!   cargo bench -p syncara-core -- --profile-time 30
//!
//! For filtering to a specific benchmark group:
//!   cargo bench -p syncara-core -- router/exact_host

#[path = "bench_modules/balancer.rs"]
mod balancer;

#[path = "bench_modules/routing.rs"]
mod routing;

#[path = "bench_modules/security.rs"]
mod security;

use criterion::{criterion_group, criterion_main, Criterion};

fn all_groups(c: &mut Criterion) {
    balancer::group(c);
    routing::group(c);
    security::group(c);
}

criterion_group! {
    name = syncara_bench;
    config = Criterion::default()
        .significance_level(0.05)
        .sample_size(100)
        .warm_up_time(std::time::Duration::from_secs(2))
        .measurement_time(std::time::Duration::from_secs(5));
    targets = all_groups
}

criterion_main!(syncara_bench);
