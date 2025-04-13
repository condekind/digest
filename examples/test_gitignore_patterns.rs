use anyhow::Result;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

use digest::{check_for_digestignore, check_for_gitignore, should_ignore};

/// Create a temporary test directory with some files
fn create_test_directory() -> Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    // Create some directories
    fs::create_dir_all(temp_path.join("src"))?;
    fs::create_dir_all(temp_path.join("src/test"))?;
    fs::create_dir_all(temp_path.join("node_modules"))?;
    fs::create_dir_all(temp_path.join("build"))?;

    // Create some files
    File::create(temp_path.join("src/main.rs"))?.write_all(b"fn main() {}")?;
    File::create(temp_path.join("src/test/test_utils.rs"))?.write_all(b"pub fn test_util() {}")?;
    File::create(temp_path.join("README.md"))?.write_all(b"# Project README")?;
    File::create(temp_path.join("src/file.test.rs"))?.write_all(b"#[test] fn test_file() {}")?;
    File::create(temp_path.join("node_modules/package.json"))?.write_all(b"{}")?;
    File::create(temp_path.join("build/output.js"))?.write_all(b"// Build output")?;

    Ok(temp_dir)
}

/// Create a .gitignore file with the given patterns
fn create_gitignore(root_path: &Path, patterns: &[&str]) -> Result<()> {
    let gitignore_content = patterns.join("\n");
    fs::write(root_path.join(".gitignore"), gitignore_content)?;
    Ok(())
}

/// Create a .digestignore file with the given patterns
fn create_digestignore(root_path: &Path, patterns: &[&str]) -> Result<()> {
    let digestignore_content = patterns.join("\n");
    fs::write(root_path.join(".digestignore"), digestignore_content)?;
    Ok(())
}

fn test_gitignore_patterns() -> Result<()> {
    let temp_dir = create_test_directory()?;
    let gitignore_patterns = &["node_modules/", "build/", "*.test.*"];

    create_gitignore(temp_dir.path(), gitignore_patterns)?;

    // Load the gitignore patterns
    let patterns = check_for_gitignore(temp_dir.path())?;

    // Test paths that should be ignored
    let should_be_ignored = vec![
        temp_dir.path().join("node_modules/package.json"),
        temp_dir.path().join("build/output.js"),
        temp_dir.path().join("src/file.test.rs"),
    ];

    for path in should_be_ignored {
        if !should_ignore(&path, &patterns) {
            println!("ERROR: Expected {:?} to be ignored, but it wasn't", path);
        } else {
            println!("OK: {:?} was ignored as expected", path);
        }
    }

    // Test paths that should NOT be ignored
    let should_not_be_ignored = vec![
        temp_dir.path().join("src/main.rs"),
        temp_dir.path().join("README.md"),
    ];

    for path in should_not_be_ignored {
        if should_ignore(&path, &patterns) {
            println!("ERROR: Expected {:?} NOT to be ignored, but it was", path);
        } else {
            println!("OK: {:?} was not ignored as expected", path);
        }
    }

    Ok(())
}

fn test_both_ignore_files() -> Result<()> {
    let temp_dir = create_test_directory()?;
    let gitignore_patterns = &["node_modules/", "build/"];

    let digestignore_patterns = &["*.md", "*.test.*"];

    create_gitignore(temp_dir.path(), gitignore_patterns)?;
    create_digestignore(temp_dir.path(), digestignore_patterns)?;

    // Load both ignore patterns
    let mut ignore_patterns = HashSet::new();

    if let Ok(git_patterns) = check_for_gitignore(temp_dir.path()) {
        ignore_patterns.extend(git_patterns);
    }

    if let Ok(digest_patterns) = check_for_digestignore(temp_dir.path()) {
        ignore_patterns.extend(digest_patterns);
    }

    // Test paths that should be ignored
    let should_be_ignored = vec![
        temp_dir.path().join("node_modules/package.json"), // From gitignore
        temp_dir.path().join("build/output.js"),           // From gitignore
        temp_dir.path().join("README.md"),                 // From digestignore
        temp_dir.path().join("src/file.test.rs"),          // From digestignore
    ];

    for path in should_be_ignored {
        if !should_ignore(&path, &ignore_patterns) {
            println!("ERROR: Expected {:?} to be ignored, but it wasn't", path);
        } else {
            println!("OK: {:?} was ignored as expected", path);
        }
    }

    // Test paths that should NOT be ignored
    let should_not_be_ignored = vec![
        temp_dir.path().join("src/main.rs"),
        temp_dir.path().join("src/test/test_utils.rs"),
    ];

    for path in should_not_be_ignored {
        if should_ignore(&path, &ignore_patterns) {
            println!("ERROR: Expected {:?} NOT to be ignored, but it was", path);
        } else {
            println!("OK: {:?} was not ignored as expected", path);
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    println!("Testing gitignore patterns...");
    test_gitignore_patterns()?;

    println!("\nTesting both gitignore and digestignore...");
    test_both_ignore_files()?;

    Ok(())
}
