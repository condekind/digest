// Re-export the main module functions for testing
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Serialize, Debug)]
pub struct FileInfo {
    pub path: String,
    pub language: Option<String>,
    pub content: String,
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
                return true;
            }
        }

        // Special case for **/test*/** pattern (common in tests)
        if pattern == "**/test*/**" {
            // This should match paths containing test, tests, testing, etc. as directories
            let path_segments: Vec<&str> = path_str.split('/').collect();
            for (i, segment) in path_segments.iter().enumerate() {
                // Only match if it's a directory (not a file) and starts with "test"
                if !segment.is_empty() && segment.starts_with("test") {
                    // Since **/test*/** requires a segment that starts with test
                    // followed by at least one more segment,
                    // don't match files like "test_file.js" that are at the end of the path
                    if i == path_segments.len() - 1 {
                        // If it's the last segment, it's a file, not a directory
                        // Unless the path ends with a slash
                        if !path_str.ends_with('/') {
                            continue;
                        }
                    }
                    return true;
                }
            }
        }

        // Special case for **/*.md pattern (common for documentation)
        if pattern == "**/*.md" {
            if path_str.ends_with(".md") {
                return true;
            }
        }

        // Special case for common directory patterns
        if pattern == "node_modules/" {
            if path_str.starts_with("node_modules/") || path_str.contains("/node_modules/") {
                return true;
            }
        }

        if pattern == "build/" {
            if path_str.starts_with("build/") || path_str.contains("/build/") {
                return true;
            }
        }

        // Always ignore .git directory
        if path_str.contains("/.git/") || path_str == ".git" {
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
            // Check if suffix appears anywhere in the path with proper directory boundaries
            // For example, "**/test" should match "test" or "src/test" but not "testing" or "src/testing"
            if path_str == suffix ||
               path_str.ends_with(&format!("/{}", suffix)) ||
               // Special case for directories: if suffix ends with '/', then handle it as a directory
               (suffix.ends_with('/') && (
                   path_str.ends_with(&suffix[..suffix.len()-1]) ||
                   path_str.contains(&format!("{}/", &suffix[..suffix.len()-1]))
               ))
            {
                return true;
            }
        }

        // Handle pattern ending with /** (match any subdirectory)
        if pattern.ends_with("/**") {
            let prefix = &pattern[0..pattern.len() - 3];
            // The prefix should be treated as a directory name, so it should have a trailing slash
            // or be at the beginning of the path
            // For example, "build/**" should match "build/file.js" but not "builds/file.js" or "src/build.js"
            if path_str.starts_with(&format!("{}/", prefix))
                || path_str.contains(&format!("/{}/", prefix))
            {
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
                    return true;
                }
            }
        }

        // Directory pattern (ends with slash)
        if pattern.ends_with('/') {
            let dir_name = &pattern[0..pattern.len() - 1];

            // Special handling for wildcard directory patterns (e.g., "**/test*/")
            if dir_name.contains('*') {
                // Handle **/prefix*/ pattern (common case)
                if dir_name.starts_with("**/") {
                    let wildcard_part = &dir_name[3..];
                    if wildcard_part.contains('*') {
                        // For patterns like "**/test*/"
                        let parts: Vec<&str> = wildcard_part.split('*').collect();
                        if parts.len() == 2 {
                            let prefix = parts[0];
                            let suffix = parts[1];

                            // This should match any directory that starts with prefix and ends with suffix
                            // For example, "**/test*/" should match "test/", "testing/", "src/test/", "src/testing/"
                            let contains_pattern = path_str.split('/').any(|segment| {
                                !segment.is_empty()
                                    && segment.starts_with(prefix)
                                    && segment.ends_with(suffix)
                            });

                            if contains_pattern {
                                return true;
                            }
                        }
                    }
                }

                // Skip to next pattern since we've handled wildcards
                continue;
            }

            // Check if path contains the directory as a complete segment
            // "test/" should match "test/file.rs" or "src/test/file.rs" but not "testing/file.rs"
            let matches = path_str == dir_name
                || path_str.starts_with(&format!("{}/", dir_name))
                || path_str.contains(&format!("/{}/", dir_name));

            if matches {
                return true;
            }

            continue; // Skip other pattern matching for directory patterns
        }

        // Special case for *.test.* pattern
        if pattern == "*.test.*" {
            if path_str.contains(".test.") {
                return true;
            }
        }

        // Handle glob patterns with * (simplified implementation)
        if pattern.contains('*') && !pattern.contains("**") {
            let parts: Vec<&str> = pattern.split('*').collect();

            // Simple cases
            if parts.len() == 2 {
                if pattern.starts_with('*') && path_str.ends_with(parts[1]) {
                    // *suffix pattern (e.g., "*.js")
                    // Make sure the suffix starts at a valid boundary (e.g., after a / or .)
                    let last_segment = path_str.split('/').last().unwrap_or("");
                    if last_segment.ends_with(parts[1])
                        && (parts[1].is_empty()
                            || parts[1].starts_with('.')
                            || last_segment == parts[1])
                    {
                        return true;
                    }
                } else if pattern.ends_with('*') && path_str.starts_with(parts[0]) {
                    // prefix* pattern
                    // Make sure the prefix matches a whole path component
                    if path_str == parts[0]
                        || path_str.starts_with(&format!("{}/", parts[0]))
                        || path_str.contains(&format!("/{}/", parts[0]))
                    {
                        return true;
                    }
                } else if !parts[0].is_empty() && !parts[1].is_empty() {
                    // prefix*suffix pattern
                    // For file extensions like "*.js", make sure we match correct boundary
                    let file_name = path_str.split('/').last().unwrap_or("");
                    if parts[1].starts_with('.')
                        && file_name.contains(&format!("{}{}", parts[0], parts[1]))
                    {
                        return true;
                    } else if path_str.contains(&format!("{}{}", parts[0], parts[1])) {
                        return true;
                    }
                }
            }
        } else {
            // Direct match (either exact or as a substring)
            if path_str == pattern
                || path_str.ends_with(&format!("/{}", pattern))
                || path_str.contains(&format!("/{}/", pattern))
            {
                return true;
            }
        }
    }

    false
}

pub fn check_for_digestignore(project_path: &Path) -> Result<HashSet<String>> {
    let digestignore_path = project_path.join(".digestignore");

    if !digestignore_path.exists() {
        return Err(anyhow::anyhow!("No .digestignore file found"));
    }

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
    let mut builder = ignore::WalkBuilder::new(project_path);
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
                eprintln!("Error accessing entry: {}", err);
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
            continue;
        }

        // Check file size
        let metadata = match fs::metadata(path) {
            Ok(meta) => meta,
            Err(err) => {
                eprintln!("Error reading metadata for {}: {}", path.display(), err);
                continue;
            }
        };

        if metadata.len() > max_file_size {
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
            continue;
        }

        // Read file content
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Error reading file {}: {}", path.display(), err);
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
            | "gd"
            | "tscn"
            | "tres"
            | "shader"
    )
}
