use anyhow::Result;
use rmcp::{
    Error as McpError,
    ServiceExt,
    ServerHandler,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars,
    tool,
};
use serde::{Deserialize, Serialize};

use crate::config::{apply_policy, load_config};
use crate::license::classifier::classify;
use crate::models::{Ecosystem, LicenseSource, PolicyVerdict};
use crate::registry;
use crate::scanner;

// ── Tool input structs ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ScanLicensesInput {
    /// Absolute or relative path to the project root to scan
    pub path: String,
    /// Fetch license data from upstream registries
    pub online: Option<bool>,
    /// Path to a custom policy config file
    pub config: Option<String>,
    /// Ecosystems to exclude: rust, python, java, node
    pub exclude_lang: Option<Vec<String>>,
    /// Recursively scan sub-projects (workspace mode)
    pub recursive: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetPackageLicenseInput {
    /// Package name (e.g. "serde", "requests", "com.google.guava:guava")
    pub name: String,
    /// Package version string
    pub version: String,
    /// Ecosystem: rust | python | java | node
    pub ecosystem: String,
}

// ── Tool output structs ───────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ScanResult {
    path: String,
    total: usize,
    pass: usize,
    warn: usize,
    error: usize,
    ecosystems: Vec<String>,
    dependencies: Vec<crate::models::Dependency>,
}

#[derive(Debug, Serialize)]
struct PackageLicenseResult {
    name: String,
    version: String,
    ecosystem: String,
    license: Option<String>,
    risk: String,
    verdict: String,
    source: String,
}

// ── MCP server implementation ─────────────────────────────────────────────────

#[derive(Clone)]
pub struct LicenseCheckerMcp;

impl LicenseCheckerMcp {
    pub fn new() -> Self {
        Self
    }
}

#[tool(tool_box)]
impl LicenseCheckerMcp {
    /// Scan a project directory for dependency licenses and evaluate against policy
    #[tool(description = "Scan a project directory for dependency licenses and evaluate them against the configured policy. Returns a JSON summary with per-dependency license, risk, and verdict.")]
    async fn scan_licenses(
        &self,
        #[tool(aggr)] input: ScanLicensesInput,
    ) -> Result<CallToolResult, McpError> {
        eprintln!(
            "[mcp] scan_licenses: path={:?} online={} recursive={}",
            input.path,
            input.online.unwrap_or(false),
            input.recursive.unwrap_or(false),
        );
        let path = std::path::PathBuf::from(&input.path);
        let path = path.canonicalize().unwrap_or(path);

        let config_path = input.config.as_deref().map(std::path::Path::new);
        let config = load_config(&path, config_path)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let excluded: Vec<Ecosystem> = input
            .exclude_lang
            .unwrap_or_default()
            .iter()
            .filter_map(|s| parse_ecosystem(s))
            .collect();

        let online = input.online.unwrap_or(false);
        let recursive = input.recursive.unwrap_or(false);

        let all_deps = if recursive {
            let project_paths = crate::detector::find_workspace_projects(&path);
            let mut deps = Vec::new();
            for proj_path in project_paths {
                let proj_config = load_config(&proj_path, None)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                let mut proj_deps =
                    scanner::scan_project(&proj_path, &proj_config, &excluded, online, true)
                        .await
                        .map_err(|e| McpError::internal_error(e.to_string(), None))?;
                classify_and_apply_policy(&mut proj_deps, &proj_config);
                deps.extend(proj_deps);
            }
            deps
        } else {
            let mut deps = scanner::scan_project(&path, &config, &excluded, online, true)
                .await
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            classify_and_apply_policy(&mut deps, &config);
            deps
        };

        let mut ecosystems: Vec<String> = all_deps
            .iter()
            .map(|d| d.ecosystem.to_string())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        ecosystems.sort();

        let pass = all_deps.iter().filter(|d| d.verdict == PolicyVerdict::Pass).count();
        let warn = all_deps.iter().filter(|d| d.verdict == PolicyVerdict::Warn).count();
        let error = all_deps.iter().filter(|d| d.verdict == PolicyVerdict::Error).count();

        let result = ScanResult {
            path: path.display().to_string(),
            total: all_deps.len(),
            pass,
            warn,
            error,
            ecosystems,
            dependencies: all_deps,
        };

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Look up the license for a single package from its upstream registry
    #[tool(description = "Look up the license for a single package by name, version, and ecosystem (rust/python/java/node). Fetches from the upstream registry and returns the license with risk and policy verdict.")]
    async fn get_package_license(
        &self,
        #[tool(aggr)] input: GetPackageLicenseInput,
    ) -> Result<CallToolResult, McpError> {
        eprintln!(
            "[mcp] get_package_license: name={} version={} ecosystem={}",
            input.name, input.version, input.ecosystem,
        );
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let license_opt = match input.ecosystem.to_lowercase().as_str() {
            "rust" => registry::crates_io::fetch_license(&client, &input.name, &input.version).await,
            "python" => registry::pypi::fetch_license(&client, &input.name, &input.version).await,
            "java" => registry::maven::fetch_license(&client, &input.name, &input.version).await,
            "node" => registry::npm::fetch_license(&client, &input.name, &input.version).await,
            other => {
                return Err(McpError::invalid_params(
                    format!("Unknown ecosystem '{}'. Use: rust, python, java, node", other),
                    None,
                ));
            }
        }
        .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let config = crate::config::Config::default();
        let license_str = license_opt.as_deref();
        let risk = classify(license_str.unwrap_or("unknown"));
        let verdict = apply_policy(&config, license_str);
        let source = if license_opt.is_some() {
            LicenseSource::Registry
        } else {
            LicenseSource::Unknown
        };

        let result = PackageLicenseResult {
            name: input.name,
            version: input.version,
            ecosystem: input.ecosystem,
            license: license_opt,
            risk: risk.to_string(),
            verdict: verdict.to_string(),
            source: source.to_string(),
        };

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

#[tool(tool_box)]
impl ServerHandler for LicenseCheckerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Check software license compliance for projects and packages. \
                Use scan_licenses to audit a full project, or get_package_license \
                to look up a single dependency."
                    .into(),
            ),
            ..Default::default()
        }
    }
}

