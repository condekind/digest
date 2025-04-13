use anyhow::Result;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// Re-export the main module functions for testing
use digest::{
    check_for_digestignore, check_for_gitignore, collect_relevant_files, should_ignore, FileInfo,
};

mod pattern_generator;
use pattern_generator::{
    create_test_directory, get_common_test_cases, get_common_test_structure,
    get_complex_test_cases, get_test_file_patterns, run_ignore_pattern_tests,
};

/// Create a directory structure for testing ignore patterns
/// Returns the temporary directory path
fn create_test_directory_structure() -> Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();

    // Create src directory with some files
    fs::create_dir_all(temp_path.join("src"))?;
    fs::create_dir_all(temp_path.join("src/test"))?;
    fs::create_dir_all(temp_path.join("src/lib"))?;
    fs::create_dir_all(temp_path.join("docs"))?;
    fs::create_dir_all(temp_path.join("test"))?;
    fs::create_dir_all(temp_path.join("node_modules"))?;
    fs::create_dir_all(temp_path.join("build"))?;
    fs::create_dir_all(temp_path.join(".git"))?;

    // Create some files
    File::create(temp_path.join("src/main.rs"))?.write_all(b"fn main() {}")?;
    File::create(temp_path.join("src/lib.rs"))?.write_all(b"pub fn lib_fn() {}")?;
    File::create(temp_path.join("src/test/test_utils.rs"))?.write_all(b"pub fn test_util() {}")?;
    File::create(temp_path.join("src/lib/utils.rs"))?.write_all(b"pub fn util() {}")?;
    File::create(temp_path.join("src/lib/README.md"))?.write_all(b"# Lib README")?;
    File::create(temp_path.join("docs/README.md"))?.write_all(b"# Docs README")?;
    File::create(temp_path.join("README.md"))?.write_all(b"# Project README")?;
    File::create(temp_path.join("test/file.rs"))?.write_all(b"#[test] fn test() {}")?;
    File::create(temp_path.join("src/file.test.rs"))?.write_all(b"#[test] fn test_file() {}")?;
    File::create(temp_path.join("src/file.rs"))?.write_all(b"pub fn file() {}")?;
    File::create(temp_path.join("node_modules/package.json"))?.write_all(b"{}")?;
    File::create(temp_path.join("package.json"))?.write_all(b"{}")?;
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

/// Test if a file exists in a collection of FileInfo structs by path
fn file_exists_in_result(files: &[FileInfo], relative_path: &str) -> bool {
    files.iter().any(|f| f.path == relative_path)
}

/// Run a test with specific ignore files and flags
fn run_ignore_test(
    temp_dir: &Path,
    gitignore_patterns: Option<&[&str]>,
    digestignore_patterns: Option<&[&str]>,
    max_files: usize,
    max_file_size: u64,
    respect_gitignore: bool,
) -> Result<Vec<FileInfo>> {
    // Clear any existing ignore files
    let gitignore_path = temp_dir.join(".gitignore");
    let digestignore_path = temp_dir.join(".digestignore");

    if gitignore_path.exists() {
        fs::remove_file(&gitignore_path)?;
    }

    if digestignore_path.exists() {
        fs::remove_file(&digestignore_path)?;
    }

    // Create new ignore files if requested
    if let Some(patterns) = gitignore_patterns {
        create_gitignore(temp_dir, patterns)?;
    }

    if let Some(patterns) = digestignore_patterns {
        create_digestignore(temp_dir, patterns)?;
    }

    // Build the ignore patterns set
    let mut ignore_patterns = HashSet::new();

    // Try to get patterns from .digestignore if it exists
    if digestignore_path.exists() {
        if let Ok(patterns) = check_for_digestignore(temp_dir) {
            ignore_patterns.extend(patterns);
        }
    }

    // Try to get patterns from .gitignore if it exists and respect_gitignore is true
    if gitignore_path.exists() && respect_gitignore {
        if let Ok(patterns) = check_for_gitignore(temp_dir) {
            ignore_patterns.extend(patterns);
        }
    }

    // Collect the files
    collect_relevant_files(
        temp_dir,
        &ignore_patterns,
        max_files,
        max_file_size,
        false, // not a Godot project
        respect_gitignore,
    )
}

