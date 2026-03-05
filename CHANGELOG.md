# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.3.0] — 2026-03-05

### Added
- **SBOM generation** (`license-checkr sbom generate`): produce a Software Bill
  of Materials from scanned project dependencies in three formats:
  - CycloneDX JSON v1.5 (`--format cyclonedx-json`, default)
  - CycloneDX XML v1.5 (`--format cyclonedx-xml`)
  - SPDX JSON v2.3 (`--format spdx-json`)
- `--output` / `-o` flag to control the SBOM output file path
- `--pdf` flag to generate an SBOM-specific PDF report alongside the SBOM
  (defaults to `sbom-report.pdf` when used without a value)
- `--recursive` / `-r` flag to scan all sub-projects under a workspace root
  and produce a merged SBOM covering every discovered project
- `--online`, `--config`, `--exclude-lang` flags mirroring the main scan
  command for consistent behaviour across all subcommands
- `src/sbom.rs`: `build_cyclonedx`, `build_spdx`, `cyclonedx_to_json`,
  `cyclonedx_to_xml`, `spdx_to_json` helpers
- `src/report/sbom_pdf.rs`: SBOM-specific PDF renderer with `render`
  (single-project) and `render_workspace` (multi-project) entry points (#60)

---

## [0.2.4] — 2026-03-02

### Changed
- `classify_and_apply_policy` in `src/mcp.rs` now accepts `&mut [Dependency]`
  instead of `&mut Vec<Dependency>`, satisfying the Clippy `ptr_arg` lint and
  following idiomatic Rust API conventions (#57)

---

## [0.2.3] — 2026-03-02

### Fixed
- **MCP server Ctrl+C on Windows**: pressing Ctrl+C no longer exits with
  `STATUS_CONTROL_C_EXIT (0xc000013a)`. The Windows OS default Ctrl+C handler
  fires on a new thread and calls `ExitProcess` synchronously, before tokio's
  async runtime gets any CPU time. A synchronous `SetConsoleCtrlHandler`
  callback is now registered via raw `extern "system"` FFI at the start of
  `mcp::serve()`; it returns `TRUE` for `CTRL_C_EVENT` (suppressing the default
  handler) and calls `std::process::exit(0)` directly. No new dependencies.
  Non-Ctrl+C console events (CTRL_BREAK, CLOSE, etc.) are passed through
  unchanged. Linux and macOS are unaffected (#56)

---

## [0.2.2] — 2026-03-02

### Added
- **Startup update check**: on every CLI run (except `mcp serve` and `--quiet`
  mode), the tool fetches the latest release tag from the GitHub Releases API
  and prints a one-line notice to stderr when a newer version is available;
  fails silently on network errors or timeouts so the scan is never blocked (#54)
- `src/updater.rs`: `check_for_update()` with a 3-second timeout and
  `is_newer()` semver tuple comparator; 5 unit tests added (#54)

---

## [0.2.1] — 2026-03-02

### Changed
- MCP server now logs lifecycle events (start, ready, disconnect) and tool-call
  arguments to stderr so MCP clients can observe server activity without
  corrupting the stdio transport (#53)
- `mcp serve` catches Ctrl+C via `tokio::signal::ctrl_c()` and shuts down
  gracefully with exit code 0 instead of propagating a transport error (#53)

---

## [0.2.0] — 2026-03-02

### Added
- **MCP server** (`license-checkr mcp serve`): exposes the license scanner as
  an [MCP (Model Context Protocol)](https://modelcontextprotocol.io) tool over
  stdio JSON-RPC, letting AI agents such as Claude Desktop and Cursor query it
  directly (#50)
- `scan_licenses` MCP tool: accepts `path`, `online`, `config`, `exclude_lang`,
  and `recursive` — runs the full scan pipeline and returns a JSON summary with
  per-dependency license, risk, and verdict fields (#50)
- `get_package_license` MCP tool: accepts `name`, `version`, and `ecosystem`
  (`rust` / `python` / `java` / `node`) — fetches from the upstream registry
  and returns license, risk classification, and policy verdict (#50)
- `src/scanner.rs`: `scan_project()` and `enrich_online()` extracted from
  `main.rs` as `pub async fn`, shared by both the CLI and the MCP server (#50)
- `mcp serve` subcommand added to the CLI via a new `Commands` / `McpAction`
  enum in `cli.rs`; existing scan flags and behaviour are unchanged (#50)
- Claude Desktop configuration documented in README and GitHub Pages site (#51)

### Changed
- README: new **🤖 MCP Server** feature bullet and `## 🤖 MCP Server (AI Agent Tool)`
  section with tools table, Claude Desktop config JSON, and example prompts (#51)
- GitHub Pages: MCP feature card added to the features grid; new "MCP server"
  section (two-column layout, config code block, example prompt chips) added
  before the CTA (#51)

---

## [0.1.7] — 2026-02-27

### Changed
- README installation section now leads with per-platform download badges
  (Windows x64, Linux x64, Linux ARM64, macOS Apple Silicon) linking to the
  latest GitHub Release, followed by quick extraction instructions (#48)
- GitHub Pages landing page hero and final CTA sections now feature an
  OS-detected primary download button that resolves the visitor's platform at
  runtime and links directly to the correct release binary; falls back to the
  releases page for unrecognised platforms (#49)
- Secondary CTAs ("Get Started", "View on GitHub") demoted to outline style
  now that the download button is the primary hero action (#49)

---

## [0.1.6] — 2026-02-26

### Fixed
- Workspace scan progress output was interleaved: dependency count lines
  appeared grouped under whichever project's `tokio::spawn` task printed its
  "→ scanning" header last, instead of under their own project line (#47)
- All inline `println!`/`eprintln!` calls removed from inside spawned tasks;
  scan summaries are now printed in deterministic discovery order after
  `join_all` completes, grouping each project's ecosystem dep counts directly
  below its own header (#47)

---

## [0.1.5] — 2026-02-26

### Added
- **Workspace scanning** (`-r` / `--recursive`): a single pass now discovers
  all sub-projects under a given root directory and produces a unified report.
  Each sub-project is scanned concurrently via `tokio::spawn` and may carry
  its own `.license-checkr/config.toml` policy (#46)
- `find_workspace_projects()` in `detector.rs`: recursive directory walk that
  stops descending once a manifest file is found and skips noise directories
  (`node_modules`, `target`, `vendor`, `.git`, `__pycache__`, `.venv`, etc.)
  with symlink-cycle protection via path canonicalization (#46)
- `ProjectScan` struct in `models.rs` to carry per-project name, path, and
  resolved dependency list (#46)
- Terminal workspace report: aggregated summary box showing total / pass / warn
  / error counts across all projects, followed by per-project sections with
  error and warn dependency tables (#46)
- PDF workspace report: new workspace cover page with aggregate stat cards and
  a projects-scanned table, followed by per-project Risk Summary and Dependency
  Table pages labelled with the project name (#46)
- JSON workspace output: array of `{ project, path, dependencies }` objects
  compatible with tools such as `jq` (#46)
- 8 unit tests for `find_workspace_projects` covering root project detection,
  sub-project discovery, no-recurse-into-project, skipped dirs (`node_modules`,
  `target`), empty directory, and sort order (#46)

### Changed
- Online enrichment batch size reduced from 75 → 50 to improve stability when
  multiple workspace projects trigger concurrent registry requests (#46)

---

## [0.1.4] — 2026-02-26

### Changed
- README: logo image removed from the header; document now starts directly
  with the title heading (#42)
- CI: `x86_64-apple-darwin` (Intel Mac) build target and its associated
  `brew install pkg-config fontconfig` step dropped from the release
  workflow; only the `aarch64-apple-darwin` (Apple Silicon) target is
  retained (#41)

---

## [0.1.3] — 2026-02-26

### Added
- Offline license resolution for Rust: the local Cargo registry cache
  (`$CARGO_HOME/registry/src/`) is now walked to extract `[package].license`
  from each crate's `Cargo.toml`, populating license data without any network
  access. A new `LicenseSource::Cache` variant distinguishes these results
  from manifest, registry, and unknown sources (#36)

### Changed
- README: project logo (shield icon) displayed at the top of the page (#38)
- README: ecosystem table expanded with unit-tested / offline-validated /
  online-validated status columns per language (#37)
- CI: cross-platform release workflow added; pushing a `v*` tag now builds
  release binaries for Windows x86_64, Linux x86_64, Linux ARM64, macOS
  x86_64, and macOS ARM64, then creates a GitHub Release with the matching
  changelog section attached (#40)

---

## [0.1.2] — 2026-02-25

### Added
- Full SPDX compound expression parser with proper operator precedence:
  `AND` binds tighter than `OR`, parentheses override, `WITH` exception
  clauses recognized and stripped before evaluation (#32)
- 10 new unit tests covering all expression forms: simple, OR/AND, nested
  parentheses, precedence rules, slash separator, WITH exceptions (#32)

### Changed
- Config file moved from `license-checkr.toml` at the project root to
  `.license-checkr/config.toml` in a hidden directory, following the
  convention of `.github/` and `.vscode/` (#17, #18)
- PDF report rewritten with native `printpdf` primitives; new
  "Liquid Glass" design with cover page, risk summary table, and paginated
  dependency table — `plotters` and `image` dependencies removed (#33)

### Fixed
- GitHub license detection: removed non-standard trailing content from
  `LICENSE` that caused GitHub's Licensee to report the license as
  "Unknown" (#16)
- 13 Clippy lints resolved (manual_contains, needless_borrows_for_generic_args,
  ptr_arg) that were failing the CI lint job (#15)
- README PDF section, Rust eco-card file list, and landing-page install
  command corrected to match actual implementation (#34)

---

## [0.1.1] — 2026-02-25

### Added
- `LICENSE` file — MIT license with a non-binding Buy Me a Coffee donation note (#11)
- `.github/FUNDING.yml` — enables GitHub's native Sponsor button pointing to Buy Me a Coffee (#12)
- Buy Me a Coffee badge in README header and `☕ Support` section in README (#12)
- `license = "MIT"` field in `Cargo.toml` for crates.io metadata (#11)

### Fixed
- License classifier now handles slash `/` as an OR-equivalent separator
  (e.g. `MIT/Apache-2.0` is treated as `MIT OR Apache-2.0`, most permissive wins) (#9)
- CI job now installs `libfontconfig1-dev` on the Ubuntu runner before building,
  fixing the `yeslogic-fontconfig-sys` build failure (#10)

### Changed
- GitHub Pages site redesigned with liquid glass aesthetic and light/dark theme toggle

---

## [0.1.0] — 2026-02-25

### Added

#### Core CLI
- `license-checkr` binary with `clap` v4 derive-based argument parsing
- Flags: `--online`, `--config`, `--report`, `--pdf [FILE]`, `--exclude-lang`, `-v/--verbose`, `-q/--quiet`
- Exit code `1` when any dependency produces a `PolicyVerdict::Error`; `0` otherwise

#### Ecosystem support (auto-detected, all opt-out via `--exclude-lang`)
- **Rust** — parses `Cargo.lock`; filters local workspace members
- **Python** — parses `Pipfile.lock` → `requirements.txt` → `pyproject.toml` (priority order, deduplicated)
- **Java** — parses `pom.xml`, `build.gradle` / `build.gradle.kts`, `gradle.lockfile`
- **.NET** — parses `*.csproj` / `*.fsproj` (`<PackageReference>`), `packages.config`, `paket.lock`
- **Node.js** — parses `package-lock.json` (v2/v3), `yarn.lock`, `package.json`; extracts embedded license data

#### Online registry enrichment (`--online`)
- Async batch fetching (75 dependencies per batch) via `futures::join_all`
- Registries: crates.io (Rust), PyPI (Python), Maven Central (Java), npm (Node.js)
- Progress bar in non-quiet mode

#### License classification
- SPDX identifier classifier covering 24 permissive, 16 weak-copyleft, and 10 strong-copyleft licenses
- Normalizer mapping 20+ common non-SPDX strings to canonical SPDX identifiers
- SPDX expression support: `MIT OR Apache-2.0` (most permissive wins), `MIT AND GPL-3.0` (most restrictive wins)
- `WITH` exception stripping (e.g. `GPL-2.0 WITH Classpath-exception-2.0`)
- Proprietary/commercial keyword detection

#### Policy engine
- TOML config at `./.license-checkr/config.toml`, `~/.config/license-checkr/config.toml`, or `--config <path>`
- Per-SPDX-identifier rules: `pass`, `warn`, `error`
- Catch-all `default` verdict for unlisted licenses
- Built-in defaults: permissive → pass, LGPL-2.1 → warn, GPL/AGPL → error

#### Report formats
- **Terminal** — colored summary box + per-verdict tables using `comfy-table`
- **JSON** — pretty-printed full dependency array via `serde_json`
- **PDF** — multi-page report with cover page, risk + ecosystem bar charts (via `plotters`), and paginated dependency table (via `printpdf`)

#### Documentation & infrastructure
- `///` doc comments on all public types, fields, and functions
- `//!` module-level documentation for `registry`, `license`, `report`, `main`
- `README.md` with badges, feature grid, ecosystem table, policy config reference, and contributing guide
- `docs/index.html` — GitHub Pages landing site (dark theme, responsive, scroll animations, terminal demo mockup, no external JS)
- `.github/workflows/pages.yml` — deploys `docs/` to GitHub Pages on `v*` tag push; injects release version via `sed`
- `.github/workflows/ci.yml` — runs `cargo test` + `cargo clippy` on push/PR to `main`
- 19 unit tests covering all parsers, SPDX classifier, normalizer, and Maven POM extraction

[0.2.4]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.2.4
[0.2.3]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.2.3
[0.2.2]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.2.2
[0.2.1]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.2.1
[0.2.0]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.2.0
[0.1.6]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.1.6
[0.1.5]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.1.5
[0.1.4]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.1.4
[0.1.3]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.1.3
[0.1.2]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.1.2
[0.1.1]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.1.1
[0.1.0]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.1.0
