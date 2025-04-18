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
    #[clap(short = 's', long, default_value = "500")]
    max_file_size: u64,

    /// Output format (json or markdown)
    #[clap(short, long, default_value = "markdown")]
    format: String,

    /// Output file (defaults to stdout)
    #[clap(short, long)]
    output: Option<PathBuf>,

    /// List files that would be included without generating the digest
    #[clap(short, long)]
    list: bool,

    /// Disable using .gitignore for ignore patterns
    #[clap(long)]
    no_gitignore: bool,

    /// Disable using .digestignore for ignore patterns
    #[clap(long)]
    no_digestignore: bool,

    /// Disable all ignore patterns (both .gitignore and .digestignore)
    #[clap(long)]
    no_ignore: bool,

    /// Additional patterns to ignore (can be specified multiple times)
    #[clap(long = "ignore-pattern", value_name = "PATTERN")]
    ignore_patterns: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct FileInfo {
    pub path: String,
    pub language: Option<String>,
    pub content: String,
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

    // Check if it's a Godot project
    let is_godot_project = is_godot_project(&project_path);
    if is_godot_project {
        info!("Detected Godot project");
    }

    // Check if it's a Lua project
    let is_lua_project = is_lua_project(&project_path);
    if is_lua_project {
        info!("Detected Lua project");
    }

    // Step 1: Determine the predominant language
    let languages = detect_languages(&project_path)?;
    let language_breakdown = get_language_breakdown(&languages);
    let main_language = get_main_language(&language_breakdown);

    debug!("Main language detected: {:?}", main_language);
    debug!("Language breakdown: {:?}", language_breakdown);

    // Step 2: Get ignore patterns from .digestignore, .gitignore, or defaults
    let mut ignore_patterns = HashSet::new();

    // Don't process any ignore files if --no-ignore is used
    if !cli.no_ignore {
        // Try to get patterns from .digestignore, unless --no-digestignore is used
        let using_digestignore = if !cli.no_digestignore {
            match check_for_digestignore(&project_path) {
                Ok(digestignore_patterns) => {
                    ignore_patterns.extend(digestignore_patterns);
                    true
                }
                Err(_) => {
                    debug!("No .digestignore file found.");
                    false
                }
            }
        } else {
            debug!("Skipping .digestignore due to --no-digestignore flag.");
            false
        };

        // Try to get patterns from .gitignore, unless --no-gitignore is used
        let using_gitignore = if !cli.no_gitignore {
            match check_for_gitignore(&project_path) {
                Ok(gitignore_patterns) => {
                    ignore_patterns.extend(gitignore_patterns);
                    true
                }
                Err(_) => {
                    debug!("No .gitignore file found.");
                    false
                }
            }
        } else {
            debug!("Skipping .gitignore due to --no-gitignore flag.");
            false
        };

        // If no ignore files were found or used, use default patterns
        if ignore_patterns.is_empty() {
            info!("No ignore files found or used. Using default ignore patterns.");
            ignore_patterns = build_ignore_patterns(&main_language, is_godot_project);
        } else {
            let mut ignore_sources = Vec::new();
            if using_digestignore {
                ignore_sources.push(".digestignore");
            }
            if using_gitignore {
                ignore_sources.push(".gitignore");
            }
            info!("Using ignore patterns from: {}", ignore_sources.join(", "));
        }
    } else {
        info!("Ignoring all ignore files due to --no-ignore flag.");
        // Always ignore .git directory at minimum
        ignore_patterns.insert(".git".to_string());
    }

    // Add patterns from --ignore-pattern CLI arguments
    if !cli.ignore_patterns.is_empty() {
        info!(
            "Adding {} custom ignore patterns from command line",
            cli.ignore_patterns.len()
        );
        for pattern in &cli.ignore_patterns {
            ignore_patterns.insert(pattern.clone());
        }
    }

    // Step 3: Collect relevant files
    let files = collect_relevant_files(
        &project_path,
        &ignore_patterns,
        cli.max_files,
        cli.max_file_size * 1024, // Convert KB to bytes
        is_godot_project,
        !cli.no_gitignore && !cli.no_ignore, // Respect gitignore unless disabled
    )?;

    info!("Found {} relevant files", files.len());

    // If list option is specified, just print the file paths and exit
    if cli.list {
        println!("Files that would be included in the digest:");
        for file in &files {
            println!("{}", file.path);
        }
        return Ok(());
    }

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

pub fn build_ignore_patterns(
    main_language: &Option<String>,
    is_godot_project: bool,
) -> HashSet<String> {
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
        ".venv".to_string(),
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
            "Lua" => {
                patterns.insert("*.luac".to_string()); // Compiled Lua files
                patterns.insert("luarocks".to_string()); // LuaRocks package manager directory
            }
            "C#" => {
                // If it's not a Godot project, use default C# ignores
                if !is_godot_project {
                    patterns.insert("bin".to_string());
                    patterns.insert("obj".to_string());
                    patterns.insert("*.dll".to_string());
                }
            }
            _ => {}
        }
    }

    // For Godot projects, make sure we don't ignore important Godot files
    if is_godot_project {
        // Don't ignore .import directory as it contains important Godot metadata
        patterns.remove(".import");
        // Don't ignore addons directory as it contains Godot plugins
        patterns.remove("addons");
    }

    patterns
}

