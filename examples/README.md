# Digest Examples

This directory contains examples to test and demonstrate various functionality of the `digest` crate.

## Running Examples

Run an example with:

```sh
cargo run --example <example_name>
```

For example:

```sh
cargo run --example test_gitignore_patterns
```

## Available Examples

### `test_gitignore_patterns`

Tests the basic functionality of gitignore and digestignore pattern matching. This example:

1. Creates a temporary directory with a set of files and directories
2. Creates `.gitignore` and `.digestignore` files with specific patterns
3. Tests which files should be ignored and which should be included
4. Tests the interaction between both ignore files

### `test_complex_patterns`

Tests various complex gitignore pattern types to ensure they're properly handled. This example tests:

1. Directory patterns (ending with `/`)
2. File extension patterns (like `*.js`)
3. Deep matching patterns with `**/prefix`
4. Subdirectory patterns with `prefix/**`
5. Path matching with `/**/` in the middle
6. Special patterns like `*.test.*`

Each pattern is tested against multiple paths that should match and paths that should not match. 