//! Benchmarks for the piece table implementation.

use core::hint::black_box;
use core::num::NonZero;
use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use fvi_core::collections::Table;
use fvi_core::collections::table::{Context, Version};
use fvi_core::piece::{Buffer, PieceDesc};

const SALT: usize = 0xDEAD_BEEF;
const SIZES: &[u64] = &[1_000, 10_000, 100_000];
const UNDO: u32 = 32;
const CHURN_CYCLES: u32 = 64;
const CHURN_FIRST_VERSION: u32 = 100;

const fn context(version: u32) -> Context {
    Context {
        undo_max_len: NonZero::new(UNDO).unwrap(),
        version: Version::new(version).unwrap(),
    }
}

const fn original(i: u64) -> PieceDesc {
    // Gaps prevent adjacent pieces from coarsening into a single node.
    PieceDesc {
        buffer: Buffer::Original,
        start: 2 * i,
        end: 2 * i + 1,
    }
}

fn populate(table: &mut Table, size: u64) {
    let cx = context(0);
    for i in 0..size {
        table.insert(&cx, black_box(i), black_box(original(i)));
    }
}

fn populated(size: u64) -> Table {
    let mut table = Table::new(SALT, &context(0));
    populate(&mut table, size);
    // Settle deferred metadata before isolated update/deletion measurements.
    table.update(&context(0));
    table
}

fn delete_sparse(table: &mut Table, size: u64) {
    // Delete 1% of the nodes, evenly scattered. Descending offsets stay valid.
    let cx = context(1);
    for i in (0..size / 100).rev() {
        let offset = i * 100;
        table.remove(&cx, offset, offset + 1);
    }
}

fn delete_dense(table: &mut Table, size: u64) {
    table.remove(&context(1), size / 4, 3 * size / 4);
}

fn churn(table: &mut Table, size: u64) {
    // One inserted/deleted piece per cycle; the original document stays intact.
    // The history boundary advances through retained tombstones during the run.
    for cycle in 0..CHURN_CYCLES {
        let start = u64::from(cycle) * 2;
        table.insert(
            &context(CHURN_FIRST_VERSION + cycle * 2),
            black_box(size / 2),
            black_box(PieceDesc {
                buffer: Buffer::Append,
                start,
                end: start + 1,
            }),
        );
        table.remove(
            &context(CHURN_FIRST_VERSION + cycle * 2 + 1),
            size / 2,
            size / 2 + 1,
        );
    }
}

#[derive(Clone, Copy)]
enum Workload {
    Build,
    ForwardUpdate,
    DeleteRetained,
    ExpireSparse,
    ExpireDense,
    EditChurn,
}

impl Workload {
    const ALL: [Self; 6] = [
        Self::Build,
        Self::ForwardUpdate,
        Self::DeleteRetained,
        Self::ExpireSparse,
        Self::ExpireDense,
        Self::EditChurn,
    ];

    /// Prepare a fixture outside timing.
    fn prepare(self, size: u64) -> Table {
        if matches!(self, Self::Build) {
            return Table::new(SALT, &context(0));
        }
        let mut table = populated(size);
        match self {
            Self::ExpireSparse => delete_sparse(&mut table, size),
            Self::ExpireDense => delete_dense(&mut table, size),
            _ => return table,
        }
        table.update(&context(1));
        table
    }

    /// Execute one complete workload, without fixture setup or destruction.
    fn run(self, table: &mut Table, size: u64) {
        match self {
            Self::Build => {
                populate(table, size);
                table.update(&context(0));
            }
            Self::ForwardUpdate | Self::ExpireSparse | Self::ExpireDense => {
                table.update(black_box(&context(UNDO)));
            }
            Self::DeleteRetained => delete_dense(table, size),
            Self::EditChurn => churn(table, size),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::ForwardUpdate => "forward_update",
            Self::DeleteRetained => "delete_retained",
            Self::ExpireSparse => "expire_sparse",
            Self::ExpireDense => "expire_dense",
            Self::EditChurn => "edit_churn",
        }
    }

    const fn operations(self, size: u64) -> u64 {
        match self {
            Self::Build => size,
            Self::ForwardUpdate => 1,
            Self::DeleteRetained | Self::ExpireDense => size / 2,
            Self::ExpireSparse => size / 100,
            Self::EditChurn => CHURN_CYCLES as u64 * 2,
        }
    }
}

fn bench_workload(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    size: u64,
    workload: Workload,
) {
    let mut fixture = None;
    group.bench_function(BenchmarkId::from_parameter(size), |b| {
        let fixture = fixture.get_or_insert_with(|| workload.prepare(size));
        if matches!(workload, Workload::ForwardUpdate) {
            let mut table = fixture.clone();
            let mut version = UNDO;
            b.iter(|| {
                table.update(black_box(&context(version)));
                black_box(&mut table);
                version += 1;
            });
        } else {
            b.iter_batched_ref(
                || fixture.clone(),
                |table| {
                    workload.run(table, size);
                    black_box(table);
                },
                BatchSize::PerIteration,
            );
        }
    });
}

fn bench_table(c: &mut Criterion) {
    for workload in Workload::ALL {
        let mut group = c.benchmark_group(format!("table/{}", workload.name()));
        for &size in SIZES {
            group.throughput(Throughput::Elements(workload.operations(size)));
            bench_workload(&mut group, size, workload);
        }
        group.finish();
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(20)
        .warm_up_time(Duration::from_millis(200))
        .measurement_time(Duration::from_secs(1));
    targets = bench_table
}
criterion_main!(benches);
