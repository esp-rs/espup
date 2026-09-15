//! ESP32 chip variants support.

use crate::error::Error;
use log::debug;
use miette::Result;
use std::{collections::HashSet, str::FromStr};
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

#[derive(Clone, Copy, EnumIter, EnumString, PartialEq, Hash, Eq, Debug, Display)]
#[strum(serialize_all = "lowercase")]
pub enum Target {
    /// Xtensa LX6 based dual core
    ESP32 = 0,
    /// RISC-V based single core
    ESP32C2,
    /// RISC-V based single core
    ESP32C3,
    /// RISC-V based single core
    ESP32C5,
    /// RISC-V based single core
    ESP32C6,
    /// RISC-V based single core
    ESP32C61,
    /// RISC-V based single core
    ESP32H2,
    /// Xtensa LX7 based single core
    ESP32S2,
    /// Xtensa LX7 based dual core
    ESP32S3,
    /// RISC-V based dual core
    ESP32P4,
}

impl Target {
    /// Returns true if the target is a RISC-V based chip.
    pub fn is_riscv(&self) -> bool {
        !self.is_xtensa()
    }

    /// Returns true if the target is a Xtensa based chip.
    pub fn is_xtensa(&self) -> bool {
        matches!(self, Target::ESP32 | Target::ESP32S2 | Target::ESP32S3)
    }
}

/// Returns a vector of Chips from a comma or space separated string.
pub fn parse_targets(targets_str: &str) -> Result<HashSet<Target>, Error> {
    debug!("Parsing targets: {targets_str}");

    let targets_str = targets_str.to_lowercase();
    let targets_str = targets_str.trim();

    let targets: HashSet<Target> = if targets_str.contains("all") {
        Target::iter().collect()
    } else {
        let mut targets = HashSet::new();
        for target in targets_str.split([',', ' ']) {
            targets.insert(
                Target::from_str(target).map_err(|_| Error::UnsupportedTarget(target.into()))?,
            );
        }

        targets
    };

    debug!("Parsed targets: {targets:?}");
    Ok(targets)
}

#[cfg(test)]
mod tests {
    use crate::{
        error::Error,
        targets::{Target, parse_targets},
    };
    use std::collections::HashSet;
    use strum::IntoEnumIterator;

    #[test]
    fn test_parse_targets() {
        for (input, expected) in [
            ("esp32", vec![Target::ESP32]),
            ("esp32,esp32s2", vec![Target::ESP32, Target::ESP32S2]),
            ("esp32s3 esp32", vec![Target::ESP32S3, Target::ESP32]),
            (
                "esp32s3,esp32,esp32c3",
                vec![Target::ESP32S3, Target::ESP32, Target::ESP32C3],
            ),
        ] {
            assert_eq!(
                parse_targets(input).unwrap(),
                expected.into_iter().collect::<HashSet<_>>()
            );
        }

        let all_targets: HashSet<Target> = Target::iter().collect();
        assert_eq!(parse_targets("all").unwrap(), all_targets);
        assert_eq!(parse_targets("ALL").unwrap(), all_targets);
        assert_eq!(
            parse_targets("esp32,esp32").unwrap(),
            [Target::ESP32].into_iter().collect()
        );

        for input in ["", "esp32,", "esp32  esp32s2", "unknown"] {
            assert!(
                matches!(parse_targets(input), Err(Error::UnsupportedTarget(_))),
                "{input:?}"
            );
        }
    }
}
