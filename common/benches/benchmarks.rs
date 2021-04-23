use common::world::{storage::*, *};
use criterion::{criterion_group, criterion_main, Criterion};
use rayon::ThreadPoolBuilder;
use tempfile::tempdir;

const COMPRESSION_LEVEL: u8 = 6;

fn save_single_chunk(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let storage = ChunkDiskStorage::initialize(dir.path(), COMPRESSION_LEVEL);

    let chunk = ChunkData::create(ChunkCoordinate::new(0, 0, 0));

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
    let storage = ChunkDiskStorage::initialize(dir.path(), COMPRESSION_LEVEL);

    let chunk = ChunkData::create(ChunkCoordinate::new(0, 0, 0));
    storage.save_chunk(&chunk).unwrap();

    let profiler = pprof::ProfilerGuard::new(100).unwrap();

    c.bench_function("load_single_chunk", |b| {
        b.iter(|| {
            assert!(storage.get_chunk(ChunkCoordinate::new(0, 0, 0)).unwrap().is_some());
        })
    });

    if let Ok(report) = profiler.report().build() {
        let file = std::fs::File::create("flamegraphs/load_single_chunk.svg").unwrap();
        report.flamegraph(file).unwrap();
    };
}

fn bulk_load(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let storage = ChunkDiskStorage::initialize(dir.path(), COMPRESSION_LEVEL);

    let radius = 4;

    let mut chunks = Vec::new();

    println!("Generating chunks...");
    for y in -radius..=radius {
        for x in -radius..=radius {
            for z in -radius..=radius {
                chunks.push(ChunkData::create(ChunkCoordinate::new(x, y, z)));
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

    {
        let profiler = pprof::ProfilerGuard::new(100).unwrap();

        c.bench_function("bulk_load_multi_thread", |b| {
            b.iter(|| {
                thread_pool.scope(|scope| {
                    for y in -radius..=radius {
                        for x in -radius..=radius {
                            for z in -radius..=radius {
                                // Only hand a reference to the thread.
                                let storage = &storage;
                                scope.spawn(move |_| {
                                    assert!(storage.get_chunk(ChunkCoordinate::new(x, y, z)).unwrap().is_some());
                                })
                            }
                        }
                    }
                });
            })
        });
        if let Ok(report) = profiler.report().build() {
            let file = std::fs::File::create("flamegraphs/bulk_load_multi_thread.svg").unwrap();
            report.flamegraph(file).unwrap();
        };
    }

    {
        let profiler = pprof::ProfilerGuard::new(100).unwrap();

        c.bench_function("bulk_load_single_thread", |b| {
            b.iter(|| {
                for y in -radius..=radius {
                    for x in -radius..=radius {
                        for z in -radius..=radius {
                            assert!(storage.get_chunk(ChunkCoordinate::new(x, y, z)).unwrap().is_some());
                        }
                    }
                }
            })
        });
        if let Ok(report) = profiler.report().build() {
            let file = std::fs::File::create("flamegraphs/bulk_load_single_thread.svg").unwrap();
            report.flamegraph(file).unwrap();
        };
    }
}

fn bulk_save(c: &mut Criterion) {
    let radius = 4;

    let mut chunks = Vec::new();

    println!("Generating chunks...");
    for y in -radius..=radius {
        for x in -radius..=radius {
            for z in -radius..=radius {
                chunks.push(ChunkData::create(ChunkCoordinate::new(x, y, z)));
            }
        }
    }

    let thread_pool = ThreadPoolBuilder::new().num_threads(0).build().unwrap();

    {
        let profiler = pprof::ProfilerGuard::new(100).unwrap();

        c.bench_function("bulk_save_multi_thread", |b| {
            // We have to start fresh each time.
            let dir = tempdir().unwrap();
            let storage = ChunkDiskStorage::initialize(dir.path(), COMPRESSION_LEVEL);
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
            let file = std::fs::File::create("flamegraphs/bulk_save_multi_thread.svg").unwrap();
            report.flamegraph(file).unwrap();
        };
    }

    {
        let profiler = pprof::ProfilerGuard::new(100).unwrap();

        c.bench_function("bulk_save_single_thread", |b| {
            // We have to start fresh each time.
            let dir = tempdir().unwrap();
            let storage = ChunkDiskStorage::initialize(dir.path(), COMPRESSION_LEVEL);
            b.iter(|| {
                for chunk in &chunks {
                    // Only hand a reference to the thread.
                    storage.save_chunk(chunk).unwrap();
                }
            })
        });
        if let Ok(report) = profiler.report().build() {
            let file = std::fs::File::create("flamegraphs/bulk_save_single_thread.svg").unwrap();
            report.flamegraph(file).unwrap();
        };
    }
}

criterion_group!(terrain_io, load_single_chunk, save_single_chunk, bulk_load, bulk_save);
criterion_main!(terrain_io);
