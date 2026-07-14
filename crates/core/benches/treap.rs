//! A benchmark for treap implementation.

use std::hint::black_box;
use std::num::{NonZeroU8, NonZeroU16};

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

use fvi_core::piece::Pieces;
use fvi_core::view::{View, ViewSize};

const SALT: u16 = 0xCAFE;
const PIECE_SIZE: u16 = 8;
const TREE_SIZES: &[usize] = &[16, 64, 256, 768];

fn view_size(size: u16) -> ViewSize {
    ViewSize::from(NonZeroU16::new(size).expect("nonzero view size"))
}

fn view(buf: u8, ver: u16, off: u16, size: u16) -> View {
    View::new(
        NonZeroU8::new(buf).expect("nonzero buffer"),
        ver,
        off,
        view_size(size),
    )
    .expect("valid view")
}

/// Builds exactly `pieces` nodes.
///
/// Alternating versions prevents adjacent views from coalescing.
fn fragmented(pieces: usize, salt: u16) -> Pieces {
    let mut tree = Pieces::new(salt);

    for index in 0..pieces {
        let source_off = u16::try_from(index)
            .expect("benchmark size fits u16")
            .checked_mul(PIECE_SIZE)
            .expect("source offset fits u16");

        let insert_off = u16::try_from(tree.len()).expect("tree length fits u16");

        tree.insert(
            insert_off,
            #[expect(clippy::unwrap_used, reason = "should not be failed")]
            view(1, u16::try_from(index % 2).unwrap(), source_off, PIECE_SIZE),
        );
    }

    tree
}

fn bench_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("treap/build");

    for &pieces in TREE_SIZES {
        group.throughput(Throughput::Elements(pieces as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(pieces),
            &pieces,
            |b, &pieces| {
                b.iter(|| {
                    let tree = fragmented(pieces, black_box(SALT));
                    black_box(tree.len())
                });
            },
        );
    }

    group.finish();
}

fn bench_insert_middle(c: &mut Criterion) {
    let mut group = c.benchmark_group("treap/insert_middle");

    for &pieces in TREE_SIZES {
        group.bench_with_input(
            BenchmarkId::from_parameter(pieces),
            &pieces,
            |b, &pieces| {
                b.iter_batched_ref(
                    || fragmented(pieces, SALT),
                    |tree| {
                        // TREE_SIZES are even, so len / 2 is a piece
                        // boundary. Adding 3 forces an internal split.
                        #[expect(clippy::unwrap_used, reason = "should not be failed")]
                        let insert_off = u16::try_from(tree.len() / 2 + 3).unwrap();

                        tree.insert(black_box(insert_off), black_box(view(7, 0x1FFE, 0, 3)));

                        black_box(tree.len());
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_remove_middle(c: &mut Criterion) {
    let mut group = c.benchmark_group("treap/remove_middle");

    for &pieces in TREE_SIZES {
        group.bench_with_input(
            BenchmarkId::from_parameter(pieces),
            &pieces,
            |b, &pieces| {
                b.iter_batched_ref(
                    || fragmented(pieces, SALT),
                    |tree| {
                        // Starts inside one piece and ends inside the
                        // following piece, forcing two boundary splits.
                        #[expect(clippy::unwrap_used, reason = "should not be failed")]
                        let remove_off = u16::try_from(tree.len() / 2 + 3).unwrap();

                        let orphan = tree
                            .remove(black_box(remove_off), black_box(view_size(10)))
                            .expect("range is valid");

                        black_box(tree.len());
                        _ = black_box(orphan);
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }

    group.finish();
}

fn boundary_case(coalescing: bool) -> Pieces {
    let mut tree = Pieces::new(SALT);

    // [buf=1, ver=0, off=0, size=1]
    tree.insert(0, view(1, 0, 0, 1));

    // This starts at source offset 11, so it does not initially
    // coalesce with the first piece.
    let right_ver = u16::from(!coalescing);
    tree.insert(1, view(1, right_ver, 11, 2));

    tree
}

fn bench_boundary_concat(c: &mut Criterion) {
    let mut group = c.benchmark_group("treap/boundary_concat");

    group.bench_function("coalesce", |b| {
        b.iter_batched_ref(
            || boundary_case(true),
            |tree| {
                // Produces:
                // [0,1], [0,11], [11,2]
                //
                // The last two views are coalescible into [0,13].
                tree.insert(1, black_box(view(1, 0, 0, 11)));

                black_box(tree.len());
                black_box(tree.iter().count());
            },
            BatchSize::LargeInput,
        );
    });

    group.bench_function("no_coalesce", |b| {
        b.iter_batched_ref(
            || boundary_case(false),
            |tree| {
                tree.insert(1, black_box(view(1, 0, 0, 11)));

                black_box(tree.len());
                black_box(tree.iter().count());
            },
            BatchSize::LargeInput,
        );
    });

    group.finish();
}

fn bench_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("treap/iteration");

    for &pieces in TREE_SIZES {
        let tree = fragmented(pieces, SALT);
        group.throughput(Throughput::Elements(pieces as u64));

        group.bench_with_input(BenchmarkId::from_parameter(pieces), &pieces, |b, _| {
            b.iter(|| {
                let total: u32 = tree.iter().map(|piece| piece.size().get()).sum();

                black_box(total)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_build,
    bench_insert_middle,
    bench_remove_middle,
    bench_boundary_concat,
    bench_iteration,
);

criterion_main!(benches);
