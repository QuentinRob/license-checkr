use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::models::Ecosystem;

#[derive(Parser, Debug)]
#[command(
    name = "license-checkr",
    about = "Scan project dependencies and check license compliance",
    version
)]
pub struct Cli {
    /// Subcommand (e.g. `mcp serve`, `sbom generate`)
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Project path to scan
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Fetch license data from package registries
    #[arg(long)]
    pub online: bool,

    /// Recursively scan subdirectories for sub-projects (workspace mode)
    #[arg(short = 'r', long)]
    pub recursive: bool,

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

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// MCP (Model Context Protocol) server commands
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },
    /// SBOM (Software Bill of Materials) commands
    Sbom {
        #[command(subcommand)]
        action: SbomAction,
    },
}

#[derive(Subcommand, Debug)]
pub enum McpAction {
    /// Start an MCP server over stdio for agent tool use
    Serve,
}

#[derive(Subcommand, Debug)]
pub enum SbomAction {
    /// Generate an SBOM from project dependencies
    Generate {
        /// Project path to scan
        #[arg(default_value = ".")]
        path: PathBuf,

        /// SBOM output format
        #[arg(long, default_value = "cyclonedx-json", value_name = "FORMAT")]
        format: SbomFormat,

        /// Output file path [default: sbom.json or sbom.xml]
        #[arg(long, short, value_name = "FILE")]
        output: Option<PathBuf>,

        /// Also generate a PDF report; use without value to default to sbom-report.pdf
        #[arg(long, value_name = "FILE", num_args = 0..=1, default_missing_value = "sbom-report.pdf")]
        pdf: Option<PathBuf>,

        /// Fetch license data from package registries
        #[arg(long)]
        online: bool,

        /// Recursively scan subdirectories for sub-projects (workspace mode)
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Policy config file
        #[arg(long)]
        config: Option<PathBuf>,

        /// Exclude an ecosystem from scanning (repeatable)
        #[arg(long = "exclude-lang", value_name = "LANG")]
        exclude_lang: Vec<EcosystemArg>,
    },
}

/// SBOM output format.
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum SbomFormat {
    /// CycloneDX JSON (v1.5)
    #[value(name = "cyclonedx-json")]
    CycloneDxJson,
    /// CycloneDX XML (v1.5)
    #[value(name = "cyclonedx-xml")]
    CycloneDxXml,
    /// SPDX JSON (v2.3)
    #[value(name = "spdx-json")]
    SpdxJson,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum ReportFormat {
    Terminal,
    Json,
    Pdf,
    /// GitLab Code Quality JSON artifact
    #[value(name = "gitlab")]
    Gitlab,
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
