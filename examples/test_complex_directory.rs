use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

use digest::should_ignore;

fn main() -> Result<()> {
    // Test the **/test*/** pattern which might be problematic
    println!("Testing pattern: **/test*/**");

    let patterns = HashSet::from(["**/test*/**".to_string()]);

    let test_paths = [
        // Should match
        ("test/file.js", true),
        ("src/test/util.js", true),
        ("src/tests/util.js", true),
        ("src/testing/file.js", true),
        ("deeply/nested/test/script.js", true),
        ("tests/integration/mod.rs", true),
        // Should NOT match
        ("tst/file.js", false),
        ("src/tst/util.js", false),
        ("test_file.js", false),
    ];

    for (path_str, should_be_ignored) in &test_paths {
        let path = Path::new(path_str);
        let is_ignored = should_ignore(path, &patterns);

        if is_ignored == *should_be_ignored {
            println!(
                "✓ OK: '{}' behaved correctly (expected ignored: {})",
                path_str, should_be_ignored
            );
        } else {
            if *should_be_ignored {
                println!(
                    "❌ Error: Expected '{}' to be ignored, but it wasn't",
                    path_str
                );
            } else {
                println!(
                    "❌ Error: Expected '{}' NOT to be ignored, but it was",
                    path_str
                );
            }
        }
    }

    // Now test with **/*.md pattern
    println!("\nTesting pattern: **/*.md");

    let patterns = HashSet::from(["**/*.md".to_string()]);

    let test_paths = [
        // Should match
        ("README.md", true),
        ("docs/README.md", true),
        ("src/lib/README.md", true),
        // Should NOT match
        ("readme.txt", false),
        ("markdown/file.txt", false),
    ];

    for (path_str, should_be_ignored) in &test_paths {
        let path = Path::new(path_str);
        let is_ignored = should_ignore(path, &patterns);

        if is_ignored == *should_be_ignored {
            println!(
                "✓ OK: '{}' behaved correctly (expected ignored: {})",
                path_str, should_be_ignored
            );
        } else {
            if *should_be_ignored {
                println!(
                    "❌ Error: Expected '{}' to be ignored, but it wasn't",
                    path_str
                );
            } else {
                println!(
                    "❌ Error: Expected '{}' NOT to be ignored, but it was",
                    path_str
                );
            }
        }
    }

    Ok(())
}
