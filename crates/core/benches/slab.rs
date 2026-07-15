//! A benchmark for slab implementation.

use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use fvi_core::collections::Slab;

const SALT: u64 = 0xDEAD_BEEF;
const SIZES: &[u64] = &[16, 256, 768];

const fn xorsft(seed: u64) -> u64 {
    let mut x = seed;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x * 0x2545_F491_4F6C_DD1D
}

fn fragmented(size: u64) -> Slab<u64> {
    let mut slab = Slab::new();

    for i in 0..(size * 2) {
        _ = slab.insert(i);
    }

    for i in 0..size {
        let j = xorsft(i ^ SALT);
        let j = j % (slab.len() as u64);
        let j = j & (usize::MAX as u64);

        #[expect(clippy::cast_possible_truncation, reason = "j & usize::MAX")]
        (_ = slab.remove(j as usize));
    }

    slab
}

macro_rules! bench {
    ($($fn_name:ident($size:ident, $slab:ident = $init:block) $blk:block)+) => {
        $(fn $fn_name(c: &mut Criterion) {
            let mut g = c.benchmark_group(
                stringify!($fn_name)
                    .replace("_", "/")
                    .replace("bench", "slab")
            );

            for &$size in SIZES {
                g.throughput(Throughput::Elements($size));

                g.bench_with_input(BenchmarkId::from_parameter($size), &$size, |b, &$size| {
                    b.iter_batched(
                        || $init,
                        |$slab| {
                            let mut $slab = black_box($slab);
                            $blk
                            black_box($slab.len());
                        },
                        BatchSize::SmallInput,
                    );
                });
            }

            g.finish();
        })+
    };
}

bench![
    bench_insert_clean(size, slab = { Slab::new() }) {
        for i in 0..size {
            _ = slab.insert(black_box(i));
        }
    }

    bench_insert_dirty(size, slab = { fragmented(size) }) {
        for i in 0..size {
            _ = slab.insert(black_box(i));
        }
    }

    bench_remove_dirty(size, slab = { fragmented(size) }) {
        for i in 0..size {
            #[expect(clippy::cast_possible_truncation, reason = "size < usize::MAX")]
            (_ = slab.remove(black_box((i % (size * 2)) as usize)));
        }
    }
];

criterion_group!(
    benches,
    bench_insert_clean,
    bench_insert_dirty,
    bench_remove_dirty,
);

criterion_main!(benches);
