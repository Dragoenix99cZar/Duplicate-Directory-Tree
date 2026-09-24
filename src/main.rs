use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug, Default)]
struct Config {
    #[serde(default)]
    excluded_files: Vec<String>,
    #[serde(default)]
    ignored_folders: Vec<String>,
}

#[derive(Clone, Debug)]
struct FileNode {
    name: String,
    path: PathBuf,
    size: u64,
    hash: Option<String>,
    is_dir: bool,
    children: Vec<FileNode>,
}

struct LayoutNode {
    name: String,
    x: f64,
    y: f64,
    is_dir: bool,
    is_duplicate: bool,
    cluster_id: Option<usize>,
    children: Vec<LayoutNode>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run -- <DIRECTORY_PATH> [output.svg]");
        std::process::exit(1);
    }

    let target_dir = &args[1];
    let output_svg = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "directory_tree.svg".to_string());
    let path = Path::new(target_dir);

    if !path.exists() {
        eprintln!("Error: Path does not exist.");
        std::process::exit(1);
    }

    // Load config.json if present
    let config = load_config();
    println!(
        "Loaded config: {} excluded file patterns, {} ignored folders.",
        config.excluded_files.len(),
        config.ignored_folders.len()
    );

    // ========================================================================
    // PHASE 1: Get directory size, sub-directory count, files-count
    // ========================================================================
    println!("============================================================");
    println!("PHASE 1: Collecting Directory Metrics");
    println!("============================================================");
    let (total_size, file_count, sub_dir_count) = gather_directory_stats(path, &config);
    println!("Target Directory : {}", path.display());
    println!(
        "Total Size       : {} ({})",
        total_size,
        format_file_size(total_size)
    );
    println!("Files Count      : {}", file_count);
    println!("Sub-dirs Count   : {}\n", sub_dir_count);

    // ========================================================================
    // PHASE 2: Processing/hashing with live single-line progress & flush
    // ========================================================================
    println!("============================================================");
    println!("PHASE 2: Processing & Hashing Files");
    println!("============================================================");
    let mut hash_to_files: HashMap<String, Vec<(PathBuf, u64)>> = HashMap::new();
    let mut files_processed = 0;
    let mut bytes_processed = 0;

    let root_node = build_tree_and_hash(
        path,
        &mut hash_to_files,
        file_count,
        total_size,
        &mut files_processed,
        &mut bytes_processed,
        &config,
    );

    // Print a final newline to clear the active progress line cursor
    println!("\n\nHashing complete!\n");

    // ========================================================================
    // PHASE 3: Generating report: list of duplicate files, paths, size, etc.
    // ========================================================================
    println!("============================================================");
    println!("PHASE 3: Generating Duplicate Report & SVG Tree");
    println!("============================================================");

    let duplicates: Vec<(&String, &Vec<(PathBuf, u64)>)> = hash_to_files
        .iter()
        .filter(|(_, files)| files.len() > 1)
        .collect();

    // Build a lookup mapping for cluster IDs (1-indexed)
    let mut hash_to_cluster_id: HashMap<String, usize> = HashMap::new();
    if duplicates.is_empty() {
        println!("No duplicate files found.");
    } else {
        println!("Found {} duplicate file clusters:\n", duplicates.len());
        for (i, (hash, files)) in duplicates.iter().enumerate() {
            let cluster_num = i + 1;
            hash_to_cluster_id.insert((*hash).clone(), cluster_num);

            println!("Cluster #{}: Hash [{}...]", cluster_num, &hash[..8]);
            let file_size = files[0].1;
            println!(
                "  File Size: {} each ({})",
                file_size,
                format_file_size(file_size)
            );
            println!("  Instances:");
            for (fpath, _) in *files {
                println!("    - {}", fpath.display());
            }
            println!();
        }
    }

    // Prune the tree so ONLY duplicate clusters and their parent paths remain
    if let Some(pruned_root) = prune_non_duplicates(root_node, &hash_to_cluster_id) {
        let mut leaf_counter = 0;
        let layout_root = compute_layout(&pruned_root, 0, &hash_to_cluster_id, &mut leaf_counter);
        let svg_content = render_svg(&layout_root, leaf_counter);

        fs::write(&output_svg, svg_content).expect("Failed to write SVG file");
        println!(
            "SVG Tree successfully generated and saved to: {}",
            output_svg
        );
    } else {
        println!("No duplicates found, skipping SVG tree generation.");
    }
}

// --- Configuration Loader & Filter Helpers ---

