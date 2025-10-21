use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use vectorlawdb_core::lsm::LSMTree;
use tempfile::tempdir;

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("lsm_insert");

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let dir = tempdir().unwrap();
            let tree = LSMTree::new(dir.path()).unwrap();

            b.iter(|| {
                for i in 0..size {
                    let key = format!("key_{}", i).into_bytes();
                    let value = format!("value_{}", i).into_bytes();
                    tree.put(key, value).unwrap();
                }
            });
        });
    }

    group.finish();
}

fn bench_get(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let tree = LSMTree::new(dir.path()).unwrap();

    // Pre-populate
    for i in 0..10000 {
        let key = format!("key_{}", i).into_bytes();
        let value = format!("value_{}", i).into_bytes();
        tree.put(key, value).unwrap();
    }

    c.bench_function("lsm_get", |b| {
        b.iter(|| {
            let key = format!("key_{}", black_box(5000)).into_bytes();
            tree.get(&key).unwrap()
        });
    });
}

criterion_group!(benches, bench_insert, bench_get);
criterion_main!(benches);
