use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;
use std::thread;

// ── Types & Definitions ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolCategory {
    Rust,
    Python,
    NodeJS,
    Go,
    Systems,
}

impl ToolCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "Rust Toolchain",
            Self::Python => "Python Toolchain",
            Self::NodeJS => "Node/JS Toolchain",
            Self::Go => "Go Toolchain",
            Self::Systems => "Systems & Utilities",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tool {
    pub name: &'static str,
    pub binary: &'static str,
    pub category: ToolCategory,
    pub version_args: &'static [&'static str],
}

pub static TOOLS: &[Tool] = &[
    // Rust
    Tool {
        name: "cargo",
        binary: "cargo",
        category: ToolCategory::Rust,
        version_args: &["--version"],
    },
    Tool {
        name: "rustc",
        binary: "rustc",
        category: ToolCategory::Rust,
        version_args: &["--version"],
    },
    Tool {
        name: "rustup",
        binary: "rustup",
        category: ToolCategory::Rust,
        version_args: &["--version"],
    },
    // Python
    Tool {
        name: "python",
        binary: "python3",
        category: ToolCategory::Python,
        version_args: &["--version"],
    },
    Tool {
        name: "pip",
        binary: "pip",
        category: ToolCategory::Python,
        version_args: &["--version"],
    },
    Tool {
        name: "uv",
        binary: "uv",
        category: ToolCategory::Python,
        version_args: &["--version"],
    },
    Tool {
        name: "pipx",
        binary: "pipx",
        category: ToolCategory::Python,
        version_args: &["--version"],
    },
    // Node/JS
    Tool {
        name: "node",
        binary: "node",
        category: ToolCategory::NodeJS,
        version_args: &["--version"],
    },
    Tool {
        name: "npm",
        binary: "npm",
        category: ToolCategory::NodeJS,
        version_args: &["--version"],
    },
    Tool {
        name: "pnpm",
        binary: "pnpm",
        category: ToolCategory::NodeJS,
        version_args: &["--version"],
    },
    Tool {
        name: "yarn",
        binary: "yarn",
        category: ToolCategory::NodeJS,
        version_args: &["--version"],
    },
    Tool {
        name: "bun",
        binary: "bun",
        category: ToolCategory::NodeJS,
        version_args: &["--version"],
    },
    Tool {
        name: "deno",
        binary: "deno",
        category: ToolCategory::NodeJS,
        version_args: &["--version"],
    },
    // Go
    Tool {
        name: "go",
        binary: "go",
        category: ToolCategory::Go,
        version_args: &["version"],
    },
    // Systems
    Tool {
        name: "git",
        binary: "git",
        category: ToolCategory::Systems,
        version_args: &["--version"],
    },
    Tool {
        name: "docker",
        binary: "docker",
        category: ToolCategory::Systems,
        version_args: &["--version"],
    },
    Tool {
        name: "docker-compose",
        binary: "docker-compose",
        category: ToolCategory::Systems,
        version_args: &["--version"],
    },
    Tool {
        name: "curl",
        binary: "curl",
        category: ToolCategory::Systems,
        version_args: &["--version"],
    },
    Tool {
        name: "make",
        binary: "make",
        category: ToolCategory::Systems,
        version_args: &["--version"],
    },
];

#[derive(Debug, Clone)]
pub struct DiagnosticResult {
    pub name: String,
    pub category: ToolCategory,
    pub is_installed: bool,
    pub path: Option<PathBuf>,
    pub version: Option<String>,
}

// ── Path Resolution Helper ──────────────────────────────────────────────────

