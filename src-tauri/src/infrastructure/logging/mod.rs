use tracing_subscriber::{fmt, EnvFilter};

pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,workbench_lib=debug"));

    let _ = fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_ansi(cfg!(debug_assertions))
        .try_init();
}
