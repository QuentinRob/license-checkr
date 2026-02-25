use std::path::PathBuf;

use clap::Parser;

use crate::models::Ecosystem;

#[derive(Parser, Debug)]
#[command(
    name = "license-checkr",
    about = "Scan project dependencies and check license compliance",
    version
)]
pub struct Cli {
    /// Project path to scan
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Fetch license data from package registries
    #[arg(long)]
    pub online: bool,

    /// Policy config file [default: ./.license-checkr/config.toml, fallback ~/.config/license-checkr/config.toml]
    #[arg(long)]
    pub config: Option<PathBuf>,

    /// Report format
    #[arg(long, default_value = "terminal", value_name = "FORMAT")]
    pub report: ReportFormat,

    /// PDF output path; use without value to default to license-report.pdf
    #[arg(long, value_name = "FILE", num_args = 0..=1, default_missing_value = "license-report.pdf")]
    pub pdf: Option<PathBuf>,

    /// Exclude an ecosystem from scanning (repeatable)
    #[arg(long = "exclude-lang", value_name = "LANG")]
    pub exclude_lang: Vec<EcosystemArg>,

    /// Show all dependencies (not just warnings/errors)
    #[arg(short, long)]
    pub verbose: bool,

    /// Only print summary line
    #[arg(short, long)]
    pub quiet: bool,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum ReportFormat {
    Terminal,
    Json,
    Pdf,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum EcosystemArg {
    Rust,
    Python,
    Java,
    Node,
    Dotnet,
}

impl From<&EcosystemArg> for Ecosystem {
    fn from(arg: &EcosystemArg) -> Self {
        match arg {
            EcosystemArg::Rust => Ecosystem::Rust,
            EcosystemArg::Python => Ecosystem::Python,
            EcosystemArg::Java => Ecosystem::Java,
            EcosystemArg::Node => Ecosystem::Node,
            EcosystemArg::Dotnet => Ecosystem::DotNet,
        }
    }
}
