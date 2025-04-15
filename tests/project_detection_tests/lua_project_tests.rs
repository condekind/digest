use ignore::WalkBuilder;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

// Test implementation of is_lua_project that matches the one in main.rs
pub fn is_lua_project_impl(project_path: &Path) -> bool {
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

/// Create a temporary directory that looks like a Lua project
fn create_lua_project() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create some Lua files
    fs::create_dir_all(temp_path.join("src")).unwrap();
    fs::create_dir_all(temp_path.join("lib")).unwrap();
    fs::create_dir_all(temp_path.join("test")).unwrap();

    // Create some Lua files with common Lua project structure
    File::create(temp_path.join("init.lua"))
        .unwrap()
        .write_all(b"-- Lua init file")
        .unwrap();
    File::create(temp_path.join("src/main.lua"))
        .unwrap()
        .write_all(b"-- Lua main file")
        .unwrap();
    File::create(temp_path.join("lib/utils.lua"))
        .unwrap()
        .write_all(b"-- Lua utilities")
        .unwrap();
    File::create(temp_path.join("test/test_utils.lua"))
        .unwrap()
        .write_all(b"-- Lua test utilities")
        .unwrap();

    // Create some non-Lua files too
    File::create(temp_path.join("README.md"))
        .unwrap()
        .write_all(b"# Lua Project")
        .unwrap();
    File::create(temp_path.join("config.json"))
        .unwrap()
        .write_all(b"{\"name\": \"lua-project\"}")
        .unwrap();

    temp_dir
}

/// Create a temporary directory that has Lua files but is not a Lua project
fn create_non_lua_project_with_lua_files() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create directory structure
    fs::create_dir_all(temp_path.join("docs")).unwrap();
    fs::create_dir_all(temp_path.join("scripts")).unwrap();

    // Add some Lua files, but fewer than would trigger detection
    File::create(temp_path.join("scripts/helper.lua"))
        .unwrap()
        .write_all(b"-- A helper script")
        .unwrap();
    File::create(temp_path.join("scripts/config.lua"))
        .unwrap()
        .write_all(b"-- A config script")
        .unwrap();

    // Add non-Lua files that would make it a different type of project
    File::create(temp_path.join("package.json"))
        .unwrap()
        .write_all(b"{\"name\": \"js-project\"}")
        .unwrap();
    File::create(temp_path.join("index.js"))
        .unwrap()
        .write_all(b"console.log('Hello');")
        .unwrap();
    File::create(temp_path.join("README.md"))
        .unwrap()
        .write_all(b"# JS Project with Lua scripts")
        .unwrap();

    temp_dir
}

#[test]
fn test_lua_project_detection_with_init_lua() {
    // Test a project with init.lua (should be detected as Lua)
    let lua_project = create_lua_project();
    assert!(
        is_lua_project_impl(lua_project.path()),
        "Should detect Lua project with init.lua"
    );
}

#[test]
fn test_non_lua_project_with_few_lua_files() {
    // Test a project with Lua files but not enough to be a Lua project
    let non_lua_project = create_non_lua_project_with_lua_files();
    assert!(
        !is_lua_project_impl(non_lua_project.path()),
        "Should not detect as Lua project when there are few Lua files"
    );
}

#[test]
fn test_lua_project_with_many_files() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create directory structure
    fs::create_dir_all(temp_path.join("src")).unwrap();
    fs::create_dir_all(temp_path.join("libs")).unwrap();
    fs::create_dir_all(temp_path.join("scripts")).unwrap();

    // Create many Lua files (more than 5 to trigger detection)
    File::create(temp_path.join("src/main.lua"))
        .unwrap()
        .write_all(b"-- Main file")
        .unwrap();
    File::create(temp_path.join("src/utils.lua"))
        .unwrap()
        .write_all(b"-- Utils file")
        .unwrap();
    File::create(temp_path.join("libs/helpers.lua"))
        .unwrap()
        .write_all(b"-- Helper functions")
        .unwrap();
    File::create(temp_path.join("libs/config.lua"))
        .unwrap()
        .write_all(b"-- Config loader")
        .unwrap();
    File::create(temp_path.join("scripts/runner.lua"))
        .unwrap()
        .write_all(b"-- Script runner")
        .unwrap();
    File::create(temp_path.join("scripts/compiler.lua"))
        .unwrap()
        .write_all(b"-- Script compiler")
        .unwrap();

    // No init.lua or other main Lua file, but should still be detected due to number of .lua files
    assert!(
        is_lua_project_impl(temp_path),
        "Should detect as Lua project when there are many Lua files"
    );
}

// Test that we recognizes and correctly handles Lua files (implementation of is_common_code_file)
#[test]
fn test_lua_file_extension_recognition() {
    assert!(
        is_common_code_file("lua"),
        ".lua files should be recognized as code files"
    );
    assert!(
        is_common_code_file("rs"),
        "Other code files like .rs should still be recognized"
    );
    assert!(
        !is_common_code_file("luac"),
        ".luac (compiled Lua) should not be treated as code files"
    );
}

#[test]
fn test_lua_language_detection() {
    // Test simple implementation of language detection similar to what's in the codebase
    let lua_file_ext = "lua";
    let lua_language = get_language_from_extension(lua_file_ext, false);

    assert_eq!(
        lua_language,
        Some("Lua".to_string()),
        "Should detect .lua files as Lua language"
    );

    // Test other languages for comparison
    let rs_language = get_language_from_extension("rs", false);
    assert_eq!(
        rs_language,
        Some("Rust".to_string()),
        "Should correctly detect other languages too"
    );
}

// Simple implementation of language detection for testing
fn get_language_from_extension(ext: &str, is_godot_project: bool) -> Option<String> {
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

// Simple function to create Lua-specific ignore patterns for testing
fn build_test_ignore_patterns(main_language: &Option<String>) -> HashSet<String> {
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
                patterns.insert("bin".to_string());
                patterns.insert("obj".to_string());
                patterns.insert("*.dll".to_string());
            }
            _ => {}
        }
    }

    patterns
}

#[test]
fn test_lua_ignore_patterns() {
    // Test that Lua-specific ignore patterns are added for Lua projects
    let main_language = Some("Lua".to_string());

    let patterns = build_test_ignore_patterns(&main_language);

    // Check that Lua-specific patterns are included
    assert!(
        patterns.contains("*.luac"),
        "Should ignore compiled Lua files (*.luac)"
    );
    assert!(
        patterns.contains("luarocks"),
        "Should ignore luarocks directory"
    );

    // Check that common patterns are still included
    assert!(
        patterns.contains(".git"),
        "Should include common patterns like .git"
    );

    // Make sure we don't have patterns specific to other languages
    assert!(
        !patterns.contains("*.pyc"),
        "Should not include Python-specific patterns"
    );
}

// Simple implementation of is_common_code_file for testing
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

#[test]
fn test_lua_markdown_formatting() {
    // Test that Lua language is correctly formatted in markdown output
    let lua_language = "Lua";
    let markdown_tag = get_markdown_language_tag(lua_language);

    assert_eq!(
        markdown_tag, "lua",
        "Should format Lua as 'lua' in markdown code blocks"
    );

    // Test other languages for comparison
    let rust_tag = get_markdown_language_tag("Rust");
    assert_eq!(rust_tag, "rust", "Should format Rust correctly in markdown");
}

// Simple function to determine the markdown code block language tag
fn get_markdown_language_tag(language: &str) -> &str {
    match language {
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
    }
}
