use anyhow::Result;
use directories::ProjectDirs;
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, Registry};

pub fn setup_logging(
    log_level: tracing::Level,
) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let proj_dirs = ProjectDirs::from("", "", "trui").expect("Opening cache directory");
    let cache_dir = proj_dirs.cache_dir();
    let tracing_file_appender = tracing_appender::rolling::never(cache_dir, "trui.log");
    let (tracing_file_writer, guard) = tracing_appender::non_blocking(tracing_file_appender);

    let subscriber = Registry::default().with(
        tracing_subscriber::fmt::Layer::default()
            .with_writer(tracing_file_writer.with_max_level(log_level)),
    );
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(guard)
}
