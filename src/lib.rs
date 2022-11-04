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

pub mod update {
    use crate::emoji;
    use log::warn;
    use std::time::Duration;
    use update_informer::{registry, Check};

    /// Check crates.io for a new version of the application
    pub fn check_for_update(name: &str, version: &str) {
        // By setting the interval to 0 seconds we invalidate the cache with each
        // invocation and ensure we're getting up-to-date results
        let informer =
            update_informer::new(registry::Crates, name, version).interval(Duration::ZERO);

        if let Some(version) = informer.check_version().ok().flatten() {
            warn!(
                "{} A new version of {name} ('{version}') is available.",
                emoji::WARN
            );
        }
    }
}