pub fn find_binary_path(binary: &str) -> Option<PathBuf> {
    if let Some(paths) = std::env::var_os("PATH") {
        for path in std::env::split_paths(&paths) {
            let candidate = path.join(binary);
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if candidate.is_file() {
                    if let Ok(metadata) = candidate.metadata() {
                        // Check if file is executable by user/group/others
                        if metadata.mode() & 0o111 != 0 {
                            return Some(candidate);
                        }
                    }
                }
            }
            #[cfg(not(unix))]
            {
                if candidate.is_file() {
                    return Some(candidate);
                }
                // On Windows, the binary may have an extension
                for ext in ["exe", "bat", "cmd"] {
                    let with_ext = candidate.with_extension(ext);
                    if with_ext.is_file() {
                        return Some(with_ext);
                    }
                }
            }
        }
    }
    None
}

// ── Version Parsing ──────────────────────────────────────────────────────────

pub fn parse_version(output: &str) -> Option<String> {
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        for word in line.split_whitespace() {
            // Strip leading/trailing non-alphanumeric chars (like commas, parentheses, quotes)
            let mut cleaned = word
                .trim_matches(|c: char| !c.is_alphanumeric() && c != '.')
                .to_string();

            if cleaned.starts_with('v')
                && cleaned
                    .chars()
                    .nth(1)
                    .map_or(false, |c| c.is_ascii_digit())
            {
                cleaned = cleaned[1..].to_string();
            } else if cleaned.starts_with("go")
                && cleaned
                    .chars()
                    .nth(2)
                    .map_or(false, |c| c.is_ascii_digit())
            {
                cleaned = cleaned[2..].to_string();
            }

            // A version string starts with a digit and contains at least one dot
            if cleaned
                .chars()
                .next()
                .map_or(false, |c| c.is_ascii_digit())
                && cleaned.contains('.')
            {
                let final_version = cleaned
                    .trim_matches(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
                    .to_string();
                if !final_version.is_empty() {
                    return Some(final_version);
                }
            }
        }
    }
    None
}

// ── Threaded Runner ──────────────────────────────────────────────────────────

fn check_individual_tool(tool: &Tool) -> DiagnosticResult {
    let mut binary_name = tool.binary.to_string();
    let mut path = find_binary_path(&binary_name);

    // Fallback for python3 to python if not found
    if path.is_none() && tool.binary == "python3" {
        binary_name = "python".to_string();
        path = find_binary_path(&binary_name);
    }

    if let Some(p) = path {
        let mut cmd = Command::new(&binary_name);
        for arg in tool.version_args {
            cmd.arg(arg);
        }

        match cmd.output() {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);

                let version = parse_version(&stdout)
                    .or_else(|| parse_version(&stderr))
                    .unwrap_or_else(|| "unknown version".to_string());

                DiagnosticResult {
                    name: tool.name.to_string(),
                    category: tool.category,
                    is_installed: true,
                    path: Some(p),
                    version: Some(version),
                }
            }
            Err(_) => DiagnosticResult {
                name: tool.name.to_string(),
                category: tool.category,
                is_installed: true,
                path: Some(p),
                version: Some("unknown (execution failed)".to_string()),
            },
        }
    } else {
        DiagnosticResult {
            name: tool.name.to_string(),
            category: tool.category,
            is_installed: false,
            path: None,
            version: None,
        }
    }
}

// ── Main Entry ───────────────────────────────────────────────────────────────

