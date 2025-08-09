pub mod cli;
mod completion_shell;
pub mod env;
pub mod error;
pub mod host_triple;
pub mod targets;
pub mod toolchain;

pub mod logging {
    use env_logger::{Builder, Env, WriteStyle};

    use crate::toolchain::PROCESS_BARS;

    /// Initializes the logger
    pub fn initialize_logger(log_level: &str) {
        let logger = Builder::from_env(Env::default().default_filter_or(log_level))
            .format(|buf, record| {
                use std::io::Write;
                writeln!(
                    buf,
                    "[{}]: {}",
                    record.level().to_string().to_lowercase(),
                    record.args()
                )
            })
            .write_style(WriteStyle::Always)
            .build();
        let level = logger.filter();
        // make logging and process bar no longer mixed up
        indicatif_log_bridge::LogWrapper::new(PROCESS_BARS.clone(), logger)
            .try_init()
            .unwrap();
        log::set_max_level(level);
    }
}

pub mod update {
    use log::warn;
    use std::time::Duration;
    use update_informer::{Check, registry};

    /// Check crates.io for a new version of the application
    pub fn check_for_update(name: &str, version: &str) {
        // By setting the interval to 0 seconds we invalidate the cache with each
        // invocation and ensure we're getting up-to-date results
        let informer =
            update_informer::new(registry::Crates, name, version).interval(Duration::ZERO);

        if let Some(version) = informer.check_version().ok().flatten() {
            warn!("A new version of {name} ('{version}') is available");
        }
    }
}