pub fn check_for_digestignore(project_path: &Path) -> Result<HashSet<String>> {
    let digestignore_path = project_path.join(".digestignore");

    if !digestignore_path.exists() {
        return Err(anyhow::anyhow!("No .digestignore file found"));
    }

    info!(
        "Using .digestignore file at {}",
        digestignore_path.display()
    );

    // Use the ignore crate to build a gitignore-like matcher from the .digestignore file
    let content = fs::read_to_string(&digestignore_path).with_context(|| {
        format!(
            "Failed to read .digestignore at {}",
            digestignore_path.display()
        )
    })?;

    // Add .git to always ignore
    let mut patterns = HashSet::from([".git".to_string()]);

    for line in content.lines() {
        let line = line.trim();
        // Skip empty lines and comments
        if !line.is_empty() && !line.starts_with('#') {
            patterns.insert(line.to_string());
        }
    }

    Ok(patterns)
}

pub fn check_for_gitignore(project_path: &Path) -> Result<HashSet<String>> {
    let gitignore_path = project_path.join(".gitignore");

    if !gitignore_path.exists() {
        return Err(anyhow::anyhow!("No .gitignore file found"));
    }

    info!("Using .gitignore file at {}", gitignore_path.display());

    // Read the .gitignore file
    let content = fs::read_to_string(&gitignore_path)
        .with_context(|| format!("Failed to read .gitignore at {}", gitignore_path.display()))?;

    // Add .git to always ignore
    let mut patterns = HashSet::from([".git".to_string()]);

    for line in content.lines() {
        let line = line.trim();
        // Skip empty lines and comments
        if !line.is_empty() && !line.starts_with('#') {
            patterns.insert(line.to_string());
        }
    }

    Ok(patterns)
}