fn load_config() -> Config {
    if let Ok(data) = fs::read_to_string("config.json") {
        if let Ok(config) = serde_json::from_str(&data) {
            return config;
        } else {
            eprintln!("Warning: Failed to parse config.json, using defaults.");
        }
    } else {
        println!("Note: config.json not found, running with zero exclusions.");
    }
    Config::default()
}

fn should_ignore_folder(name: &str, ignored_folders: &[String]) -> bool {
    ignored_folders.iter().any(|folder| folder == name)
}

fn should_exclude_file(name: &str, excluded_files: &[String]) -> bool {
    for pattern in excluded_files {
        if pattern.starts_with("*.") || pattern.starts_with('*') {
            let ext = &pattern[1..];
            if name.ends_with(ext) {
                return true;
            }
        } else {
            if name == pattern {
                return true;
            }
        }
    }
    false
}

// --- Tree Pruning Helper ---

fn prune_non_duplicates(
    node: FileNode,
    hash_cluster_map: &HashMap<String, usize>,
) -> Option<FileNode> {
    if node.is_dir {
        let mut filtered_children = Vec::new();
        for child in node.children {
            if let Some(pruned_child) = prune_non_duplicates(child, hash_cluster_map) {
                filtered_children.push(pruned_child);
            }
        }
        if filtered_children.is_empty() {
            None
        } else {
            Some(FileNode {
                name: node.name,
                path: node.path,
                size: node.size,
                hash: node.hash,
                is_dir: node.is_dir,
                children: filtered_children,
            })
        }
    } else {
        if let Some(h) = &node.hash {
            if hash_cluster_map.contains_key(h) {
                return Some(node);
            }
        }
        None
    }
}

// --- Helper Functions for the Phases ---

fn gather_directory_stats(path: &Path, config: &Config) -> (u64, usize, usize) {
    let mut total_size = 0;
    let mut file_count = 0;
    let mut sub_dir_count = 0;

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            let name = p.file_name().unwrap_or_default().to_string_lossy();

            if p.is_dir() {
                if should_ignore_folder(&name, &config.ignored_folders) {
                    continue;
                }
                sub_dir_count += 1;
                let (sub_size, sub_files, sub_dirs) = gather_directory_stats(&p, config);
                total_size += sub_size;
                file_count += sub_files;
                sub_dir_count += sub_dirs;
            } else if p.is_file() {
                if should_exclude_file(&name, &config.excluded_files) {
                    continue;
                }
                file_count += 1;
                if let Ok(m) = fs::metadata(&p) {
                    total_size += m.len();
                }
            }
        }
    }
    (total_size, file_count, sub_dir_count)
}

fn build_tree_and_hash(
    path: &Path,
    hash_to_files: &mut HashMap<String, Vec<(PathBuf, u64)>>,
    total_files: usize,
    total_bytes: u64,
    files_processed: &mut usize,
    bytes_processed: &mut u64,
    config: &Config,
) -> FileNode {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let is_dir = path.is_dir();
    let mut children = Vec::new();
    let mut file_hash = None;
    let mut size = 0;

    if is_dir {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                let entry_name = entry_path.file_name().unwrap_or_default().to_string_lossy();

                if entry_path.is_dir() && should_ignore_folder(&entry_name, &config.ignored_folders)
                {
                    continue;
                }
                if entry_path.is_file() && should_exclude_file(&entry_name, &config.excluded_files)
                {
                    continue;
                }

                children.push(build_tree_and_hash(
                    &entry_path,
                    hash_to_files,
                    total_files,
                    total_bytes,
                    files_processed,
                    bytes_processed,
                    config,
                ));
            }
        }
    } else {
        if let Ok(metadata) = fs::metadata(path) {
            size = metadata.len();
        }

        *files_processed += 1;
        *bytes_processed += size;

        let percent = if total_files > 0 {
            (*files_processed as f64 / total_files as f64) * 100.0
        } else {
            0.0
        };

        let path_str = path.display().to_string();
        let display_path = if path_str.chars().count() > 60 {
            let truncated: String = path_str
                .chars()
                .skip(path_str.chars().count() - 57)
                .collect();
            format!("...{}", truncated)
        } else {
            path_str
        };

        print!(
            "\r\x1b[K-> Hashing: {} [{}/{} | {:.1}% | {} of {} processed]",
            display_path,
            files_processed,
            total_files,
            percent,
            format_file_size(*bytes_processed),
            format_file_size(total_bytes)
        );
        let _ = io::stdout().flush();

        if let Ok(h) = hash_file_incrementally(path) {
            hash_to_files
                .entry(h.clone())
                .or_default()
                .push((path.to_path_buf(), size));
            file_hash = Some(h);
        }
    }

    FileNode {
        name,
        path: path.to_path_buf(),
        size,
        hash: file_hash,
        is_dir,
        children,
    }
}