// ── Windows Ctrl+C handler ────────────────────────────────────────────────────
//
// On Windows, pressing Ctrl+C dispatches CTRL_C_EVENT on a new OS thread.
// If no registered handler returns TRUE the default handler calls
// ExitProcess(STATUS_CONTROL_C_EXIT) synchronously — before tokio's async
// runtime ever gets CPU time.  Register a synchronous handler via raw FFI so
// we can suppress the default exit code and terminate cleanly with 0.
#[cfg(windows)]
#[link(name = "Kernel32")]
extern "system" {
    fn SetConsoleCtrlHandler(
        handler_routine: Option<unsafe extern "system" fn(dw_ctrl_type: u32) -> i32>,
        add: i32,
    ) -> i32;
}

#[cfg(windows)]
fn install_ctrl_c_handler() {
    unsafe extern "system" fn handler(dw_ctrl_type: u32) -> i32 {
        if dw_ctrl_type == 0 {
            // CTRL_C_EVENT — exit cleanly before the OS default handler fires
            eprintln!("\n[mcp] Received Ctrl+C, shutting down gracefully");
            std::process::exit(0);
        }
        0 // FALSE — pass CTRL_BREAK / CLOSE / etc. to the next handler
    }
    // SAFETY: `handler` has the correct extern "system" signature for
    // SetConsoleCtrlHandler and does not reference any Rust state.
    unsafe { SetConsoleCtrlHandler(Some(handler), 1); }
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Start an MCP server over stdio and block until the client disconnects or Ctrl+C is received.
pub async fn serve() -> Result<()> {
    // On Windows install a synchronous Ctrl+C handler before starting the
    // server so the process exits with code 0 instead of STATUS_CONTROL_C_EXIT.
    #[cfg(windows)]
    install_ctrl_c_handler();

    eprintln!("[mcp] Starting license-checkr MCP server (stdio transport)");

    let service = LicenseCheckerMcp::new();
    let server = service
        .serve(rmcp::transport::io::stdio())
        .await
        .map_err(|e| anyhow::anyhow!("MCP transport error: {}", e))?;

    eprintln!("[mcp] Server ready — waiting for requests");

    tokio::select! {
        result = server.waiting() => {
            match result {
                Ok(_) => eprintln!("[mcp] Client disconnected, shutting down"),
                Err(e) => eprintln!("[mcp] Server error: {e}"),
            }
        }
        _ = tokio::signal::ctrl_c() => {
            eprintln!("\n[mcp] Received Ctrl+C, shutting down gracefully");
        }
    }

    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_ecosystem(s: &str) -> Option<Ecosystem> {
    match s.to_lowercase().as_str() {
        "rust" => Some(Ecosystem::Rust),
        "python" => Some(Ecosystem::Python),
        "java" => Some(Ecosystem::Java),
        "node" => Some(Ecosystem::Node),
        "dotnet" => Some(Ecosystem::DotNet),
        _ => None,
    }
}

fn classify_and_apply_policy(deps: &mut Vec<crate::models::Dependency>, config: &crate::config::Config) {
    for dep in deps.iter_mut() {
        let license = dep
            .license_spdx
            .as_deref()
            .or(dep.license_raw.as_deref())
            .unwrap_or("unknown");
        dep.risk = classify(license);
        dep.verdict = apply_policy(config, Some(license));
    }
}