pub fn should_ignore(path: &Path, ignore_patterns: &HashSet<String>) -> bool {
    // Get the path as a string
    let path_str = path.to_string_lossy();

    // Normalize path for matching (replace backslashes with forward slashes on Windows)
    let path_str = path_str.replace('\\', "/");

    // Check if the path matches any of the ignore patterns
    for pattern in ignore_patterns {
        // Special case for **/test/** pattern since it's common and important
        if pattern == "**/test/**" {
            if path_str.contains("/test/") || path_str.starts_with("test/") {
                debug!("Ignoring {} - matches **/test/** pattern", path_str);
                return true;
            }
        }

        // Always ignore .git directory
        if path_str.contains("/.git/") || path_str == ".git" {
            debug!("Ignoring {} - matches .git pattern", path_str);
            return true;
        }

        // Handle different gitignore pattern types
        let pattern = pattern.trim();

        // Empty lines or comments
        if pattern.is_empty() || pattern.starts_with('#') {
            continue;
        }

        // Negated patterns (we're not supporting these for simplicity)
        if pattern.starts_with('!') {
            continue;
        }

        // Handle **/ pattern at the beginning (match any directory depth)
        if pattern.starts_with("**/") {
            let suffix = &pattern[3..];
            // Check if suffix appears anywhere in the path
            if path_str == suffix
                || path_str.ends_with(suffix)
                || path_str.contains(&format!("/{}", suffix))
            {
                debug!("Ignoring {} - matches **/ pattern: {}", path_str, pattern);
                return true;
            }
        }

        // Handle pattern ending with /** (match any subdirectory)
        if pattern.ends_with("/**") {
            let prefix = &pattern[0..pattern.len() - 3];
            if path_str.starts_with(prefix) || path_str.contains(&format!("/{}", prefix)) {
                debug!("Ignoring {} - matches /** pattern: {}", path_str, pattern);
                return true;
            }
        }

        // Handle /**/ pattern (matches any directory in the middle)
        if pattern.contains("/**/") {
            let segments: Vec<&str> = pattern.split("/**/").collect();

            if segments.len() >= 2 {
                let prefix = segments[0];
                let suffix = segments[1];

                // Check if both prefix and suffix match parts of the path
                // If prefix is empty, it's a pattern like "/**/suffix"
                let prefix_matches = prefix.is_empty()
                    || path_str.starts_with(prefix)
                    || path_str.contains(&format!("/{}", prefix));

                // If suffix is empty, it's a pattern like "prefix/**/"
                let suffix_matches = suffix.is_empty()
                    || path_str.ends_with(suffix)
                    || path_str.contains(&format!("{}/", suffix));

                if prefix_matches && suffix_matches {
                    debug!("Ignoring {} - matches /**/ pattern: {}", path_str, pattern);
                    return true;
                }
            }
        }

        // Directory pattern (ends with slash)
        if pattern.ends_with('/') {
            let dir_name = &pattern[0..pattern.len() - 1];

            // Check if path contains the directory as a complete segment
            // "test/" should match "test/file.rs" or "src/test/file.rs" but not "testing/file.rs"
            let matches = path_str == dir_name
                || path_str.starts_with(&format!("{}/", dir_name))
                || path_str.contains(&format!("/{}/", dir_name));

            if matches {
                debug!(
                    "Ignoring {} - matches directory pattern: {}",
                    path_str, pattern
                );
                return true;
            }

            continue; // Skip other pattern matching for directory patterns
        }

        // Special case for *.test.* pattern
        if pattern == "*.test.*" {
            if path_str.contains(".test.") {
                debug!("Ignoring {} - matches *.test.* pattern", path_str);
                return true;
            }
        }

        // Handle glob patterns with * (simplified implementation)
        if pattern.contains('*') && !pattern.contains("**") {
            let parts: Vec<&str> = pattern.split('*').collect();

            // Simple cases
            if parts.len() == 2 {
                if pattern.starts_with('*') && path_str.ends_with(parts[1]) {
                    // *suffix pattern
                    debug!(
                        "Ignoring {} - matches *suffix pattern: {}",
                        path_str, pattern
                    );
                    return true;
                } else if pattern.ends_with('*') && path_str.starts_with(parts[0]) {
                    // prefix* pattern
                    debug!(
                        "Ignoring {} - matches prefix* pattern: {}",
                        path_str, pattern
                    );
                    return true;
                } else if path_str.starts_with(parts[0]) && path_str.ends_with(parts[1]) {
                    // prefix*suffix pattern
                    debug!(
                        "Ignoring {} - matches prefix*suffix pattern: {}",
                        path_str, pattern
                    );
                    return true;
                }
            }
        } else {
            // Direct match (either exact or as a substring)
            if path_str == pattern
                || path_str.ends_with(pattern)
                || path_str.contains(&format!("/{}", pattern))
            {
                debug!(
                    "Ignoring {} - matches direct pattern: {}",
                    path_str, pattern
                );
                return true;
            }
        }
    }

    false
}

