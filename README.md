# Digest

A CLI tool to create digests of code repositories for LLM consumption.

## Overview

Digest analyzes a codebase to create a summarized representation that's easier for Large Language Models (LLMs) to process. It automatically detects the predominant language of the project, identifies the most relevant files while skipping irrelevant ones (like node_modules, .git, etc.), and produces a structured output.

## Features

- Automatic detection of the project's primary programming language
- Intelligent filtering of irrelevant files and directories
- Language-specific filtering rules
- Output in either Markdown or JSON format
- Configurable limits for file size and count

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/digest.git
cd digest

# Build the project
cargo build --release

# (Optional) Install globally
cargo install --path .
```

## Usage

```bash
# Generate a digest for the current directory
digest

# Generate a digest for a specific project
digest /path/to/project

# Limit to 20 files with a maximum size of 50KB each
digest --max-files 20 --max-file-size 50

# Output in JSON format
digest --format json

# Save output to a file
digest --output project-digest.md
```

### Options

- `<PROJECT_PATH>`: Path to the project directory (defaults to current directory)
- `-m, --max-files <MAX_FILES>`: Maximum number of files to include (default: 50)
- `-s, --max-file-size <MAX_FILE_SIZE>`: Maximum file size in KB (default: 100)
- `-f, --format <FORMAT>`: Output format: 'markdown' or 'json' (default: markdown)
- `-o, --output <OUTPUT>`: Output file (defaults to stdout)

## Example Output

The Markdown output includes:

1. Project name
2. Language breakdown with statistics
3. A list of all included files with their content

## License

MIT 