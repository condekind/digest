use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

use digest::{check_for_digestignore, check_for_gitignore, should_ignore};

/// A structure representing an ignore pattern test case
pub struct IgnorePatternTestCase {
    /// Patterns to put in .gitignore
    pub gitignore_patterns: Vec<String>,
    /// Patterns to put in .digestignore
    pub digestignore_patterns: Vec<String>,
    /// Files that should be included (not ignored)
    pub expected_included: Vec<String>,
    /// Files that should be excluded (ignored)
    pub expected_excluded: Vec<String>,
    /// A description of what this test is checking
    pub description: String,
}

/// A directory structure for testing
pub struct TestDirectoryStructure {
    /// The files to create in the test directory
    /// Each entry is (relative_path, content)
    pub files: Vec<(String, String)>,
    /// A description of the directory structure
    pub description: String,
}

/// Create a test directory with the specified structure
pub fn create_test_directory(structure: &TestDirectoryStructure) -> Result<TempDir> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    // Create the files
    for (path, content) in &structure.files {
        let full_path = root.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full_path, content)?;
    }

    Ok(temp_dir)
}

/// Create a .gitignore file with the given patterns
pub fn create_gitignore(root_path: &Path, patterns: &[String]) -> Result<()> {
    let gitignore_content = patterns.join("\n");
    fs::write(root_path.join(".gitignore"), gitignore_content)?;
    Ok(())
}

/// Create a .digestignore file with the given patterns
pub fn create_digestignore(root_path: &Path, patterns: &[String]) -> Result<()> {
    let digestignore_content = patterns.join("\n");
    fs::write(root_path.join(".digestignore"), digestignore_content)?;
    Ok(())
}

/// Run all the specified tests against a directory structure
pub fn run_ignore_pattern_tests(
    structure: &TestDirectoryStructure,
    test_cases: &[IgnorePatternTestCase],
) -> Result<()> {
    let temp_dir = create_test_directory(structure)?;
    let root = temp_dir.path();

    println!("Testing directory structure: {}", structure.description);

    for (i, test) in test_cases.iter().enumerate() {
        println!("Running test case {}: {}", i, test.description);

        // Clear any existing ignore files
        let gitignore_path = root.join(".gitignore");
        let digestignore_path = root.join(".digestignore");

        if gitignore_path.exists() {
            fs::remove_file(&gitignore_path)?;
        }

        if digestignore_path.exists() {
            fs::remove_file(&digestignore_path)?;
        }

        // Create new ignore files if needed
        if !test.gitignore_patterns.is_empty() {
            create_gitignore(root, &test.gitignore_patterns)?;
        }

        if !test.digestignore_patterns.is_empty() {
            create_digestignore(root, &test.digestignore_patterns)?;
        }

        // Build the ignore patterns set
        let mut ignore_patterns = HashSet::new();

        // Add patterns from both files
        if let Ok(patterns) = check_for_digestignore(root) {
            ignore_patterns.extend(patterns);
        }

        if let Ok(patterns) = check_for_gitignore(root) {
            ignore_patterns.extend(patterns);
        }

        // Check each expected included file
        for path in &test.expected_included {
            let full_path = root.join(path);
            assert!(
                !should_ignore(&full_path, &ignore_patterns),
                "Test case {}: Expected {} to be included but it was ignored",
                i,
                path
            );
        }

        // Check each expected excluded file
        for path in &test.expected_excluded {
            let full_path = root.join(path);
            assert!(
                should_ignore(&full_path, &ignore_patterns),
                "Test case {}: Expected {} to be excluded but it was included",
                i,
                path
            );
        }
    }

    Ok(())
}

/// Generate a common test directory structure
pub fn get_common_test_structure() -> TestDirectoryStructure {
    TestDirectoryStructure {
        files: vec![
            // Source code files
            ("src/main.rs".to_string(), "fn main() {}".to_string()),
            ("src/lib.rs".to_string(), "pub fn lib() {}".to_string()),
            (
                "src/utils/helpers.rs".to_string(),
                "pub fn helper() {}".to_string(),
            ),
            // Test files
            (
                "src/tests/test_main.rs".to_string(),
                "#[test] fn test() {}".to_string(),
            ),
            (
                "tests/integration/mod.rs".to_string(),
                "pub mod tests;".to_string(),
            ),
            (
                "tests/unit/test_utils.rs".to_string(),
                "#[test] fn test_utils() {}".to_string(),
            ),
            // Documentation
            ("docs/README.md".to_string(), "# Documentation".to_string()),
            (
                "src/lib/README.md".to_string(),
                "# Library README".to_string(),
            ),
            ("README.md".to_string(), "# Project README".to_string()),
            // Build artifacts
            ("build/output.js".to_string(), "// Built output".to_string()),
            ("dist/app.js".to_string(), "// Distributed app".to_string()),
            // Dependencies
            (
                "node_modules/package/index.js".to_string(),
                "module.exports = {}".to_string(),
            ),
            // Data files
            ("data/sample.json".to_string(), "{}".to_string()),
            ("src/data/config.json".to_string(), "{}".to_string()),
            // Hidden files and directories
            (".git/HEAD".to_string(), "ref: refs/heads/main".to_string()),
            (".vscode/settings.json".to_string(), "{}".to_string()),
        ],
        description: "Common project structure with source, tests, docs, build artifacts"
            .to_string(),
    }
}