#[test]
fn test_gitignore_only() -> Result<()> {
    let temp_dir = create_test_directory_structure()?;
    let gitignore_patterns = &["node_modules/", "build/", "*.test.*"];

    let files = run_ignore_test(
        temp_dir.path(),
        Some(gitignore_patterns),
        None,
        50,
        10000000, // 10MB, larger than any test file
        true,     // respect gitignore
    )?;

    // These should be ignored due to .gitignore
    assert!(!file_exists_in_result(&files, "node_modules/package.json"));
    assert!(!file_exists_in_result(&files, "build/output.js"));
    assert!(!file_exists_in_result(&files, "src/file.test.rs"));

    // These should be included
    assert!(file_exists_in_result(&files, "src/main.rs"));
    assert!(file_exists_in_result(&files, "src/lib.rs"));
    assert!(file_exists_in_result(&files, "README.md"));

    Ok(())
}

#[test]
fn test_digestignore_only() -> Result<()> {
    let temp_dir = create_test_directory_structure()?;
    let digestignore_patterns = &["docs/", "test/", "README.md"];

    let files = run_ignore_test(
        temp_dir.path(),
        None,
        Some(digestignore_patterns),
        50,
        10000000,
        false, // don't respect gitignore
    )?;

    // These should be ignored due to .digestignore
    assert!(!file_exists_in_result(&files, "docs/README.md"));
    assert!(!file_exists_in_result(&files, "test/file.rs"));
    assert!(!file_exists_in_result(&files, "README.md"));

    // These should be included
    assert!(file_exists_in_result(&files, "src/main.rs"));
    assert!(file_exists_in_result(&files, "src/lib.rs"));
    assert!(file_exists_in_result(&files, "node_modules/package.json")); // included because we're not respecting gitignore

    Ok(())
}

#[test]
fn test_both_ignore_files() -> Result<()> {
    let temp_dir = create_test_directory_structure()?;
    let gitignore_patterns = &["node_modules/", "build/"];

    let digestignore_patterns = &["docs/", "test/"];

    let files = run_ignore_test(
        temp_dir.path(),
        Some(gitignore_patterns),
        Some(digestignore_patterns),
        50,
        10000000,
        true, // respect gitignore
    )?;

    // These should be ignored due to .gitignore
    assert!(!file_exists_in_result(&files, "node_modules/package.json"));
    assert!(!file_exists_in_result(&files, "build/output.js"));

    // These should be ignored due to .digestignore
    assert!(!file_exists_in_result(&files, "docs/README.md"));
    assert!(!file_exists_in_result(&files, "test/file.rs"));

    // These should be included
    assert!(file_exists_in_result(&files, "src/main.rs"));
    assert!(file_exists_in_result(&files, "src/lib.rs"));
    assert!(file_exists_in_result(&files, "README.md"));

    Ok(())
}

#[test]
fn test_no_respect_gitignore() -> Result<()> {
    let temp_dir = create_test_directory_structure()?;
    let gitignore_patterns = &["node_modules/", "build/"];

    let digestignore_patterns = &["docs/", "test/"];

    let files = run_ignore_test(
        temp_dir.path(),
        Some(gitignore_patterns),
        Some(digestignore_patterns),
        50,
        10000000,
        false, // don't respect gitignore
    )?;

    // These should NOT be ignored (gitignore not respected)
    assert!(file_exists_in_result(&files, "node_modules/package.json"));
    assert!(file_exists_in_result(&files, "build/output.js"));

    // These should still be ignored (digestignore is respected)
    assert!(!file_exists_in_result(&files, "docs/README.md"));
    assert!(!file_exists_in_result(&files, "test/file.rs"));

    // These should be included
    assert!(file_exists_in_result(&files, "src/main.rs"));
    assert!(file_exists_in_result(&files, "src/lib.rs"));
    assert!(file_exists_in_result(&files, "README.md"));

    Ok(())
}

