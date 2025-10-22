# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Karkinos is a powerful and flexible web scraper written in Rust, inspired by scrape-it. It allows users to define scraping rules using YAML configuration files and supports advanced features like pagination, data transformations, caching, and multiple output formats.

## Common Commands

### Build and Run
```bash
# Build the project
cargo build

# Build for release
cargo build --release

# Run the main scraper with a config file
cargo run --bin main example/example.krk.yaml

# Save output to file
cargo run --bin main config.krk.yaml -o output.json

# Export to CSV
cargo run --bin main config.krk.yaml -o output.csv -f csv
```

### Testing
```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_name
```

### Schema Generation
```bash
# Generate JSON schema for configuration validation
cargo run --bin gen
```

This creates `krk-schema.json` which can be used for IDE autocomplete and YAML validation.

## Code Architecture

### Core Components

**src/types.rs** - Type definitions and data structures
- `ScrapeRoot`: Top-level configuration structure containing `config` and `data`
- `ScrapeRootConfig`: HTTP settings (url/urls, headers, timeout, retries, proxy, caching, pagination)
- `ItemConfig`: Individual field extraction rules (selector, transformations, type conversions)
- `ReturnedDataItem`: Enum for output types (String, Number, Bool, or nested Arrays)
- `PaginationConfig`: Pagination strategies (URL patterns or next-link following)

**src/main.rs** - Main application logic (593 lines)
- `fetch_with_config()`: HTTP client with retry logic, caching, and proxy support
- `populate_values()`: Core scraping engine that extracts data using CSS selectors
- `apply_transformations()`: Transformation pipeline (regex, case conversion, HTML stripping, etc.)
- `generate_paginated_urls()`: Pagination URL generation and next-link following
- `is_empty_result()`: Helper for detecting empty pages (used by stopOnEmpty)
- `write_output()`: JSON/CSV output generation

**src/gen.rs** - JSON schema generator using schemars

### Key Design Patterns

**Parallel Processing**: Uses Rayon for concurrent extraction of list items. When extracting nested data (e.g., multiple articles on a page), processing happens in parallel across CPU cores.

**Transformation Pipeline**: Text transformations are applied in sequence:
1. Extract raw text using CSS selector
2. Strip HTML tags (if `stripHtml: true`)
3. Apply regex extraction (if `regex` specified)
4. Apply text replacement (if `replace` specified)
5. Case conversion (if `uppercase`/`lowercase`)
6. Trim whitespace (default, unless `trim: false`)
7. Type conversion (`toNumber` or `toBoolean`)

**Caching System**: Responses are cached using SHA-256 hash of URL as filename. Cache is stored in `cacheDir` and controlled by `useCache` flag. No automatic expiration - manual cleanup required.

**Pagination Strategies**:
- **URL Pattern**: Generate URLs using `{page}` placeholder (e.g., `?page={page}`)
- **Next Link**: Follow CSS selector for "next page" link until none found
- Both strategies support `stopOnEmpty` to halt when page has no results

### Important Implementation Details

**URL Handling**: The code supports both single `url` or multiple `urls`. When multiple URLs are provided, output is an array; single URL returns an object.

**Retry Logic**: Exponential backoff is implemented for retries. Each failed request waits progressively longer before retry attempt.

**Rate Limiting**: The `delay` parameter (milliseconds) is applied between sequential requests to avoid overwhelming servers. Critical for pagination and multi-URL scraping.

**Type Conversion**:
- `toNumber`: Parses string to f64, returns 0.0 on failure
- `toBoolean`: "true", "yes", "1", "on" (case-insensitive) → true; everything else → false

**Nested Data**: When `data` field is present in `ItemConfig`, the selector matches multiple elements and recursively extracts sub-fields from each match, returning an array.

**Default Values**: If selector matches no elements, `default` value is used instead. Without default, field may be empty string.

## Configuration Structure

YAML files must have two top-level keys:

```yaml
config:
  url: https://example.com  # OR urls: [...]
  # HTTP settings
  headers: { User-Agent: "..." }
  timeout: 30
  retries: 3
  delay: 1000
  proxy: http://proxy:8080
  # Caching
  cacheDir: ./.cache
  useCache: true
  # Pagination
  pagination:
    pagePattern: "?page={page}"  # OR nextSelector: "a.next"
    startPage: 1
    maxPages: 10
    stopOnEmpty: false

data:
  fieldName:
    selector: .css-selector
    attr: src  # Extract attribute instead of text
    nth: 0  # Select specific element index
    # Transformations
    regex: '\d+\.\d+'
    replace: ["pattern", "replacement"]
    uppercase: true
    lowercase: true
    stripHtml: true
    trim: true  # default
    # Type conversion
    toNumber: true
    toBoolean: true
    default: "fallback value"
    # Nested data
    data:
      nestedField:
        selector: .nested
```

## Testing Guidelines

Tests are in `src/main.rs` under `#[cfg(test)]` module. Test coverage includes:
- Basic scraping (selectors, nested data)
- Type conversions (number, boolean)
- Transformations (regex, case, HTML stripping, replacement)
- Default values
- Combined transformation pipelines
- Pagination (URL patterns, next-link, stopOnEmpty)

When adding features:
1. Add unit test in `src/main.rs` tests module
2. Create example config in `example/` directory
3. Update `FEATURES.md` with detailed documentation
4. Update `README.md` usage section
5. Run `cargo run --bin gen` to update schema