/// Generate a set of common test cases for ignore patterns
pub fn get_common_test_cases() -> Vec<IgnorePatternTestCase> {
    vec![
        // Simple directory patterns
        IgnorePatternTestCase {
            gitignore_patterns: vec!["node_modules/".to_string(), "build/".to_string()],
            digestignore_patterns: vec!["docs/".to_string()],
            expected_included: vec![
                "src/main.rs".to_string(),
                "src/lib.rs".to_string(),
                "src/utils/helpers.rs".to_string(),
                "src/tests/test_main.rs".to_string(),
                "tests/integration/mod.rs".to_string(),
                "tests/unit/test_utils.rs".to_string(),
                "README.md".to_string(),
                "src/lib/README.md".to_string(),
                "dist/app.js".to_string(),
                "data/sample.json".to_string(),
                "src/data/config.json".to_string(),
            ],
            expected_excluded: vec![
                "docs/README.md".to_string(),
                "build/output.js".to_string(),
                "node_modules/package/index.js".to_string(),
                ".git/HEAD".to_string(), // .git is always ignored
            ],
            description: "Basic directory ignore patterns".to_string(),
        },
        // Glob patterns
        IgnorePatternTestCase {
            gitignore_patterns: vec!["**/test*/**".to_string()],
            digestignore_patterns: vec!["**/*.md".to_string()],
            expected_included: vec![
                "src/main.rs".to_string(),
                "src/lib.rs".to_string(),
                "src/utils/helpers.rs".to_string(),
                "build/output.js".to_string(),
                "dist/app.js".to_string(),
                "node_modules/package/index.js".to_string(),
                "data/sample.json".to_string(),
                "src/data/config.json".to_string(),
            ],
            expected_excluded: vec![
                "src/tests/test_main.rs".to_string(),
                "tests/integration/mod.rs".to_string(),
                "tests/unit/test_utils.rs".to_string(),
                "docs/README.md".to_string(),
                "README.md".to_string(),
                "src/lib/README.md".to_string(),
                ".git/HEAD".to_string(),
            ],
            description: "Glob patterns (**/test*/** and **/*.md)".to_string(),
        },
        // File extension patterns
        IgnorePatternTestCase {
            gitignore_patterns: vec!["*.js".to_string()],
            digestignore_patterns: vec!["*.json".to_string()],
            expected_included: vec![
                "src/main.rs".to_string(),
                "src/lib.rs".to_string(),
                "src/utils/helpers.rs".to_string(),
                "src/tests/test_main.rs".to_string(),
                "tests/integration/mod.rs".to_string(),
                "tests/unit/test_utils.rs".to_string(),
                "docs/README.md".to_string(),
                "README.md".to_string(),
                "src/lib/README.md".to_string(),
            ],
            expected_excluded: vec![
                "build/output.js".to_string(),
                "dist/app.js".to_string(),
                "node_modules/package/index.js".to_string(),
                "data/sample.json".to_string(),
                "src/data/config.json".to_string(),
                ".vscode/settings.json".to_string(),
                ".git/HEAD".to_string(),
            ],
            description: "File extension patterns (*.js and *.json)".to_string(),
        },
        // Negated patterns (not currently supported, but testing that they're ignored)
        IgnorePatternTestCase {
            gitignore_patterns: vec!["*.js".to_string(), "!dist/app.js".to_string()],
            digestignore_patterns: vec![],
            expected_included: vec![
                "src/main.rs".to_string(),
                "src/lib.rs".to_string(),
                "src/utils/helpers.rs".to_string(),
                "src/tests/test_main.rs".to_string(),
                "tests/integration/mod.rs".to_string(),
                "tests/unit/test_utils.rs".to_string(),
                "docs/README.md".to_string(),
                "README.md".to_string(),
                "src/lib/README.md".to_string(),
                "data/sample.json".to_string(),
                "src/data/config.json".to_string(),
                ".vscode/settings.json".to_string(),
                // dist/app.js should still be excluded because negated patterns aren't supported
            ],
            expected_excluded: vec![
                "build/output.js".to_string(),
                "dist/app.js".to_string(),
                "node_modules/package/index.js".to_string(),
                ".git/HEAD".to_string(),
            ],
            description: "Negated patterns (not supported, should be ignored)".to_string(),
        },
        // Comments in ignore files
        IgnorePatternTestCase {
            gitignore_patterns: vec![
                "# This is a comment".to_string(),
                "node_modules/".to_string(),
                "# Another comment".to_string(),
                "build/".to_string(),
            ],
            digestignore_patterns: vec!["# Digestignore comment".to_string(), "docs/".to_string()],
            expected_included: vec![
                "src/main.rs".to_string(),
                "src/lib.rs".to_string(),
                "src/utils/helpers.rs".to_string(),
                "src/tests/test_main.rs".to_string(),
                "tests/integration/mod.rs".to_string(),
                "tests/unit/test_utils.rs".to_string(),
                "README.md".to_string(),
                "src/lib/README.md".to_string(),
                "dist/app.js".to_string(),
                "data/sample.json".to_string(),
                "src/data/config.json".to_string(),
                ".vscode/settings.json".to_string(),
            ],
            expected_excluded: vec![
                "docs/README.md".to_string(),
                "build/output.js".to_string(),
                "node_modules/package/index.js".to_string(),
                ".git/HEAD".to_string(),
            ],
            description: "Comments in ignore files".to_string(),
        },
        // Complex directory patterns
        IgnorePatternTestCase {
            gitignore_patterns: vec!["**/data/".to_string()],
            digestignore_patterns: vec!["**/lib/".to_string()],
            expected_included: vec![
                "src/main.rs".to_string(),
                "src/lib.rs".to_string(),
                "src/utils/helpers.rs".to_string(),
                "src/tests/test_main.rs".to_string(),
                "tests/integration/mod.rs".to_string(),
                "tests/unit/test_utils.rs".to_string(),
                "docs/README.md".to_string(),
                "README.md".to_string(),
                "build/output.js".to_string(),
                "dist/app.js".to_string(),
                "node_modules/package/index.js".to_string(),
            ],
            expected_excluded: vec![
                "src/lib/README.md".to_string(),
                "data/sample.json".to_string(),
                "src/data/config.json".to_string(),
                ".git/HEAD".to_string(),
            ],
            description: "Complex directory patterns (**/data/ and **/lib/)".to_string(),
        },
    ]
}

