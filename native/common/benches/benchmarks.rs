use common::world::*;
use criterion::{criterion_group, criterion_main, Criterion};
use rayon::{ThreadPool, ThreadPoolBuilder};
use tempfile::tempdir;

fn save_single_chunk(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let storage = ChunkDiskStorage::initialize(dir.path());

    let chunk = ChunkData::create(0, 0, 0);

    let profiler = pprof::ProfilerGuard::new(100).unwrap();

    c.bench_function("save_single_chunk", |b| {
        b.iter(|| {
            storage.save_chunk(&chunk).unwrap();
        })
    });

    if let Ok(report) = profiler.report().build() {
        let file = std::fs::File::create("flamegraphs/save_single_chunk.svg").unwrap();
        report.flamegraph(file).unwrap();
    };
}

fn load_single_chunk(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let storage = ChunkDiskStorage::initialize(dir.path());

    let chunk = ChunkData::create(0, 0, 0);
    storage.save_chunk(&chunk).unwrap();

    let profiler = pprof::ProfilerGuard::new(100).unwrap();

    c.bench_function("load_single_chunk", |b| {
        b.iter(|| {
            assert!(storage.get_chunk(0, 0, 0).unwrap().is_some());
        })
    });

    if let Ok(report) = profiler.report().build() {
        let file = std::fs::File::create("flamegraphs/load_single_chunk.svg").unwrap();
        report.flamegraph(file).unwrap();
    };
}

fn bulk_load(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let storage = ChunkDiskStorage::initialize(dir.path());

    let radius = 4;

    let mut chunks = Vec::new();

    println!("Generating chunks...");
    for y in -radius..=radius {
        for x in -radius..=radius {
            for z in -radius..=radius {
                chunks.push(ChunkData::create(x, y, z));
            }
        }
    }

    // Remove mutability.
    let chunks = chunks;

    println!("Saving chunks...");
    for chunk in &chunks {
        storage.save_chunk(chunk).unwrap();
    }

    let thread_pool = ThreadPoolBuilder::new().num_threads(0).build().unwrap();

    let profiler = pprof::ProfilerGuard::new(100).unwrap();

    c.bench_function("bulk_load", |b| {
        b.iter(|| {
            thread_pool.scope(|scope| {
                for y in -radius..=radius {
                    for x in -radius..=radius {
                        for z in -radius..=radius {
                            // Only hand a reference to the thread.
                            let storage = &storage;
                            scope.spawn(move |_| {
                                assert!(storage.get_chunk(x, y, z).unwrap().is_some());
                            })
                        }
                    }
                }
            });
        })
    });

    if let Ok(report) = profiler.report().build() {
        let file = std::fs::File::create("flamegraphs/bulk_load.svg").unwrap();
        report.flamegraph(file).unwrap();
    };
}

fn bulk_save(c: &mut Criterion) {
    let radius = 4;

    let mut chunks = Vec::new();

    println!("Generating chunks...");
    for y in -radius..=radius {
        for x in -radius..=radius {
            for z in -radius..=radius {
                chunks.push(ChunkData::create(x, y, z));
            }
        }
    }

    let thread_pool = ThreadPoolBuilder::new().num_threads(0).build().unwrap();

    let profiler = pprof::ProfilerGuard::new(100).unwrap();

    c.bench_function("bulk_save", |b| {
        // We have to start fresh each time.
        let dir = tempdir().unwrap();
        let storage = ChunkDiskStorage::initialize(dir.path());

        b.iter(|| {
            thread_pool.scope(|scope| {
                for chunk in &chunks {
                    // Only hand a reference to the thread.
                    let storage = &storage;
                    scope.spawn(move |_| {
                        storage.save_chunk(chunk).unwrap();
                    });
                }
            });
        })
    });

    if let Ok(report) = profiler.report().build() {
        let file = std::fs::File::create("flamegraphs/bulk_save.svg").unwrap();
        report.flamegraph(file).unwrap();
    };
}

criterion_group!(terrain_io, load_single_chunk, save_single_chunk, bulk_load, bulk_save);
criterion_main!(terrain_io);