#[test]
fn test_overlapping_patterns() -> Result<()> {
    let temp_dir = create_test_directory_structure()?;
    let gitignore_patterns = &["src/test/", "README.md"];

    let digestignore_patterns = &[
        "src/lib/",
        "README.md", // Duplicate pattern
    ];

    let files = run_ignore_test(
        temp_dir.path(),
        Some(gitignore_patterns),
        Some(digestignore_patterns),
        50,
        10000000,
        true, // respect gitignore
    )?;

    // These should be ignored
    assert!(!file_exists_in_result(&files, "src/test/test_utils.rs"));
    assert!(!file_exists_in_result(&files, "src/lib/utils.rs"));
    assert!(!file_exists_in_result(&files, "README.md"));

    // These should be included
    assert!(file_exists_in_result(&files, "src/main.rs"));
    assert!(file_exists_in_result(&files, "src/lib.rs"));

    Ok(())
}

#[test]
fn test_complex_pattern_combinations() -> Result<()> {
    let temp_dir = create_test_directory_structure()?;
    let gitignore_patterns = &["**/test/**", "build/"];

    let digestignore_patterns = &["**/*.md", "node_modules/"];

    let files = run_ignore_test(
        temp_dir.path(),
        Some(gitignore_patterns),
        Some(digestignore_patterns),
        50,
        10000000,
        true, // respect gitignore
    )?;

    // These should be ignored due to .gitignore
    assert!(!file_exists_in_result(&files, "src/test/test_utils.rs"));
    assert!(!file_exists_in_result(&files, "test/file.rs"));
    assert!(!file_exists_in_result(&files, "build/output.js"));

    // These should be ignored due to .digestignore
    assert!(!file_exists_in_result(&files, "docs/README.md"));
    assert!(!file_exists_in_result(&files, "README.md"));
    assert!(!file_exists_in_result(&files, "src/lib/README.md"));
    assert!(!file_exists_in_result(&files, "node_modules/package.json"));

    // These should be included
    assert!(file_exists_in_result(&files, "src/main.rs"));
    assert!(file_exists_in_result(&files, "src/lib.rs"));
    assert!(file_exists_in_result(&files, "src/lib/utils.rs"));
    assert!(file_exists_in_result(&files, "src/file.rs"));

    Ok(())
}

#[test]
fn test_file_size_limit() -> Result<()> {
    let temp_dir = create_test_directory_structure()?;

    // Create a large file that exceeds the limit
    let large_file_path = temp_dir.path().join("large_file.rs");
    let mut large_file = File::create(&large_file_path)?;

    // Create a 10KB file
    let large_content = "// Large file content\n".repeat(500);
    large_file.write_all(large_content.as_bytes())?;

    let files = run_ignore_test(
        temp_dir.path(),
        None,
        None,
        50,
        5 * 1024, // 5KB limit, smaller than the large file
        false,
    )?;

    // The large file should be ignored due to size
    assert!(!file_exists_in_result(&files, "large_file.rs"));

    // Other files should be included
    assert!(file_exists_in_result(&files, "src/main.rs"));

    Ok(())
}

#[test]
fn test_max_files_limit() -> Result<()> {
    let temp_dir = create_test_directory_structure()?;

    // Create many additional files
    for i in 0..20 {
        let file_path = temp_dir.path().join(format!("extra_file_{}.rs", i));
        File::create(file_path)?.write_all(format!("// File {}", i).as_bytes())?;
    }

    let files = run_ignore_test(
        temp_dir.path(),
        None,
        None,
        5, // Only include 5 files maximum
        10000000,
        false,
    )?;

    // Check that only 5 files were included
    assert_eq!(files.len(), 5);

    Ok(())
}