/// Generate additional test cases with complex patterns
pub fn get_complex_test_cases() -> Vec<IgnorePatternTestCase> {
    vec![
        // Multiple pattern test
        IgnorePatternTestCase {
            gitignore_patterns: vec![
                "node_modules/".to_string(),
                "*.js".to_string(),
                "dist/".to_string(),
            ],
            digestignore_patterns: vec![
                "docs/".to_string(),
                "*.md".to_string(),
                "tests/".to_string(),
            ],
            expected_included: vec![
                "src/main.rs".to_string(),
                "src/lib.rs".to_string(),
                "src/utils/helpers.rs".to_string(),
                "src/tests/test_main.rs".to_string(),
                "data/sample.json".to_string(),
                "src/data/config.json".to_string(),
                ".vscode/settings.json".to_string(),
            ],
            expected_excluded: vec![
                "docs/README.md".to_string(),
                "README.md".to_string(),
                "src/lib/README.md".to_string(),
                "build/output.js".to_string(),
                "dist/app.js".to_string(),
                "node_modules/package/index.js".to_string(),
                "tests/integration/mod.rs".to_string(),
                "tests/unit/test_utils.rs".to_string(),
                ".git/HEAD".to_string(),
            ],
            description: "Multiple patterns from both gitignore and digestignore".to_string(),
        },
        // Wildcard in directory name
        IgnorePatternTestCase {
            gitignore_patterns: vec!["**/test*/".to_string()],
            digestignore_patterns: vec![],
            expected_included: vec![
                "src/main.rs".to_string(),
                "src/lib.rs".to_string(),
                "src/utils/helpers.rs".to_string(),
                "docs/README.md".to_string(),
                "README.md".to_string(),
                "src/lib/README.md".to_string(),
                "build/output.js".to_string(),
                "dist/app.js".to_string(),
                "node_modules/package/index.js".to_string(),
                "data/sample.json".to_string(),
                "src/data/config.json".to_string(),
                ".vscode/settings.json".to_string(),
            ],
            expected_excluded: vec![
                "src/tests/test_main.rs".to_string(),
                "tests/integration/mod.rs".to_string(),
                "tests/unit/test_utils.rs".to_string(),
                ".git/HEAD".to_string(),
            ],
            description: "Wildcard in directory name (**/test*/)".to_string(),
        },
        // Complex pattern with /**/ in the middle
        IgnorePatternTestCase {
            gitignore_patterns: vec!["src/**/config.json".to_string()],
            digestignore_patterns: vec!["**/*.md".to_string()],
            expected_included: vec![
                "src/main.rs".to_string(),
                "src/lib.rs".to_string(),
                "src/utils/helpers.rs".to_string(),
                "src/tests/test_main.rs".to_string(),
                "tests/integration/mod.rs".to_string(),
                "tests/unit/test_utils.rs".to_string(),
                "build/output.js".to_string(),
                "dist/app.js".to_string(),
                "node_modules/package/index.js".to_string(),
                "data/sample.json".to_string(),
            ],
            expected_excluded: vec![
                "src/data/config.json".to_string(),
                "docs/README.md".to_string(),
                "README.md".to_string(),
                "src/lib/README.md".to_string(),
                ".git/HEAD".to_string(),
            ],
            description: "Complex pattern with /**/ in the middle (src/**/config.json)".to_string(),
        },
        // Empty gitignore or digestignore
        IgnorePatternTestCase {
            gitignore_patterns: vec![],
            digestignore_patterns: vec!["**/*.md".to_string()],
            expected_included: vec![
                "src/main.rs".to_string(),
                "src/lib.rs".to_string(),
                "src/utils/helpers.rs".to_string(),
                "src/tests/test_main.rs".to_string(),
                "tests/integration/mod.rs".to_string(),
                "tests/unit/test_utils.rs".to_string(),
                "build/output.js".to_string(),
                "dist/app.js".to_string(),
                "node_modules/package/index.js".to_string(),
                "data/sample.json".to_string(),
                "src/data/config.json".to_string(),
                ".vscode/settings.json".to_string(),
            ],
            expected_excluded: vec![
                "docs/README.md".to_string(),
                "README.md".to_string(),
                "src/lib/README.md".to_string(),
                ".git/HEAD".to_string(),
            ],
            description: "Empty gitignore with populated digestignore".to_string(),
        },
        // Pattern with space
        IgnorePatternTestCase {
            gitignore_patterns: vec!["node_modules/ ".to_string()], // Note the trailing space
            digestignore_patterns: vec![],
            expected_included: vec![
                "src/main.rs".to_string(),
                "src/lib.rs".to_string(),
                "src/utils/helpers.rs".to_string(),
                "src/tests/test_main.rs".to_string(),
                "tests/integration/mod.rs".to_string(),
                "tests/unit/test_utils.rs".to_string(),
                "docs/README.md".to_string(),
                "README.md".to_string(),
                "src/lib/README.md".to_string(),
                "build/output.js".to_string(),
                "dist/app.js".to_string(),
                "data/sample.json".to_string(),
                "src/data/config.json".to_string(),
                ".vscode/settings.json".to_string(),
            ],
            expected_excluded: vec![
                "node_modules/package/index.js".to_string(),
                ".git/HEAD".to_string(),
            ],
            description: "Pattern with trailing space".to_string(),
        },
    ]
}

