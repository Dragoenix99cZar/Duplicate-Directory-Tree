# Duplicate-Directory-Tree

```text
  ▓▓▓▓▓▓▓▓▓▓▌▐  ▓▓▌▐    ▓▓▌▐ ▓▓▓▓▓▓▓▓▓▓▌▐      ▓▓▓▓▓▓▓▓▓▓▌▐  ▓▓▌▐ ▓▓▓▓▓▓▓▓▓▓▌▐       ▓▓▓▓▓▓▓▓▓▓▌▐ ▓▓▓▓▓▓▓▓▓▓▌▐  ▓▓▓▓▓▓▓▓▓▓▌▐  ▓▓▓▓▓▓▓▓▓▓▌▐ 
  ▓▓▌▐    ▓▓▌▐  ▓▓▌▐    ▓▓▌▐ ▓▓▌▐    ▓▓▌▐      ▓▓▌▐    ▓▓▌▐       ▓▓▌▐    ▓▓▌▐          ▓▓▌▐      ▓▓▌▐    ▓▓▌▐  ▓▓▌▐    ▓▓▌▐  ▓▓▌▐    ▓▓▌▐ 
  ▓▓▌▐    ▓▓▌▐  ▓▓▌▐    ▓▓▌▐ ▓▓▌▐    ▓▓▌▐      ▓▓▌▐    ▓▓▌▐  ▓▓▌▐ ▓▓▌▐    ▓▓▌▐          ▓▓▌▐      ▓▓▌▐    ▓▓▌▐  ▓▓▌▐          ▓▓▌▐         
  ▓▓▌▐    ▐▓▓▌▐ ▓▓▌▐    ▓▓▌▐ ▓▓▓▓▓▓▓▓▓▓▌▐      ▓▓▌▐    ▐▓▓▌▐ ▓▓▌▐ ▓▓▓▓▓▓▓▓▓▓▓▌▐         ▓▓▌▐      ▓▓▓▓▓▓▓▓▓▓▓▌▐ ▓▓▓▓▓▓▓▓▐     ▓▓▓▓▓▓▓▓▐    
  ██▌▐     ██▌▐ ██▌▐    ██▌▐ ██▌▐              ██▌▐     ██▌▐ ██▌▐ ██▌▐     ██▌▐         ██▌▐      ██▌▐     ██▌▐ ██▌▐          ██▌▐         
  ▓▓▌▐     ▓▓▌▐ ▓▓▌▐    ▓▓▌▐ ▓▓▌▐              ▓▓▌▐     ▓▓▌▐ ▓▓▌▐ ▓▓▌▐     ▓▓▌▐         ▓▓▌▐      ▓▓▌▐     ▓▓▌▐ ▓▓▌▐     ▓▓▌▐ ▓▓▌▐     ▓▓▌▐
  ▒▒▌▐     ▒▒▌▐ ▒▒▌▐    ▒▒▌▐ ▒▒▌▐              ▒▒▌▐     ▒▒▌▐ ▒▒▌▐ ▒▒▌▐     ▒▒▌▐         ▒▒▌▐      ▒▒▌▐     ▒▒▌▐ ▒▒▌▐     ▒▒▌▐ ▒▒▌▐     ▒▒▌▐
  ░░░░░░░░░░░▌▐ ░░░░░░░░░░▌▐ ░░▌▐              ░░░░░░░░░░░▌▐ ░░▌▐ ░░▌▐     ░░▌▐         ░░▌▐      ░░▌▐     ░░▌▐ ░░░░░░░░░░░▌▐ ░░░░░░░░░░░▌▐

```

---

## Project Ideation

Modern software projects and deep directory structures accumulate massive amounts of redundant files, libraries, and caches. Standard duplicate finders dump flat text lists that lack structural context, making it hard to see *where* the bloat originates.

**DupliTree** is a high-performance Rust utility designed to scan target directories, intelligently hash files using incremental SHA-256 chunks, exclude non-essential artifacts via a local configuration file, and output a clean, pruned **SVG Tree Visualization** containing *only* your duplicate file clusters and their ancestral paths.

---

## Architecture

The application runs through a tightly optimized 3-phase pipeline:

```text
  [ Target Directory ] 
          │
          ▼
  ┌────────────────────────────────────────┐
  │ PHASE 1: Metrics Gathering             │ ──> Computes total size, file/sub-dir counts
  └───────────────────┬────────────────────┘     (Skipping ignored folders & files)
                      │
                      ▼
  ┌────────────────────────────────────────┐
  │ PHASE 2: Incremental Hashing           │ ──> Streams files through SHA-256 (64KB chunks)
  └───────────────────┬────────────────────┘     (Live single-line CLI status & path truncation)
                      │
                      ▼
  ┌────────────────────────────────────────┐
  │ PHASE 3: Pruning & SVG Rendering       │ ──> Filters out unique files, maps clusters,
  └────────────────────────────────────────┘     and dynamically scales the vector graphics canvas

```

---

## Features

* **⚡ Incremental SHA-256 Hashing:** Streams files efficiently in chunks to minimize memory overhead.
* **⚙️ Config-Driven Exclusions:** Reads a local `config.json` to filter out specific files, wildcards (`*.dll`, `*.pyi`), and directory bloat (`node_modules`, `.target`, etc.).
* **🌳 Cluster-Only SVG Export:** Prunes unique paths from the tree layout, mapping only duplicate nodes with sequential cluster tags (`[Cluster #1]`, `[Cluster #2]`).
* **📏 Dynamic Canvas Scaling:** Automatically expands the SVG width and height based on the maximum tree depth and leaf count, eliminating right-edge clipping.
* **💻 Clean Terminal Feedback:** Uses ANSI escape codes (`\x1b[K`) and smart path truncation to keep progress updates locked to a single, flicker-free terminal line.

---

## Setup & Installation

### 1. Prerequisites

Ensure you have [Rust & Cargo](https://www.rust-lang.org/?utm_source=gemini) installed on your system.

### 2. Dependencies (`Cargo.toml`)

Add the required crates to your project's `Cargo.toml`:

```toml
[dependencies]
sha2 = "0.10"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

```

### 3. Configuration (`config.json`)

Create a `config.json` file in your project's root directory to define exclusions:

```json
{
  "excluded_files": [
    ".DS_Store",
    "Thumbs.db",
    "*.dll",
    "*.pyc",
    "*.bak"
  ],
  "ignored_folders": [
    ".git",
    "node_modules",
    "target",
    ".idea"
  ]
}

```

---

## 4. Running the Project

Execute the binary via Cargo, supplying your target directory path and an optional custom output path for the SVG report:

```bash
cargo run -- "C:\path\to\your\target\directory" duplicates_report.svg
```
or

```
cargo run --release -- "D:\Work\Repos\" output_tree.svg
```

---

## 5. Screenshots

<img src="./screenshots/chrome_VCNVfWVy01.webp" alt="svg" width="360" height="300">
<img src="./screenshots/WindowsTerminal_cG9J7YKxTW.webp" alt="console" width="480" height="300">