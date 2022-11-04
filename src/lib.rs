pub mod config;
pub mod emoji;
pub mod error;
pub mod host_triple;
pub mod targets;
pub mod toolchain;
pub mod logging {
    use env_logger::{Builder, Env, WriteStyle};

    /// Initializes the logger
    pub fn initialize_logger(log_level: &str) {
        Builder::from_env(Env::default().default_filter_or(log_level))
            .format_target(false)
            .format_timestamp_secs()
            .write_style(WriteStyle::Always)
            .init();
    }
}