/// Test cases specifically for *.test.* pattern
pub fn get_test_file_patterns() -> Vec<IgnorePatternTestCase> {
    vec![IgnorePatternTestCase {
        gitignore_patterns: vec!["*.test.*".to_string()],
        digestignore_patterns: vec![],
        expected_included: vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "src/utils/helpers.rs".to_string(),
            "src/tests/test_main.rs".to_string(),
            "tests/integration/mod.rs".to_string(),
            "tests/unit/test_utils.rs".to_string(),
            "docs/README.md".to_string(),
            "README.md".to_string(),
            "src/lib/README.md".to_string(),
            "build/output.js".to_string(),
            "dist/app.js".to_string(),
            "node_modules/package/index.js".to_string(),
            "data/sample.json".to_string(),
            "src/data/config.json".to_string(),
            ".vscode/settings.json".to_string(),
        ],
        expected_excluded: vec![
            ".git/HEAD".to_string(),
            // No *.test.* files in our structure
        ],
        description: "*.test.* pattern with no matching files".to_string(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_common_ignore_patterns() -> Result<()> {
        let structure = get_common_test_structure();
        let test_cases = get_common_test_cases();

        run_ignore_pattern_tests(&structure, &test_cases)?;

        Ok(())
    }

    #[test]
    fn test_complex_ignore_patterns() -> Result<()> {
        let structure = get_common_test_structure();
        let test_cases = get_complex_test_cases();

        run_ignore_pattern_tests(&structure, &test_cases)?;

        Ok(())
    }

    #[test]
    fn test_special_patterns() -> Result<()> {
        let structure = get_common_test_structure();
        let test_cases = get_test_file_patterns();

        run_ignore_pattern_tests(&structure, &test_cases)?;

        Ok(())
    }
}
