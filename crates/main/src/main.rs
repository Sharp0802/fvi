//! Non-android/wasm entrypoint module.

use fvi::App;
use fvi::render::RenderConfig;
use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::runtime::Builder;
use tracing::Level;
use tracing_subscriber::field::RecordFields;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt::FormatFields;
use tracing_subscriber::fmt::time::Uptime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug, Clone, Copy)]
struct Omit;

impl<'w> FormatFields<'w> for Omit {
    fn format_fields<R: RecordFields>(
        &self,
        _writer: tracing_subscriber::fmt::format::Writer<'w>,
        _fields: R,
    ) -> std::fmt::Result {
        Ok(())
    }
}

fn main() {
    let rt = Builder::new_multi_thread()
        .thread_name_fn(|| {
            static TID: AtomicUsize = AtomicUsize::new(1);
            format!("worker-{}", TID.fetch_add(1, Ordering::Relaxed))
        })
        .build()
        .expect("failed to create tokio runtime");
    let rt_scope = rt.enter();

    tracing_subscriber::registry()
        .with(
            Targets::new()
                .with_default(Level::INFO)
                .with_target("wgpu_hal", Level::WARN)
                .with_target("fvi", Level::TRACE),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_thread_ids(false)
                .with_thread_names(true)
                .with_ansi(true)
                .pretty()
                .fmt_fields(Omit)
                .with_timer(Uptime::default()),
        )
        .init();

    App::run(RenderConfig::default());

    _ = rt_scope;
}
