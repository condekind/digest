use anyhow::{Context, Result};
use clap::Parser;
use ignore::WalkBuilder;
use log::{debug, info, warn};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tokei::{Config, Languages};

#[derive(Parser, Debug)]
#[clap(
    name = "digest",
    about = "Creates a digest of a codebase for LLM consumption",
    version
)]
struct Cli {
    /// The path to the project directory (defaults to current directory)
    #[clap(index = 1)]
    project_path: Option<PathBuf>,

    /// Maximum number of files to include in the digest
    #[clap(short, long, default_value = "50")]
    max_files: usize,

    /// Maximum file size in KB to consider
    #[clap(short = 's', long, default_value = "100")]
    max_file_size: u64,

    /// Output format (json or markdown)
    #[clap(short, long, default_value = "markdown")]
    format: String,

    /// Output file (defaults to stdout)
    #[clap(short, long)]
    output: Option<PathBuf>,
}

#[derive(Serialize, Debug)]
struct FileInfo {
    path: String,
    language: Option<String>,
    size: u64,
    content: String,
}

#[derive(Serialize, Debug)]
struct Digest {
    project_name: String,
    main_language: Option<String>,
    language_breakdown: HashMap<String, usize>,
    files: Vec<FileInfo>,
}

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    // Determine project path
    let project_path = match cli.project_path {
        Some(path) => path,
        None => env::current_dir()?,
    };

    info!("Analyzing project at: {}", project_path.display());

    // Step 1: Determine the predominant language
    let languages = detect_languages(&project_path)?;
    let language_breakdown = get_language_breakdown(&languages);
    let main_language = get_main_language(&language_breakdown);

    debug!("Main language detected: {:?}", main_language);
    debug!("Language breakdown: {:?}", language_breakdown);

    // Step 2: Build ignore patterns based on the main language
    let ignore_patterns = build_ignore_patterns(&main_language);

    // Step 3: Collect relevant files
    let files = collect_relevant_files(
        &project_path,
        &ignore_patterns,
        cli.max_files,
        cli.max_file_size * 1024, // Convert KB to bytes
    )?;

    info!("Found {} relevant files", files.len());

    // Step 4: Create the digest
    let project_name = project_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();

    let digest = Digest {
        project_name,
        main_language: main_language.clone(),
        language_breakdown,
        files,
    };

    // Step 5: Output the digest
    output_digest(&digest, &cli.format, &cli.output)?;

    Ok(())
}

fn detect_languages(project_path: &Path) -> Result<Languages> {
    let mut languages = Languages::new();
    let config = Config::default();
    languages.get_statistics(&[project_path], &[], &config);
    Ok(languages)
}

fn get_language_breakdown(languages: &Languages) -> HashMap<String, usize> {
    let mut breakdown = HashMap::new();

    for (language, stats) in languages {
        let language_name = format!("{}", language);
        let count = stats.code + stats.comments + stats.blanks;
        breakdown.insert(language_name, count);
    }

    breakdown
}

fn get_main_language(language_breakdown: &HashMap<String, usize>) -> Option<String> {
    language_breakdown
        .iter()
        .max_by_key(|(_, &count)| count)
        .map(|(lang, _)| lang.clone())
}

fn build_ignore_patterns(main_language: &Option<String>) -> HashSet<String> {
    // Common patterns to ignore across all languages
    let mut patterns = HashSet::from([
        ".git".to_string(),
        ".github".to_string(),
        ".vscode".to_string(),
        ".idea".to_string(),
        "node_modules".to_string(),
        "target".to_string(),
        "build".to_string(),
        "dist".to_string(),
        "venv".to_string(),
        "env".to_string(),
        ".env".to_string(),
        ".DS_Store".to_string(),
        "*.log".to_string(),
        "*.lock".to_string(),
        "yarn.lock".to_string(),
        "package-lock.json".to_string(),
    ]);

    // Add language-specific patterns
    if let Some(lang) = main_language {
        match lang.as_str() {
            "JavaScript" | "TypeScript" => {
                patterns.insert("node_modules".to_string());
                patterns.insert("*.min.js".to_string());
                patterns.insert("*.bundle.js".to_string());
            }
            "Python" => {
                patterns.insert("__pycache__".to_string());
                patterns.insert("*.pyc".to_string());
                patterns.insert(".pytest_cache".to_string());
            }
            "Rust" => {
                patterns.insert("target".to_string());
                patterns.insert("Cargo.lock".to_string());
            }
            "Java" => {
                patterns.insert("*.class".to_string());
                patterns.insert("bin".to_string());
                patterns.insert("out".to_string());
            }
            "Go" => {
                patterns.insert("vendor".to_string());
                patterns.insert("*.pb.go".to_string());
            }
            _ => {}
        }
    }

    patterns
}

fn should_ignore(path: &Path, ignore_patterns: &HashSet<String>) -> bool {
    // Get the path as a string
    let path_str = path.to_string_lossy();

    // Check if the path matches any of the ignore patterns
    for pattern in ignore_patterns {
        if pattern.contains('*') {
            // Simple glob matching (this is a very basic implementation)
            let parts: Vec<&str> = pattern.split('*').collect();
            if parts.len() == 2 && pattern.starts_with('*') {
                // Handles "*.extension" pattern
                if path_str.ends_with(parts[1]) {
                    return true;
                }
            } else if parts.len() == 2 && pattern.ends_with('*') {
                // Handles "prefix*" pattern
                if path_str.starts_with(parts[0]) {
                    return true;
                }
            }
        } else {
            // Direct matches
            if path_str.contains(pattern) {
                return true;
            }
        }
    }

    false
}

