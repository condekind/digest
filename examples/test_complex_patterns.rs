use anyhow::Result;
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

use digest::should_ignore;

// A test case for pattern matching
struct PatternTest {
    pattern: String,
    paths_to_ignore: Vec<String>,
    paths_to_include: Vec<String>,
}

fn run_pattern_test(test: &PatternTest) -> Result<()> {
    let patterns = HashSet::from([test.pattern.clone()]);

    println!("Testing pattern: {}", test.pattern);

    // Test paths that should be ignored
    for path_str in &test.paths_to_ignore {
        let path = Path::new(path_str);
        if !should_ignore(path, &patterns) {
            println!(
                "❌ Error: Expected '{}' to be ignored, but it wasn't",
                path_str
            );
        } else {
            println!("✓ OK: '{}' was ignored as expected", path_str);
        }
    }

    // Test paths that should NOT be ignored
    for path_str in &test.paths_to_include {
        let path = Path::new(path_str);
        if should_ignore(path, &patterns) {
            println!(
                "❌ Error: Expected '{}' NOT to be ignored, but it was",
                path_str
            );
        } else {
            println!("✓ OK: '{}' was not ignored as expected", path_str);
        }
    }

    println!("");
    Ok(())
}

fn main() -> Result<()> {
    let tests = vec![
        // Test directory pattern (ends with /)
        PatternTest {
            pattern: "node_modules/".to_string(),
            paths_to_ignore: vec![
                "node_modules/package.json".to_string(),
                "src/node_modules/file.js".to_string(),
            ],
            paths_to_include: vec![
                "src/nodemodules.js".to_string(),
                "node_modulesdir/file.js".to_string(),
            ],
        },
        // Test file extension pattern
        PatternTest {
            pattern: "*.js".to_string(),
            paths_to_ignore: vec![
                "file.js".to_string(),
                "src/app.js".to_string(),
                "deeply/nested/script.js".to_string(),
            ],
            paths_to_include: vec![
                "file.jsx".to_string(),
                "javascript.txt".to_string(),
                "js/file.ts".to_string(),
            ],
        },
        // Test **/ prefix pattern
        PatternTest {
            pattern: "**/test/**".to_string(),
            paths_to_ignore: vec![
                "test/file.js".to_string(),
                "src/test/util.js".to_string(),
                "deeply/nested/test/script.js".to_string(),
            ],
            paths_to_include: vec![
                "testing/file.js".to_string(),
                "src/tests/util.js".to_string(),
                "test_file.js".to_string(),
            ],
        },
        // Test /** suffix pattern
        PatternTest {
            pattern: "build/**".to_string(),
            paths_to_ignore: vec![
                "build/file.js".to_string(),
                "build/output/bundle.js".to_string(),
            ],
            paths_to_include: vec!["builds/file.js".to_string(), "src/build.js".to_string()],
        },
        // Test /**/ middle pattern
        PatternTest {
            pattern: "src/**/config.json".to_string(),
            paths_to_ignore: vec![
                "src/config.json".to_string(),
                "src/app/config.json".to_string(),
                "src/deep/nested/config.json".to_string(),
            ],
            paths_to_include: vec![
                "config.json".to_string(),
                "src/config.js".to_string(),
                "src/app/configuration.json".to_string(),
            ],
        },
        // Test *.test.* pattern
        PatternTest {
            pattern: "*.test.*".to_string(),
            paths_to_ignore: vec![
                "file.test.js".to_string(),
                "component.test.tsx".to_string(),
                "src/util.test.ts".to_string(),
            ],
            paths_to_include: vec![
                "filetest.js".to_string(),
                "test.js".to_string(),
                "src/testutil.js".to_string(),
            ],
        },
    ];

    for test in tests {
        run_pattern_test(&test)?;
    }

    Ok(())
}
