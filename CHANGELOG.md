# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.1.5]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.1.5
[0.1.4]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.1.4
[0.1.3]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.1.3
[0.1.2]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.1.2
[0.1.1]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.1.1
[0.1.0]: https://github.com/QuentinRob/license-checkr/releases/tag/v0.1.0