// Integration test that creates a directory structure programmatically
// Based on the project path pattern provided
#[test]
fn test_directory_structure_generator() -> Result<()> {
    // Setup a temp directory
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    // Define our test structure
    let structure = [
        ("src/main.rs", "fn main() {}"),
        ("src/lib.rs", "pub fn lib() {}"),
        ("src/utils/helpers.rs", "pub fn helper() {}"),
        ("src/tests/test_main.rs", "#[test] fn test() {}"),
        ("tests/integration/mod.rs", "pub mod tests;"),
        ("docs/README.md", "# Documentation"),
        ("build/output.js", "// Built output"),
        ("node_modules/package/index.js", "module.exports = {}"),
    ];

    // Create the files
    for (path, content) in &structure {
        let full_path = root.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full_path, content)?;
    }

    // Test with different gitignore patterns
    let test_cases = [
        // (gitignore_patterns, digestignore_patterns, expected_included, expected_excluded)
        (
            vec!["node_modules/", "build/"],
            vec!["docs/"],
            vec![
                "src/main.rs",
                "src/lib.rs",
                "src/utils/helpers.rs",
                "src/tests/test_main.rs",
                "tests/integration/mod.rs",
            ],
            vec![
                "docs/README.md",
                "build/output.js",
                "node_modules/package/index.js",
            ],
        ),
        (
            vec!["**/test*/**"],
            vec!["**/*.md"],
            vec![
                "src/main.rs",
                "src/lib.rs",
                "src/utils/helpers.rs",
                "build/output.js",
                "node_modules/package/index.js",
            ],
            vec![
                "src/tests/test_main.rs",
                "tests/integration/mod.rs",
                "docs/README.md",
            ],
        ),
        (
            vec![],
            vec!["**/*.js"],
            vec![
                "src/main.rs",
                "src/lib.rs",
                "src/utils/helpers.rs",
                "src/tests/test_main.rs",
                "tests/integration/mod.rs",
                "docs/README.md",
            ],
            vec!["build/output.js", "node_modules/package/index.js"],
        ),
    ];

    for (i, (gitignore, digestignore, expected_included, expected_excluded)) in
        test_cases.iter().enumerate()
    {
        println!("\n--- Running test case {} ---", i);

        // Set up the ignore files
        create_gitignore(root, &gitignore.iter().map(|s| *s).collect::<Vec<_>>())?;
        create_digestignore(root, &digestignore.iter().map(|s| *s).collect::<Vec<_>>())?;

        println!("gitignore patterns: {:?}", gitignore);
        println!("digestignore patterns: {:?}", digestignore);

        // Build the ignore patterns set
        let mut ignore_patterns = HashSet::new();

        // Add patterns from both files
        if let Ok(digestignore_patterns) = check_for_digestignore(root) {
            println!(
                "Loaded patterns from digestignore: {:?}",
                digestignore_patterns
            );
            ignore_patterns.extend(digestignore_patterns);
        }

        if let Ok(gitignore_patterns) = check_for_gitignore(root) {
            println!("Loaded patterns from gitignore: {:?}", gitignore_patterns);
            ignore_patterns.extend(gitignore_patterns);
        }

        println!("Combined patterns: {:?}", ignore_patterns);

        // Check each expected included file
        for path in expected_included {
            let full_path = root.join(path);
            let is_ignored = should_ignore(&full_path, &ignore_patterns);
            println!(
                "Testing path: {} - should NOT be ignored, actual: {}",
                path, is_ignored
            );

            assert!(
                !is_ignored,
                "Test case {}: Expected {} to be included but it was ignored",
                i, path
            );
        }

        // Check each expected excluded file
        for path in expected_excluded {
            let full_path = root.join(path);
            let is_ignored = should_ignore(&full_path, &ignore_patterns);
            println!(
                "Testing path: {} - should be ignored, actual: {}",
                path, is_ignored
            );

            assert!(
                is_ignored,
                "Test case {}: Expected {} to be excluded but it was included",
                i, path
            );
        }
    }

    Ok(())
}

#[test]
fn test_programmatic_pattern_tests() -> Result<()> {
    // Get the common test structure
    let structure = get_common_test_structure();

    // Run tests with common test cases
    run_ignore_pattern_tests(&structure, &get_common_test_cases())?;

    // Run tests with complex test cases
    run_ignore_pattern_tests(&structure, &get_complex_test_cases())?;

    // Run tests with test file patterns
    run_ignore_pattern_tests(&structure, &get_test_file_patterns())?;

    Ok(())
}