fn hash_file_incrementally(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024]; // 64 KB buffer

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

fn compute_layout(
    node: &FileNode,
    depth: usize,
    hash_cluster_map: &HashMap<String, usize>,
    leaf_counter: &mut usize,
) -> LayoutNode {
    let x = (depth as f64) * 180.0 + 50.0;

    let (is_duplicate, cluster_id) = if let Some(h) = &node.hash {
        if let Some(&cluster_num) = hash_cluster_map.get(h) {
            (true, Some(cluster_num))
        } else {
            (false, None)
        }
    } else {
        (false, None)
    };

    let mut layout_children = Vec::new();
    if node.is_dir && !node.children.is_empty() {
        for child in &node.children {
            layout_children.push(compute_layout(
                child,
                depth + 1,
                hash_cluster_map,
                leaf_counter,
            ));
        }
    } else {
        *leaf_counter += 1;
    }

    let y = if layout_children.is_empty() {
        (*leaf_counter as f64) * 35.0 + 50.0
    } else {
        let sum_y: f64 = layout_children.iter().map(|c| c.y).sum();
        sum_y / layout_children.len() as f64
    };

    LayoutNode {
        name: node.name.clone(),
        x,
        y,
        is_dir: node.is_dir,
        is_duplicate,
        cluster_id,
        children: layout_children,
    }
}

// Helper to calculate maximum depth of layout nodes
fn get_max_depth(node: &LayoutNode, current_depth: usize) -> usize {
    if node.children.is_empty() {
        current_depth
    } else {
        node.children
            .iter()
            .map(|child| get_max_depth(child, current_depth + 1))
            .max()
            .unwrap_or(current_depth)
    }
}

// Helper to escape raw text for valid XML output
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn render_svg(root: &LayoutNode, total_leaves: usize) -> String {
    let max_depth = get_max_depth(root, 0);
    // Dynamically expand width based on depth + generous space for text strings on the right
    let width = (((max_depth + 1) as f64) * 180.0 + 400.0).max(1200.0) as usize;
    let height = ((total_leaves * 35) + 100).max(600);

    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" style="background-color: #1e1e1e; font-family: sans-serif;">
"#,
        width, height, width, height
    );

    svg.push_str(
        r#"  <style>
    .node-text { fill: #d4d4d4; font-size: 12px; }
    .dup-text { fill: #f44747; font-size: 12px; font-weight: bold; }
    .link { stroke: #555555; stroke-width: 1.5; fill: none; }
    .node-dir { fill: #4ec9b0; stroke: #222; stroke-width: 1.5; }
    .node-file { fill: #9cdcfe; stroke: #222; stroke-width: 1.5; }
    .node-dup { fill: #f44747; stroke: #ff8888; stroke-width: 2px; }
  </style>
"#,
    );

    let mut paths_svg = String::new();
    let mut nodes_svg = String::new();
    render_nodes_recursive(root, &mut paths_svg, &mut nodes_svg);

    svg.push_str(&paths_svg);
    svg.push_str(&nodes_svg);
    svg.push_str("</svg>");
    svg
}

fn render_nodes_recursive(node: &LayoutNode, paths: &mut String, nodes: &mut String) {
    for child in &node.children {
        let mid_x = (node.x + child.x) / 2.0;
        paths.push_str(&format!(
            r#"  <path class="link" d="M {} {} C {} {}, {} {}, {} {}" />
"#,
            node.x, node.y, mid_x, node.y, mid_x, child.y, child.x, child.y
        ));

        render_nodes_recursive(child, paths, nodes);
    }

    let (css_class, text_class) = if node.is_dir {
        ("node-dir", "node-text")
    } else if node.is_duplicate {
        ("node-dup", "dup-text")
    } else {
        ("node-file", "node-text")
    };

    let radius = if node.is_dir { 6.0 } else { 4.5 };

    nodes.push_str(&format!(
        r#"  <circle cx="{}" cy="{}" r="{}" class="{}" />
"#,
        node.x, node.y, radius, css_class
    ));

    let raw_label = if let Some(cluster_id) = node.cluster_id {
        format!("{} [Cluster #{}]", node.name, cluster_id)
    } else {
        node.name.clone()
    };
    let label = escape_xml(&raw_label);

    nodes.push_str(&format!(
        r#"  <text x="{}" y="{}" class="{}">{}</text>
"#,
        node.x + 10.0,
        node.y + 4.0,
        text_class,
        label
    ));
}
