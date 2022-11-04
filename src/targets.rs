//! ESP32 chip variants support.

use crate::emoji;
use anyhow::Context;
use log::debug;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, str::FromStr};
use strum::{Display, IntoEnumIterator};
use strum_macros::{EnumIter, EnumString};

#[derive(
    Clone, Copy, EnumIter, EnumString, PartialEq, Hash, Eq, Debug, Display, Deserialize, Serialize,
)]
#[strum(serialize_all = "lowercase")]
pub enum Target {
    /// Xtensa LX6 based dual core
    ESP32 = 0,
    /// Xtensa LX7 based single core
    ESP32S2,
    /// Xtensa LX7 based dual core
    ESP32S3,
    /// RISC-V based single core
    ESP32C3,
}

/// Returns a vector of Chips from a comma or space separated string.
pub fn parse_targets(targets_str: &str) -> Result<HashSet<Target>, String> {
    debug!("{} Parsing targets: {}", emoji::DEBUG, targets_str);

    let targets_str = targets_str.to_lowercase();
    let targets_str = targets_str.trim();

    let targets: HashSet<Target> = if targets_str.contains("all") {
        Target::iter().collect()
    } else {
        targets_str
            .split([',', ' '])
            .map(|target| {
                Target::from_str(target)
                    .context(format!(
                        "{} Target '{}' is not supported",
                        emoji::ERROR,
                        target
                    ))
                    .unwrap()
            })
            .collect()
    };

    debug!("{} Parsed targets: {:?}", emoji::DEBUG, targets);
    Ok(targets)
}

#[cfg(test)]
mod tests {
    use crate::targets::{parse_targets, Target};

    #[test]
    fn test_parse_targets() {
        assert_eq!(
            parse_targets("esp32"),
            Ok([Target::ESP32].into_iter().collect())
        );
        assert_eq!(
            parse_targets("esp32,esp32s2"),
            Ok([Target::ESP32, Target::ESP32S2].into_iter().collect())
        );
        assert_eq!(
            parse_targets("esp32s3 esp32"),
            Ok([Target::ESP32S3, Target::ESP32].into_iter().collect())
        );
        assert_eq!(
            parse_targets("esp32s3,esp32,esp32c3"),
            Ok([Target::ESP32S3, Target::ESP32, Target::ESP32C3]
                .into_iter()
                .collect())
        );
        assert_eq!(
            parse_targets("all"),
            Ok([
                Target::ESP32,
                Target::ESP32S2,
                Target::ESP32S3,
                Target::ESP32C2,
                Target::ESP32C3,
            ]
            .into_iter()
            .collect())
        );
    }
}
