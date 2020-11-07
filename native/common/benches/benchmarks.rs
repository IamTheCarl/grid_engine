use common::world::*;
use criterion::{criterion_group, criterion_main, Criterion};
use tempfile::tempfile;

fn iterate_fresh_file(c: &mut Criterion) {
    let profiler = pprof::ProfilerGuard::new(100).unwrap();

    c.bench_function("iterate_fresh_file", |b| {
        b.iter(|| {
            let mut index = TerrainDiskStorage::initialize(tempfile().unwrap(), tempfile().unwrap()).unwrap();
            for y in -50..=50 {
                for x in -50..=50 {
                    for z in -50..=50 {
                        index.get_or_create_chunk(x, y, z).unwrap();
                    }
                }
            }
        })
    });

    if let Ok(report) = profiler.report().build() {
        let file = std::fs::File::create("flamegraphs/iterate_fresh_file.svg").unwrap();
        report.flamegraph(file).unwrap();
    };
}

fn iterate_pregen_file(c: &mut Criterion) {
    let mut index = TerrainDiskStorage::initialize(tempfile().unwrap(), tempfile().unwrap()).unwrap();
    for y in -50..=50 {
        for x in -50..=50 {
            for z in -50..=50 {
                index.get_or_create_chunk(x, y, z).unwrap();
            }
        }
    }

    let profiler = pprof::ProfilerGuard::new(100).unwrap();

    c.bench_function("iterate_pregen_file", |b| {
        b.iter(|| {
            for y in -50..=50 {
                for x in -50..=50 {
                    for z in -50..=50 {
                        index.get_chunk(x, y, z).unwrap();
                    }
                }
            }
        })
    });

    if let Ok(report) = profiler.report().build() {
        let file = std::fs::File::create("flamegraphs/iterate_pregen_file.svg").unwrap();
        report.flamegraph(file).unwrap();
    };
}

fn single_chunk_fresh_file(c: &mut Criterion) {
    let profiler = pprof::ProfilerGuard::new(100).unwrap();

    c.bench_function("single_chunk_fresh_file", |b| {
        b.iter(|| {
            let mut index = TerrainDiskStorage::initialize(tempfile().unwrap(), tempfile().unwrap()).unwrap();
            index.get_or_create_chunk(0, 0, 0).unwrap();
        })
    });

    if let Ok(report) = profiler.report().build() {
        let file = std::fs::File::create("flamegraphs/single_chunk_fresh_file.svg").unwrap();
        report.flamegraph(file).unwrap();
    };
}

criterion_group!(terrain_io, single_chunk_fresh_file, iterate_fresh_file, iterate_pregen_file);
criterion_main!(terrain_io);
