# karkinos `Καρκινος`

> 🦀🦀🦀 Powerful and flexible website scraper written in Rust 🦀🦀🦀

Inspired by [scrape-it](https://github.com/IonicaBizau/scrape-it)

## Features

- **Simple YAML Configuration**: Define scraping rules using intuitive YAML syntax
- **CSS Selectors**: Extract data using standard CSS selectors
- **Nested Data**: Support for complex nested data structures
- **Data Transformations**: Built-in text processing (regex, case conversion, type casting)
- **Multiple URLs**: Batch scrape multiple pages in one configuration
- **HTTP Features**: Custom headers, proxy support, timeout, and retry logic
- **Rate Limiting**: Respectful scraping with configurable delays
- **Caching**: Cache responses for faster development and testing
- **Multiple Output Formats**: Export to JSON or CSV
- **Parallel Processing**: Fast extraction using Rayon
- **Type Conversion**: Automatic conversion to numbers and booleans

## Installation

```bash
cargo install --path .
```

## Usage

```bash
# Basic usage
main config.krk.yaml

# Save to file
main config.krk.yaml -o output.json

# Export to CSV
main config.krk.yaml -o output.csv -f csv
```

## Configuration

### Basic Structure

```yaml
config:
  url: https://example.com
data:
  title:
    selector: h1
  description:
    selector: .description
```

### Configuration Options

#### HTTP Configuration

```yaml
config:
  # Single URL
  url: https://example.com

  # OR multiple URLs for batch scraping
  urls:
    - https://example.com/page1
    - https://example.com/page2

  # Custom headers
  headers:
    User-Agent: "Mozilla/5.0"
    Cookie: "session=abc123"

  # Timeout in seconds (default: 30)
  timeout: 60

  # Number of retry attempts (default: 0)
  retries: 3

  # Delay between requests in milliseconds (default: 0)
  delay: 1000

  # Proxy configuration
  proxy: http://proxy.example.com:8080

  # Cache configuration
  cacheDir: ./.cache
  useCache: true
```

#### Data Extraction

```yaml
data:
  # Simple text extraction
  title:
    selector: h1

  # Extract from attribute
  image:
    selector: img.featured
    attr: src

  # Select nth element (0-indexed)
  firstParagraph:
    selector: p
    nth: 0

  # Default value if not found
  author:
    selector: .author
    default: "Unknown"

  # Disable trimming
  rawText:
    selector: .content
    trim: false
```

#### Data Transformations

```yaml
data:
  # Extract using regex
  price:
    selector: .price
    regex: '\d+\.\d+'
    toNumber: true

  # Text replacement
  cleanTitle:
    selector: h1
    replace: ["Breaking: ", ""]

  # Case conversion
  upperTitle:
    selector: h1
    uppercase: true

  lowerTitle:
    selector: h1
    lowercase: true

  # Strip HTML tags
  cleanText:
    selector: .content
    stripHtml: true

  # Type conversion
  rating:
    selector: .rating
    toNumber: true

  isActive:
    selector: .status
    toBoolean: true
```

#### Nested Data

```yaml
data:
  articles:
    selector: article
    data:
      title:
        selector: h2
      author:
        selector: .author
      tags:
        selector: .tag
        data:
          name:
            selector: span
```

## Examples

### Example 1: Basic Scraping

```yaml
config:
  url: https://news.ycombinator.com
data:
  stories:
    selector: .athing
    data:
      title:
        selector: .titleline > a
      score:
        selector: .score
        toNumber: true
```

### Example 2: Multiple URLs with Rate Limiting

```yaml
config:
  urls:
    - https://example.com/page1
    - https://example.com/page2
  delay: 2000
  headers:
    User-Agent: "Karkinos/1.0"
data:
  title:
    selector: h1
  content:
    selector: article
```

### Example 3: Advanced Transformations

```yaml
config:
  url: https://example.com/products
data:
  products:
    selector: .product
    data:
      name:
        selector: .product-name
        stripHtml: true
      price:
        selector: .price
        regex: '\d+\.\d+'
        toNumber: true
      inStock:
        selector: .availability
        toBoolean: true
```

### Example 4: Cached Development

```yaml
config:
  url: https://example.com
  cacheDir: ./.scrape-cache
  useCache: true
  timeout: 30
  retries: 2
data:
  content:
    selector: .main-content
```

## Output Formats

### JSON (default)

```bash
main config.krk.yaml -o output.json
```

### CSV

```bash
main config.krk.yaml -o output.csv -f csv
```

Note: CSV output flattens simple fields. Nested arrays are JSON-encoded.

## Development

### Generate JSON Schema

```bash
cargo run --bin gen
```

This creates `krk-schema.json` for configuration validation.

### Run Tests

```bash
cargo test
```

## License

MIT
