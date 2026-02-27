# 🔍 license-checkr

[![Build](https://github.com/QuentinRob/license-checkr/actions/workflows/ci.yml/badge.svg)](https://github.com/QuentinRob/license-checkr/actions)
[![Release](https://img.shields.io/github/v/release/QuentinRob/license-checkr?color=brightgreen)](https://github.com/QuentinRob/license-checkr/releases)
[![License](https://img.shields.io/github/license/QuentinRob/license-checkr)](LICENSE)
[![Rust 2021](https://img.shields.io/badge/rust-2021_edition-orange?logo=rust)](https://www.rust-lang.org)
[![Stars](https://img.shields.io/github/stars/QuentinRob/license-checkr?style=social)](https://github.com/QuentinRob/license-checkr/stargazers)
[![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-support-yellow?logo=buy-me-a-coffee&logoColor=white)](https://buymeacoffee.com/quentinrob)

> **Scan your dependencies. Know your risks. Ship with confidence.**

`license-checkr` is a blazing-fast CLI tool written in Rust that scans your project's dependency manifests, resolves license information, evaluates it against a policy you define, and outputs a clear report — in your terminal, as JSON, or as a PDF.

---

## ✨ Features

- 🌍 **Multi-ecosystem** — Rust, Python, Java, Node.js, and .NET in a single run
- 🔎 **Auto-detection** — no configuration required; detects your stack automatically
- 📡 **Online enrichment** — fetch missing license data from crates.io, PyPI, Maven Central, and npm
- ⚖️ **Policy engine** — define per-license rules (`pass` / `warn` / `error`) in a simple TOML file
- 🏷️ **SPDX-aware** — normalizes 20+ non-standard license strings to SPDX identifiers
- 🧮 **Expression support** — parses full SPDX compound expressions including `(Apache-2.0 OR MIT) AND BSD-3-Clause` with proper operator precedence (`AND` binds tighter than `OR`, parentheses override)
- 📊 **Multiple outputs** — colored terminal table, machine-readable JSON, or a shareable PDF report
- 🚦 **CI-friendly** — exits with code `1` when a policy error is found; `0` otherwise
- 🗂️ **Workspace scanning** — use `--recursive` to scan all sub-projects in a monorepo in a single run

---

## 🚀 Installation

### Pre-built binaries

Download the latest release for your platform:

<p>
  <a href="https://github.com/QuentinRob/license-checkr/releases/latest">
    <img src="https://img.shields.io/badge/Windows-x86__64-0078D6?style=for-the-badge&logo=windows&logoColor=white" alt="Windows x64" />
  </a>
  <a href="https://github.com/QuentinRob/license-checkr/releases/latest">
    <img src="https://img.shields.io/badge/Linux-x86__64-FCC624?style=for-the-badge&logo=linux&logoColor=black" alt="Linux x64" />
  </a>
  <a href="https://github.com/QuentinRob/license-checkr/releases/latest">
    <img src="https://img.shields.io/badge/Linux-ARM64-FCC624?style=for-the-badge&logo=linux&logoColor=black" alt="Linux ARM64" />
  </a>
  <a href="https://github.com/QuentinRob/license-checkr/releases/latest">
    <img src="https://img.shields.io/badge/macOS-Apple_Silicon-000000?style=for-the-badge&logo=apple&logoColor=white" alt="macOS ARM64" />
  </a>
</p>

Extract and place the binary somewhere on your `PATH`:

```bash
# Linux / macOS
tar -xzf license-checkr-*.tar.gz
sudo mv license-checkr /usr/local/bin/

# Windows — extract the .zip and move license-checkr.exe to a folder in your PATH
```

### From source (requires Rust 1.75+)

```bash
cargo install --git https://github.com/QuentinRob/license-checkr
```

### Build locally

```bash
git clone https://github.com/QuentinRob/license-checkr
cd license-checkr
cargo build --release
# binary at ./target/release/license-checkr
```

---

## 📖 Usage

```
license-checkr [OPTIONS] [PATH]
```

| Argument | Description |
|---|---|
| `[PATH]` | Project root to scan (default: current directory) |
| `--online` | Fetch license data from package registries |
| `--config <FILE>` | Override policy config file path |
| `--report <FORMAT>` | Output format: `terminal` (default), `json`, `pdf` |
| `--pdf [FILE]` | Write PDF report (default: `license-report.pdf`) |
| `--exclude-lang <LANG>` | Skip an ecosystem: `rust` `python` `java` `node` `dotnet` (repeatable) |
| `-r, --recursive` | Recursively scan sub-projects (workspace mode) |
| `-v, --verbose` | Show all dependencies, not just warnings and errors |
| `-q, --quiet` | Print summary line only |

### Examples

```bash
# Scan the current directory
license-checkr

# Scan a specific project with online registry lookup
license-checkr ~/my-project --online

# Export a PDF report
license-checkr --pdf report.pdf

# Output machine-readable JSON for CI pipelines
license-checkr --report json | jq '.[] | select(.verdict == "error")'

# Scan only Rust and Node, skip Python and Java
license-checkr --exclude-lang python --exclude-lang java

# Quiet mode — perfect for CI scripts
license-checkr -q && echo "✅ All licenses OK"
```

### Workspace scanning

When your repository contains multiple sub-projects (a monorepo), use `--recursive` to discover and scan every sub-project in a single pass:

```bash
# Scan all sub-projects under the current directory
license-checkr --recursive

# Workspace scan with online enrichment and a unified PDF report
license-checkr --recursive --online --pdf workspace-report.pdf

# JSON output: array of { project, path, dependencies }
license-checkr --recursive --report json | jq '.[].project'

# Quiet workspace summary — great for CI
license-checkr --recursive -q && echo "✅ All workspace licenses OK"
```

Each sub-project is scanned independently with its own policy config (if present). The PDF report includes a workspace cover page with an aggregated summary, followed by per-project Risk Summary and Dependency Table sections.

---

## 🌍 Supported Ecosystems

| Ecosystem | Manifest files parsed | Unit tested | Offline validated | Online validated |
|---|---|:---:|:---:|:---:|
| 🦀 **Rust** | `Cargo.lock` | ✅ | ✅ | ✅ crates.io |
| 🐍 **Python** | `Pipfile.lock`, `requirements.txt`, `pyproject.toml` | ✅ | ⚠️ not validated | ⚠️ not validated |
| ☕ **Java** | `pom.xml`, `build.gradle`, `build.gradle.kts`, `gradle.lockfile` | ✅ | ⚠️ not validated | ⚠️ not validated |
| 🟢 **Node.js** | `package-lock.json`, `yarn.lock`, `package.json` | ✅ | ⚠️ not validated | ⚠️ not validated |
| 🔷 **.NET** | `*.csproj`, `*.fsproj`, `packages.config`, `paket.lock` | ✅ | ⚠️ not validated | ❌ no NuGet client yet |

Multiple ecosystems are detected automatically in a single pass. Use `--exclude-lang` to opt out of any you don't need.

---

## ⚙️ Policy Configuration

Create a `.license-checkr/config.toml` file in your project root (or at `~/.config/license-checkr/config.toml` for a global policy). If no config is found, a sensible default policy is applied.

```toml
[policy]
# Default verdict for any license not listed below
default = "warn"   # pass | warn | error

[policy.licenses]
# Permissive — always allowed
"MIT"          = "pass"
"Apache-2.0"   = "pass"
"BSD-2-Clause" = "pass"
"BSD-3-Clause" = "pass"
"ISC"          = "pass"
"0BSD"         = "pass"
"Unlicense"    = "pass"
"CC0-1.0"      = "pass"

# Weak copyleft — review required
"LGPL-2.1"    = "warn"
"MPL-2.0"     = "warn"
"LGPL-3.0"    = "warn"

# Strong copyleft — blocked
"GPL-2.0"     = "error"
"GPL-3.0"     = "error"
"AGPL-3.0"    = "error"

# Unknown licenses — warn but don't block
"unknown"      = "warn"
```

### Config lookup order

1. `--config <FILE>` argument
2. `./.license-checkr/config.toml` (project-level)
3. `~/.config/license-checkr/config.toml` (global)
4. Built-in default policy

---

## 📊 Output Examples

### Terminal (default)

```
  → Rust   42 dependencies
  → Node   87 dependencies

 ┌──────────────────────────────────────────────────────┐
 │  SUMMARY                                             │
 │  Scanned path   :  /home/user/my-project             │
 │  Total          :  129                               │
 │  ✓  Pass        :   114  MIT (68), Apache-2.0 (32)  │
 │  ⚠  Warn        :    12  unknown (12)               │
 │  ✗  Error       :     3  GPL-3.0 (3)                │
 └──────────────────────────────────────────────────────┘

 Errors
 ┌───────────────────┬─────────┬───────────┬─────────┬───────────────┬────────┐
 │ Name              │ Version │ Ecosystem │ License │ Risk          │Verdict │
 ╞═══════════════════╪═════════╪═══════════╪═════════╪═══════════════╪════════╡
 │ some-gpl-package  │ 2.1.0   │ Node      │ GPL-3.0 │ StrongCopyleft│ error  │
 └───────────────────┴─────────┴───────────┴─────────┴───────────────┴────────┘
```

### JSON

```bash
license-checkr --report json
```

```json
[
  {
    "name": "serde",
    "version": "1.0.136",
    "ecosystem": "Rust",
    "license_raw": "MIT OR Apache-2.0",
    "license_spdx": "MIT OR Apache-2.0",
    "risk": "Permissive",
    "verdict": "pass",
    "source": "registry"
  }
]
```

### PDF

```bash
license-checkr --pdf report.pdf
```

Generates a multi-page PDF with:
- Cover page with scan summary and verdict statistics
- Risk summary table with per-verdict counts and ecosystem breakdown
- Full dependency table (paginated)

---

## 🔬 License Risk Levels

| Risk | Description | Examples |
|---|---|---|
| ✅ **Permissive** | Minimal restrictions; use freely | MIT, Apache-2.0, BSD, ISC, Unlicense |
| ⚠️ **Weak Copyleft** | Share-alike applies only to the library | LGPL, MPL-2.0, EPL |
| 🔴 **Strong Copyleft** | May require your project to be open-sourced | GPL-2.0, GPL-3.0, AGPL-3.0 |
| 🔒 **Proprietary** | Commercial; requires explicit agreement | `commercial`, `proprietary` |
| ❓ **Unknown** | Could not be determined | missing or unrecognized license |

---

## 🤝 Contributing

Contributions are welcome! Here's how to get started:

1. **Fork** the repository
2. **Clone** your fork: `git clone https://github.com/YOUR_USERNAME/license-checkr`
3. **Create a branch**: `git checkout -b feat/my-improvement`
4. **Make your changes** and add tests
5. **Run the test suite**: `cargo test`
6. **Open a pull request** — describe what you changed and why

### Ideas for contribution

- 🆕 New ecosystem analyzer (Go modules, Ruby gems, PHP Composer, Swift SPM…)
- 📡 NuGet registry client for `--online` .NET support
- 🌐 Additional SPDX identifiers in the classifier
- 🧪 More unit tests and edge-case coverage

Please open an issue before starting work on a large change so we can discuss the approach.

---

## 📄 License

This project is licensed under the **MIT License** — see the [LICENSE](LICENSE) file for details.

---

## ☕ Support

If `license-checkr` saved you time, a coffee is always appreciated — but never required!

<a href="https://buymeacoffee.com/quentinrob">
  <img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" height="50" />
</a>

---

<p align="center">
  Made with ❤️ and 🦀 Rust
</p>
