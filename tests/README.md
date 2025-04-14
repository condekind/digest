# Digest Tests

This directory contains tests for the `digest` crate, primarily focusing on testing the ignore pattern functionality.

## Running Tests

Run all tests with:

```sh
cargo test
```

Run a specific test with:

```sh
cargo test test_gitignore_only
```

Run tests with output:

```sh
cargo test -- --nocapture
```

## Test Structure

### Ignore Pattern Tests

The `ignore_pattern_tests` directory contains tests for the gitignore and digestignore pattern matching functionality:

1. **Basic Tests**: Test individual ignore pattern handling
   - `test_gitignore_only`: Tests only gitignore patterns
   - `test_digestignore_only`: Tests only digestignore patterns
   - `test_both_ignore_files`: Tests the interaction between both files

2. **Programmatic Pattern Tests**: Test a wide variety of ignore pattern combinations
   - Uses a pattern generator to test many combinations
   - Tests simple patterns, glob patterns, and complex patterns
   - Creates files programmatically to test against

### How It Works

The tests use the following approach:

1. Creates a temporary directory with a predefined file structure
2. Creates `.gitignore` and `.digestignore` files with specific patterns
3. Calls the `collect_relevant_files` function to gather files based on the patterns
4. Verifies that the correct files are included/excluded

The programmatic tests generate multiple combinations of patterns and directory structures
to ensure thorough testing of the pattern matching logic.

## Adding New Tests

To add a new test case:

1. For simple tests, create a new test function in `ignore_pattern_tests/mod.rs`
2. For programmatic tests, add a new test case to one of the functions in `pattern_generator.rs`:
   - `get_common_test_cases()`: For basic pattern tests
   - `get_complex_test_cases()`: For more complex pattern combinations
   - Or create a new function for a specific type of pattern

Alternatively, create a new test structure by adding a function similar to 
`get_common_test_structure()`. 