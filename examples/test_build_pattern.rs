use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

use digest::should_ignore;

fn main() -> Result<()> {
    // Test the build/** pattern specifically
    println!("Testing pattern: build/**");

    let patterns = HashSet::from(["build/**".to_string()]);

    let test_paths = [
        // Should match
        ("build/file.js", true),
        ("build/output/bundle.js", true),
        // Should NOT match
        ("builds/file.js", false),
        ("src/build.js", false),
    ];

    for (path_str, should_be_ignored) in &test_paths {
        let path = Path::new(path_str);
        let is_ignored = should_ignore(path, &patterns);

        if is_ignored == *should_be_ignored {
            println!("✓ OK: '{}' behaved correctly", path_str);
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

    // Test a more complex pattern
    println!("\nTesting pattern: **/test*/**");

    let patterns = HashSet::from(["**/test*/**".to_string()]);

    let test_paths = [
        // Should match
        ("test/file.js", true),
        ("src/test/util.js", true),
        ("src/testing/file.js", true),
        ("deeply/nested/test/script.js", true),
        // Should NOT match
        ("tst/file.js", false),
        ("src/tst/util.js", false),
        ("test_file.js", false),
    ];

    for (path_str, should_be_ignored) in &test_paths {
        let path = Path::new(path_str);
        let is_ignored = should_ignore(path, &patterns);

        if is_ignored == *should_be_ignored {
            println!("✓ OK: '{}' behaved correctly", path_str);
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
