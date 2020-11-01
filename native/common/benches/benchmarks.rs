use common::world::*;
use criterion::{criterion_group, criterion_main, Criterion};
use tempfile::tempfile;

fn iterate_fresh_file(c: &mut Criterion) {
    let index = TerrainDiskStorage::initialize(tempfile().unwrap(), tempfile().unwrap()).unwrap();

    c.bench_function("iterate_fresh_file", |b| {
        b.iter(|| {
            index.get_chunks_in_range((-50, -50, -50), (50, 50, 50), |_chunk| Ok(())).unwrap();
        })
    });
}

fn iterate_pregen_file(c: &mut Criterion) {
    let index = TerrainDiskStorage::initialize(tempfile().unwrap(), tempfile().unwrap()).unwrap();
    index.get_chunks_in_range((-50, -50, -50), (50, 50, 50), |_chunk| Ok(())).unwrap();

    c.bench_function("iterate_pregen_file", |b| {
        b.iter(|| {
            index.get_chunks_in_range((-50, -50, -50), (50, 50, 50), |_chunk| Ok(())).unwrap();
        })
    });
}

fn single_chunk_fresh_file(c: &mut Criterion) {
    let index = TerrainDiskStorage::initialize(tempfile().unwrap(), tempfile().unwrap()).unwrap();

    c.bench_function("single_chunk_fresh_file", |b| {
        b.iter(|| {
            index.get_chunk(0, 0, 0, |_chunk| Ok(())).unwrap();
        })
    });
}

criterion_group!(terrain_io, iterate_fresh_file, iterate_pregen_file, single_chunk_fresh_file);
criterion_main!(terrain_io);