fn collect_relevant_files(
    project_path: &Path,
    ignore_patterns: &HashSet<String>,
    max_files: usize,
    max_file_size: u64,
) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();

    // Use the ignore crate to respect .gitignore files
    let walker = WalkBuilder::new(project_path)
        .hidden(false)  // Include hidden files
        .git_ignore(true)  // Respect .gitignore
        .build();

    for result in walker {
        let entry = match result {
            Ok(entry) => entry,
            Err(err) => {
                warn!("Error accessing entry: {}", err);
                continue;
            }
        };

        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Skip files that match ignore patterns
        if should_ignore(path, ignore_patterns) {
            debug!("Ignoring file: {}", path.display());
            continue;
        }

        // Check file size
        let metadata = match fs::metadata(path) {
            Ok(meta) => meta,
            Err(err) => {
                warn!("Error reading metadata for {}: {}", path.display(), err);
                continue;
            }
        };

        if metadata.len() > max_file_size {
            debug!("Skipping large file: {} ({} bytes)", path.display(), metadata.len());
            continue;
        }

        // Read file content
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                warn!("Error reading file {}: {}", path.display(), err);
                continue;
            }
        };

        // Determine file language based on extension
        let language = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext {
                "rs" => "Rust",
                "js" => "JavaScript",
                "ts" => "TypeScript",
                "py" => "Python",
                "java" => "Java",
                "go" => "Go",
                "c" | "cpp" | "h" | "hpp" => "C/C++",
                "rb" => "Ruby",
                "php" => "PHP",
                "cs" => "C#",
                "html" => "HTML",
                "css" => "CSS",
                "json" => "JSON",
                "md" => "Markdown",
                "yml" | "yaml" => "YAML",
                "toml" => "TOML",
                _ => "Unknown",
            })
            .map(String::from);

        let relative_path = path.strip_prefix(project_path)
            .with_context(|| format!("Failed to strip prefix from {}", path.display()))?
            .to_string_lossy()
            .to_string();

        files.push(FileInfo {
            path: relative_path,
            language,
            size: metadata.len(),
            content,
        });

        if files.len() >= max_files {
            break;
        }
    }

    Ok(files)
}

fn output_digest(digest: &Digest, format: &str, output_path: &Option<PathBuf>) -> Result<()> {
    let content = match format {
        "json" => serde_json::to_string_pretty(digest)?,
        "markdown" => format_markdown(digest),
        _ => return Err(anyhow::anyhow!("Unsupported output format: {}", format)),
    };

    match output_path {
        Some(path) => {
            fs::write(path, content)?;
            info!("Digest written to {}", path.display());
        }
        None => {
            // Print to stdout
            println!("{}", content);
        }
    }

    Ok(())
}

fn format_markdown(digest: &Digest) -> String {
    let mut output = String::new();

    // Project header
    output.push_str(&format!("# Project Digest: {}\n\n", digest.project_name));

    // Language summary
    output.push_str("## Language Breakdown\n\n");
    if let Some(main) = &digest.main_language {
        output.push_str(&format!("Main language: **{}**\n\n", main));
    }

    output.push_str("| Language | Lines |\n");
    output.push_str("|----------|-------|\n");

    let mut languages: Vec<(String, usize)> = digest.language_breakdown.iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    languages.sort_by(|a, b| b.1.cmp(&a.1));

    for (lang, count) in languages {
        output.push_str(&format!("| {} | {} |\n", lang, count));
    }
    output.push_str("\n");

    // Files
    output.push_str("## Files\n\n");

    for file in &digest.files {
        output.push_str(&format!("### {}\n\n", file.path));

        if let Some(lang) = &file.language {
            output.push_str(&format!("**Language**: {}\n\n", lang));
        }

        output.push_str(&format!("**Size**: {} bytes\n\n", file.size));

        output.push_str("```");
        if let Some(lang) = &file.language {
            let lang_tag = match lang.as_str() {
                "JavaScript" => "js",
                "TypeScript" => "ts",
                "Python" => "python",
                "Rust" => "rust",
                "Java" => "java",
                "Go" => "go",
                "C/C++" => "cpp",
                "Ruby" => "ruby",
                "PHP" => "php",
                "C#" => "cs",
                "HTML" => "html",
                "CSS" => "css",
                "JSON" => "json",
                "Markdown" => "md",
                "YAML" => "yaml",
                "TOML" => "toml",
                _ => "",
            };
            if !lang_tag.is_empty() {
                output.push_str(lang_tag);
            }
        }
        output.push_str("\n");
        output.push_str(&file.content);
        output.push_str("\n```\n\n");
    }

    output
}

// Extension trait to make Path to string conversion more convenient
trait PathToStringExt {
    fn to_string_lossy(&self) -> String;
}

impl PathToStringExt for Path {
    fn to_string_lossy(&self) -> String {
        self.to_string_lossy().to_string()
    }
}