pub fn run() -> Result<()> {
    println!();
    println!("  {}", "Scanning system utilities & runtimes...".cyan().bold());

    // Spawn concurrent checks using std::thread
    let mut handles = Vec::new();
    for tool in TOOLS {
        let tool = tool.clone();
        let handle = thread::spawn(move || check_individual_tool(&tool));
        handles.push(handle);
    }

    let mut results = Vec::new();
    for handle in handles {
        if let Ok(res) = handle.join() {
            results.push(res);
        }
    }

    // Group and render by category
    let categories = [
        ToolCategory::Rust,
        ToolCategory::Python,
        ToolCategory::NodeJS,
        ToolCategory::Go,
        ToolCategory::Systems,
    ];

    for cat in &categories {
        println!("\n  {}", format!("── {} ──", cat.as_str().to_uppercase()).blue().bold());
        
        let cat_results: Vec<&DiagnosticResult> = results
            .iter()
            .filter(|r| r.category == *cat)
            .collect();

        for r in cat_results {
            if r.is_installed {
                let ver_str = r.version.as_deref().unwrap_or("unknown");
                let path_str = r
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();

                println!(
                    "  {}  {:<16}  {:<12}  {}",
                    "✓".green().bold(),
                    r.name.bold(),
                    ver_str,
                    path_str.dimmed()
                );
            } else {
                println!(
                    "  {}  {:<16}  {}",
                    "✗".red().dimmed(),
                    r.name.dimmed(),
                    "not found".red().dimmed()
                );
            }
        }
    }

    // Smart Cross-Dependency Doctor Validations
    let installed_names: HashSet<&str> = results
        .iter()
        .filter(|r| r.is_installed)
        .map(|r| r.name.as_str())
        .collect();

    let mut warnings = Vec::new();

    // NodeJS warnings
    let has_node = installed_names.contains("node");
    for &pkg_mgr in &["npm", "pnpm", "yarn"] {
        if installed_names.contains(pkg_mgr) && !has_node {
            warnings.push(format!(
                "Package manager {} is installed, but the Node.js runtime ({}) is missing.\n     Install Node.js to enable the package manager to execute packages.",
                pkg_mgr.bold().yellow(),
                "node".bold()
            ));
        }
    }

    // Python warnings
    let has_python = installed_names.contains("python");
    for &py_tool in &["pip", "uv", "pipx"] {
        if installed_names.contains(py_tool) && !has_python {
            warnings.push(format!(
                "Python utility {} is installed, but no Python runtime is active on $PATH.\n     Install Python (python3) to run Python packages.",
                py_tool.bold().yellow()
            ));
        }
    }

    // Rust warnings
    let has_rustup = installed_names.contains("rustup");
    if (installed_names.contains("cargo") || installed_names.contains("rustc")) && !has_rustup {
        warnings.push(format!(
            "Rust toolchain is active, but {} was not found on your $PATH.\n     Recommend installing rustup (https://rustup.rs) to manage toolchains.",
            "rustup".bold().yellow()
        ));
    }

    // Docker warnings
    if installed_names.contains("docker-compose") && !installed_names.contains("docker") {
        warnings.push(format!(
            "{} is installed, but the core {} utility is missing.\n     Docker Compose requires a local Docker installation.",
            "docker-compose".bold().yellow(),
            "docker".bold()
        ));
    }

    if !warnings.is_empty() {
        println!();
        println!("  {}", "⚠️  Warnings & Recommendations:".yellow().bold());
        for warning in warnings {
            println!("  • {}", warning);
        }
    }

    println!();
    Ok(())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(
            parse_version("cargo 1.78.0 (54d84157b 2024-05-03)").as_deref(),
            Some("1.78.0")
        );
        assert_eq!(
            parse_version("rustc 1.78.0 (9b00956e5 2024-04-29)").as_deref(),
            Some("1.78.0")
        );
        assert_eq!(parse_version("Python 3.10.12").as_deref(), Some("3.10.12"));
        assert_eq!(parse_version("v18.16.0").as_deref(), Some("18.16.0"));
        assert_eq!(
            parse_version("go version go1.21.3 linux/amd64").as_deref(),
            Some("1.21.3")
        );
        assert_eq!(
            parse_version("Docker version 24.0.2, build cb74df5").as_deref(),
            Some("24.0.2")
        );
        assert_eq!(parse_version("GNU Make 4.3").as_deref(), Some("4.3"));
        assert_eq!(parse_version("unknown output").as_deref(), None);
    }

    #[test]
    fn test_find_binary_path() {
        // "cargo" should always be found during unit tests
        let path = find_binary_path("cargo");
        assert!(path.is_some());
    }
}
