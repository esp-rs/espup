use clap::ValueEnum;
use clap_complete::Shell;
use clap_complete_nushell::Nushell;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum CompletionShell {
    Clap(Shell),
    Nushell,
}

impl CompletionShell {
    pub fn generate(&self, cmd: &mut clap::Command, bin_name: &str, out: &mut dyn std::io::Write) {
        match self {
            CompletionShell::Clap(sh) => {
                clap_complete::generate(*sh, cmd, bin_name, out);
            }
            CompletionShell::Nushell => {
                clap_complete::generate(Nushell, cmd, bin_name, out);
            }
        }
    }
}

impl ValueEnum for CompletionShell {
    fn value_variants<'a>() -> &'a [Self] {
        use Shell::*;
        // Static list of variants, one for each possible shell
        const VARIANTS: &[CompletionShell] = &[
            CompletionShell::Clap(Bash),
            CompletionShell::Clap(Zsh),
            CompletionShell::Clap(Fish),
            CompletionShell::Clap(PowerShell),
            CompletionShell::Clap(Elvish),
            CompletionShell::Nushell,
        ];
        VARIANTS
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(match self {
            CompletionShell::Clap(sh) => sh.to_possible_value()?,
            CompletionShell::Nushell => clap::builder::PossibleValue::new("nushell"),
        })
    }
}

impl fmt::Display for CompletionShell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompletionShell::Clap(shell) => write!(f, "{shell}"),
            CompletionShell::Nushell => write!(f, "nushell"),
        }
    }
}

impl FromStr for CompletionShell {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_ascii_lowercase().as_str() {
            "bash" | "zsh" | "fish" | "powershell" | "elvish" => input
                .parse::<Shell>()
                .map(CompletionShell::Clap)
                .map_err(|e| e.to_string()),

            "nushell" => Ok(CompletionShell::Nushell),
            _ => Err(format!("unsupported shell: {input}")),
        }
    }
}