pub fn collect_relevant_files(
    project_path: &Path,
    ignore_patterns: &HashSet<String>,
    max_files: usize,
    max_file_size: u64,
    is_godot_project: bool,
    respect_gitignore: bool,
) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();

    // Configure the walker with appropriate gitignore settings
    let mut builder = WalkBuilder::new(project_path);
    builder
        .hidden(false) // Include hidden files
        .git_ignore(respect_gitignore) // Respect .gitignore based on CLI option
        .git_global(respect_gitignore) // Also control global gitignore
        .git_exclude(respect_gitignore); // And git exclude rules

    let walker = builder.build();

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
            debug!(
                "Skipping large file: {} ({} bytes)",
                path.display(),
                metadata.len()
            );
            continue;
        }

        // Check if this is a file we want to include
        let extension = path.extension().and_then(|ext| ext.to_str());

        // For Godot projects, we want to prioritize certain file types
        let should_include = if is_godot_project {
            match extension {
                Some("gd") | Some("tscn") | Some("cs") | Some("godot") => true,
                Some("tres") | Some("import") | Some("shader") => true,
                Some(ext) if is_common_code_file(ext) => true,
                _ => false,
            }
        } else {
            // For non-Godot projects, use the regular logic
            match extension {
                Some(ext) if is_common_code_file(ext) => true,
                _ => false,
            }
        };

        if !should_include {
            debug!("Skipping non-code file: {}", path.display());
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

        // Determine file language based on extension and project type
        let language = match extension {
            Some(ext) => {
                let lang = match ext {
                    "rs" => "Rust",
                    "js" => "JavaScript",
                    "ts" => "TypeScript",
                    "py" => "Python",
                    "java" => "Java",
                    "go" => "Go",
                    "c" | "cpp" | "h" | "hpp" => "C/C++",
                    "rb" => "Ruby",
                    "php" => "PHP",
                    "lua" => "Lua",
                    "cs" => {
                        if is_godot_project {
                            "GDScript C#"
                        } else {
                            "C#"
                        }
                    }
                    "html" => "HTML",
                    "css" => "CSS",
                    "json" => "JSON",
                    "md" => "Markdown",
                    "yml" | "yaml" => "YAML",
                    "toml" => "TOML",
                    "gd" => "GDScript",
                    "tscn" | "tres" => "Godot Scene",
                    "shader" => "Godot Shader",
                    _ => "Unknown",
                };
                Some(lang.to_string())
            }
            None => None,
        };

        let relative_path = path
            .strip_prefix(project_path)
            .with_context(|| format!("Failed to strip prefix from {}", path.display()))?
            .to_string_lossy()
            .to_string();

        files.push(FileInfo {
            path: relative_path,
            language,
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

    let mut languages: Vec<(String, usize)> = digest
        .language_breakdown
        .iter()
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
                "Lua" => "lua",
                "C#" => "csharp",
                "GDScript C#" => "csharp",
                "HTML" => "html",
                "CSS" => "css",
                "JSON" => "json",
                "Markdown" => "md",
                "YAML" => "yaml",
                "TOML" => "toml",
                "GDScript" => "gdscript",
                "Godot Scene" => "gdscript",
                "Godot Shader" => "glsl",
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

// Function to detect if a project is a Godot project
pub fn is_godot_project(project_path: &Path) -> bool {
    // Check for project.godot file, which is the main project file for Godot projects
    let project_godot_path = project_path.join("project.godot");
    if project_godot_path.exists() {
        return true;
    }

    // Check for godot/ or .godot/ directories
    let godot_dir = project_path.join("godot");
    let hidden_godot_dir = project_path.join(".godot");
    if godot_dir.exists() || hidden_godot_dir.exists() {
        return true;
    }

    // Look for .tscn or .gd files in the project
    let mut builder = WalkBuilder::new(project_path);
    builder
        .hidden(false)
        .git_ignore(true) // Always respect .gitignore for detection
        .max_depth(Some(3)); // Only check a few levels deep for performance

    let walker = builder.build();

    for result in walker {
        if let Ok(entry) = result {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if ext_str == "tscn" || ext_str == "gd" {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

// Helper function to check if a file extension is a common code file
fn is_common_code_file(ext: &str) -> bool {
    matches!(
        ext,
        "rs" | "js"
            | "ts"
            | "py"
            | "java"
            | "go"
            | "c"
            | "cpp"
            | "h"
            | "hpp"
            | "rb"
            | "php"
            | "cs"
            | "html"
            | "css"
            | "json"
            | "md"
            | "yml"
            | "yaml"
            | "toml"
            | "lua"
            | "gd"
            | "tscn"
            | "tres"
            | "shader"
    )
}

// Function to detect if a project is a Lua project
pub fn is_lua_project(project_path: &Path) -> bool {
    // Common Lua project files
    let lua_files = ["init.lua", "main.lua", "conf.lua", "config.lua"];
    for file in lua_files.iter() {
        if project_path.join(file).exists() {
            return true;
        }
    }

    // Look for a concentration of Lua files in the project
    let mut builder = WalkBuilder::new(project_path);
    builder
        .hidden(false)
        .git_ignore(true) // Always respect .gitignore for detection
        .max_depth(Some(3)); // Only check a few levels deep for performance

    let walker = builder.build();

    let mut lua_file_count = 0;
    for result in walker {
        if let Ok(entry) = result {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if let Some(ext_str) = ext.to_str() {
                        if ext_str == "lua" {
                            lua_file_count += 1;
                            if lua_file_count >= 5 {
                                // If we find at least 5 Lua files, consider it a Lua project
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_should_ignore_patterns() {
        let test_cases = vec![
            // Path, Pattern, Expected Result
            ("src/test/file.rs", "**/test/**", true),
            ("src/main.rs", "**/test/**", false),
            ("README.md", "README.md", true),
            ("docs/README.md", "README.md", true),
            ("src/lib/README.md", "README.md", true),
            ("src/readme.txt", "README.md", false),
            ("test/file.rs", "test/", true),
            ("src/test/file.rs", "test/", true),
            ("testing/file.rs", "test/", false),
            ("src/file.test.rs", "*.test.*", true),
            ("src/file.rs", "*.test.*", false),
        ];

        for (path_str, pattern_str, expected) in test_cases {
            let path = PathBuf::from(path_str);
            let mut patterns = HashSet::new();
            patterns.insert(pattern_str.to_string());

            assert_eq!(
                should_ignore(&path, &patterns),
                expected,
                "Failed for path '{}' with pattern '{}'",
                path_str,
                pattern_str
            );
        }
    }
}
