use criterion::{black_box, criterion_group, criterion_main, Criterion};
use scraper::{Html, Selector};
use pypi_search;

fn bench_query() {
     let rt = tokio::runtime::Runtime::new().unwrap();
     rt.block_on(pypi_search::query_pypi("gitlab", 3)).unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("bench_query", |b| b.iter(|| bench_query()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
