# Testing Guide for Karkinos

This document describes the test suite for Karkinos and how to run tests.

## Running Tests

Once dependencies are available, run all tests:

```bash
cargo test
```

Run tests with output:

```bash
cargo test -- --nocapture
```

Run specific test:

```bash
cargo test test_name
```

## Test Coverage

### 1. Basic Functionality Tests

#### `populate_values_test`
Tests the core scraping functionality with nested data structures.

**What it tests:**
- Simple selector extraction (h1)
- Nested data extraction (articles with title and description)
- Array of nested items

**Input:** HTML with article elements
**Expected:** Correctly extracted title and array of article data

---

### 2. Type Conversion Tests

#### `test_number_conversion`
Tests conversion of text to numeric values.

**What it tests:**
- Regex extraction combined with number conversion
- Extracting "99.99" from "$99.99"
- Proper f64 conversion

**Config:**
```yaml
price:
  selector: .price
  regex: '\d+\.\d+'
  toNumber: true
```

**Expected:** `NumberItem(99.99)`

#### `test_boolean_conversion`
Tests conversion of text to boolean values.

**What it tests:**
- String to boolean conversion
- Recognizes "true", "yes", "1", "on" as truthy

**Config:**
```yaml
status:
  selector: .status
  toBoolean: true
```

**Expected:** `BoolItem(true)`

---

### 3. Default Value Tests

#### `test_default_value`
Tests fallback values when selectors don't match.

**What it tests:**
- Returns default value when selector finds no elements
- Prevents empty/null values

**Config:**
```yaml
missing:
  selector: .nonexistent
  default: "Default Value"
```

**Expected:** `StringItem("Default Value")`

---

### 4. Text Transformation Tests

#### `test_uppercase_transformation`
Tests converting text to uppercase.

**Input:** "hello world"
**Expected:** "HELLO WORLD"

#### `test_lowercase_transformation`
Tests converting text to lowercase.

**Input:** "HELLO WORLD"
**Expected:** "hello world"

#### `test_regex_extraction`
Tests extracting patterns using regular expressions.

**Input:** "Price: $99.99 USD"
**Pattern:** `\d+\.\d+`
**Expected:** "99.99"

**Use cases:**
- Extract prices from formatted text
- Extract phone numbers, emails, dates
- Parse structured text

#### `test_text_replacement`
Tests find and replace functionality.

**Input:** "Breaking: News Title"
**Replace:** ["Breaking: ", ""]
**Expected:** "News Title"

**Use cases:**
- Remove prefixes/suffixes
- Clean up formatting
- Normalize data

#### `test_html_stripping`
Tests removal of HTML tags from content.

**Input:** `<p>Hello <strong>World</strong></p>`
**Expected:** "Hello World"

**Use cases:**
- Extract plain text from rich HTML
- Clean descriptions
- Remove formatting

#### `test_trim_whitespace`
Tests whitespace trimming functionality.

**Input:** "   Hello World   "
**Expected:** "Hello World"

---

### 5. Combined Transformation Tests

#### `test_combined_transformations`
Tests chaining multiple transformations together.

**What it tests:**
- HTML stripping + regex extraction + number conversion
- Transformation pipeline execution order

**Input:** `<div class="product"><strong>Price: $99.99</strong></div>`

**Transformations:**
1. Strip HTML tags → "Price: $99.99"
2. Extract with regex → "99.99"
3. Convert to number → 99.99

**Expected:** `NumberItem(99.99)`

---

## Test Statistics

- **Total Tests:** 11
- **Coverage Areas:**
  - Basic scraping: 1 test
  - Type conversion: 2 tests
  - Default values: 1 test
  - Text transformations: 6 tests
  - Combined operations: 1 test

## Test Dependencies

Tests use these crates:
- `serde_yaml` - Parse YAML configs in tests
- `scraper` - HTML parsing (tested indirectly)
- `regex` - Pattern extraction

## Adding New Tests

When adding new features, follow this pattern:

```rust
#[test]
fn test_feature_name() {
    // Setup HTML
    let html = r#"<div class="test">content</div>"#.to_string();

    // Setup config
    let yaml_config = r#"
    field:
        selector: .test
        # your feature config
    "#;
    let data_config = serde_yaml::from_str::<DataConfig>(yaml_config).unwrap();

    // Execute
    let data = populate_values(html, data_config);

    // Assert
    assert_eq!(
        data.get("field").unwrap().clone(),
        StringItem(String::from("expected"))
    );
}
```

## Integration Tests

For full end-to-end tests, create files in `tests/` directory:

```rust
// tests/integration_test.rs
use std::fs;

#[test]
fn test_example_config() {
    // Test that example configs are valid
    let config = fs::read_to_string("example/example.krk.yaml").unwrap();
    let parsed: karkinos::ScrapeRoot = serde_yaml::from_str(&config).unwrap();
    assert!(parsed.validate().is_ok());
}
```

## Test Data

Test HTML examples should:
1. Be minimal but realistic
2. Cover edge cases
3. Use standard HTML structure
4. Include comments explaining the scenario

## Continuous Integration

Tests should run in CI:

```yaml
# .github/workflows/test.yml
name: Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test --verbose
```

## Performance Tests

For performance-critical functions, use benchmarks:

```rust
#[cfg(test)]
mod benches {
    use test::Bencher;

    #[bench]
    fn bench_populate_values(b: &mut Bencher) {
        let html = /* large HTML */;
        let config = /* config */;
        b.iter(|| populate_values(html.clone(), config.clone()));
    }
}
```

## Test Checklist

When adding new features, ensure:

- [ ] Unit tests for the feature
- [ ] Edge case tests (empty, null, malformed)
- [ ] Integration test with full config
- [ ] Example configuration file
- [ ] Documentation of the feature
- [ ] Update this TESTING.md

## Known Issues

None currently. Report issues at: https://github.com/ggagosh/karkinos/issues

## Test Results Format

Expected test output:

```
running 11 tests
test tests::populate_values_test ... ok
test tests::test_boolean_conversion ... ok
test tests::test_combined_transformations ... ok
test tests::test_default_value ... ok
test tests::test_html_stripping ... ok
test tests::test_lowercase_transformation ... ok
test tests::test_number_conversion ... ok
test tests::test_regex_extraction ... ok
test tests::test_text_replacement ... ok
test tests::test_trim_whitespace ... ok
test tests::test_uppercase_transformation ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```
