# Karkinos - New Features and Capabilities

This document provides a comprehensive overview of all the enhanced capabilities added to Karkinos.

## Table of Contents

1. [HTTP Configuration](#http-configuration)
2. [Data Transformation](#data-transformation)
3. [Multiple URL Support](#multiple-url-support)
4. [Pagination](#pagination)
5. [Caching System](#caching-system)
6. [Output Formats](#output-formats)
7. [Type System](#type-system)

---

## HTTP Configuration

### Custom Headers

Send custom HTTP headers with your requests for authentication, user-agent spoofing, or API access.

```yaml
config:
  url: https://api.example.com
  headers:
    User-Agent: "Karkinos/1.0"
    Authorization: "Bearer token123"
    Accept: "application/json"
```

**Use cases:**
- API authentication
- Bypassing simple bot detection
- Setting custom cookies
- Content negotiation

### Timeout Configuration

Control how long to wait for server responses.

```yaml
config:
  url: https://slow-website.com
  timeout: 60  # seconds (default: 30)
```

**Benefits:**
- Prevent hanging on slow servers
- Adjust for fast or slow connections
- Better error handling

### Retry Logic

Automatically retry failed requests with configurable attempts.

```yaml
config:
  url: https://unreliable-site.com
  retries: 3  # number of retry attempts (default: 0)
```

**Features:**
- Exponential backoff between retries
- Handles network failures gracefully
- Configurable retry count
- Detailed logging of retry attempts

### Proxy Support

Route requests through HTTP/HTTPS proxies.

```yaml
config:
  url: https://example.com
  proxy: http://proxy.example.com:8080
```

**Use cases:**
- IP rotation
- Geographic restrictions
- Corporate networks
- Privacy

### Rate Limiting

Add delays between requests to avoid overloading servers.

```yaml
config:
  urls:
    - https://example.com/page1
    - https://example.com/page2
  delay: 2000  # milliseconds between requests
```

**Benefits:**
- Respectful scraping
- Avoid rate limiting
- Prevent IP bans
- Server-friendly

---

## Data Transformation

### Default Values

Provide fallback values when selectors don't match.

```yaml
data:
  author:
    selector: .author
    default: "Unknown"
```

### Regex Extraction

Extract specific patterns from text using regular expressions.

```yaml
data:
  price:
    selector: .price-text
    regex: '\d+\.\d+'  # Extract "99.99" from "Price: $99.99"
```

**Examples:**
- Extract prices: `\d+\.\d+`
- Extract emails: `[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}`
- Extract phone numbers: `\d{3}-\d{3}-\d{4}`
- Extract dates: `\d{4}-\d{2}-\d{2}`

### Text Replacement

Find and replace text patterns.

```yaml
data:
  cleanTitle:
    selector: h1
    replace: ["Breaking News: ", ""]  # Remove prefix
```

**Use cases:**
- Remove prefixes/suffixes
- Clean up formatting
- Normalize text
- Remove unwanted characters

### Case Conversion

Convert text to uppercase or lowercase.

```yaml
data:
  upperTitle:
    selector: h1
    uppercase: true

  lowerTitle:
    selector: h1
    lowercase: true
```

### HTML Stripping

Remove HTML tags from extracted content.

```yaml
data:
  cleanDescription:
    selector: .description
    stripHtml: true
```

**Benefits:**
- Get plain text from rich HTML
- Remove formatting
- Clean data for processing

### Trim Control

Control whitespace trimming (enabled by default).

```yaml
data:
  preservedText:
    selector: pre
    trim: false  # Keep whitespace
```

---

## Type Conversion

### Number Conversion

Automatically convert strings to numbers.

```yaml
data:
  price:
    selector: .price
    toNumber: true  # "99.99" → 99.99
```

**Benefits:**
- Numeric calculations
- Proper JSON typing
- Sorting and comparison
- Data analysis

### Boolean Conversion

Convert text to boolean values.

```yaml
data:
  inStock:
    selector: .availability
    toBoolean: true  # "true", "yes", "1", "on" → true
```

**Truthy values:** `true`, `yes`, `1`, `on` (case-insensitive)
**Falsy values:** Everything else

---

## Multiple URL Support

### Batch Scraping

Scrape multiple pages with a single configuration.

```yaml
config:
  urls:
    - https://example.com/page1
    - https://example.com/page2
    - https://example.com/page3
  delay: 1000  # Wait 1s between requests
```

**Features:**
- Single configuration for multiple pages
- Automatic rate limiting
- Combined or separate output
- Progress logging

**Output behavior:**
- Single URL: Returns object
- Multiple URLs: Returns array of objects

---

## Pagination

Automatically scrape multiple pages using URL patterns or by following "next" links.

### URL Pattern Pagination

Generate URLs using a pattern with a `{page}` placeholder.

```yaml
config:
  url: https://example.com/products
  pagination:
    pagePattern: "?page={page}"
    startPage: 1
    endPage: 10
    stopOnEmpty: false
  delay: 1000
data:
  products:
    selector: .product
    data:
      name:
        selector: .name
      price:
        selector: .price
        toNumber: true
```

**How it works:**
1. Generates URLs: `https://example.com/products?page=1`, `?page=2`, etc.
2. Scrapes each page with the same data configuration
3. Combines results from all pages
4. Applies rate limiting between pages

**Use cases:**
- E-commerce product listings
- Search results
- Blog archives
- Any site with numbered pages

### Next Link Pagination

Follow "next page" links automatically.

```yaml
config:
  url: https://example.com/blog
  pagination:
    nextSelector: "a.next-page"
    maxPages: 20
    stopOnEmpty: true
  delay: 2000
data:
  articles:
    selector: article
    data:
      title:
        selector: h2
      content:
        selector: .content
```

**How it works:**
1. Scrapes the first page
2. Finds the "next" link using CSS selector
3. Follows the link and scrapes the next page
4. Repeats until no "next" link found or maxPages reached
5. Handles relative and absolute URLs automatically

**Use cases:**
- Blogs with "next page" buttons
- Forums with sequential pagination
- Any site with navigation links

### Full URL Pattern

Specify complete URLs with page numbers.

```yaml
config:
  url: https://example.com
  pagination:
    pagePattern: "https://example.com/search?q=rust&page={page}"
    startPage: 1
    maxPages: 5
```

**Use cases:**
- Complex URL structures
- Search results with parameters
- APIs with pagination

### Configuration Options

| Option | Type | Description | Default |
|--------|------|-------------|---------|
| `pagePattern` | string | URL pattern with `{page}` placeholder | - |
| `nextSelector` | string | CSS selector for "next page" link | - |
| `startPage` | number | Starting page number | 1 |
| `maxPages` | number | Maximum pages to scrape (0 = unlimited) | 0 |
| `endPage` | number | Ending page number (pagePattern only) | - |
| `stopOnEmpty` | boolean | Stop if no results found on page | false |

**Note:** Use either `pagePattern` OR `nextSelector`, not both.

### Stop on Empty

Automatically stop pagination when a page has no results.

```yaml
config:
  url: https://example.com/products
  pagination:
    pagePattern: "?page={page}"
    startPage: 1
    maxPages: 100
    stopOnEmpty: true  # Stops early if page is empty
```

**Benefits:**
- Avoid scraping empty pages
- Save bandwidth and time
- Automatic detection of last page

### Examples

**Example 1: E-commerce Pagination**
```yaml
config:
  url: https://shop.example.com/products
  pagination:
    pagePattern: "?page={page}"
    startPage: 1
    endPage: 50
  delay: 1000
  headers:
    User-Agent: "Karkinos Bot 1.0"
data:
  products:
    selector: .product-card
    data:
      name:
        selector: .name
      price:
        selector: .price
        regex: '\d+\.\d+'
        toNumber: true
```

**Example 2: Blog with Next Links**
```yaml
config:
  url: https://blog.example.com
  pagination:
    nextSelector: "a[rel='next']"
    maxPages: 20
    stopOnEmpty: true
  delay: 2000
  cacheDir: ./blog-cache
  useCache: true
data:
  posts:
    selector: article.post
    data:
      title:
        selector: h1
      date:
        selector: time
        attr: datetime
      body:
        selector: .post-content
        stripHtml: true
```

**Example 3: API Pagination**
```yaml
config:
  url: https://api.example.com
  pagination:
    pagePattern: "https://api.example.com/items?offset={page}0&limit=10"
    startPage: 0
    maxPages: 10
  headers:
    Authorization: "Bearer token123"
data:
  items:
    selector: .item
    data:
      id:
        selector: .id
      value:
        selector: .value
```

### Best Practices

1. **Always use delays** to avoid overwhelming servers:
   ```yaml
   delay: 1000  # At least 1 second between pages
   ```

2. **Set reasonable limits**:
   ```yaml
   maxPages: 50  # Don't scrape thousands of pages
   ```

3. **Use caching during development**:
   ```yaml
   cacheDir: ./pagination-cache
   useCache: true
   ```

4. **Use stopOnEmpty** when appropriate:
   ```yaml
   stopOnEmpty: true  # Auto-detect last page
   ```

5. **Monitor progress** with logging:
   ```bash
   RUST_LOG=info cargo run --bin main config.yaml
   ```

---

## Caching System

### Response Caching

Cache downloaded pages for faster development and testing.

```yaml
config:
  url: https://example.com
  cacheDir: ./.scrape-cache
  useCache: true
```

**How it works:**
1. First request: Download and cache response
2. Subsequent requests: Use cached version
3. Cache key: SHA-256 hash of URL

**Benefits:**
- Faster development iterations
- Reduce server load during testing
- Work offline
- Consistent test data

**Cache location:**
- Files stored in `cacheDir`
- Filename: SHA-256 hash of URL + `.html`
- Manual cleanup required

---

## Output Formats

### JSON Output (Default)

Pretty-printed JSON output.

```bash
main config.krk.yaml -o output.json
```

**Features:**
- Nested data structures
- Type preservation (string, number, boolean, array)
- Human-readable formatting
- Standard JSON

### CSV Output

Export flat data to CSV.

```bash
main config.krk.yaml -o output.csv -f csv
```

**Features:**
- Spreadsheet-compatible
- Simple data analysis
- Database import
- Nested arrays are JSON-encoded

**Limitations:**
- Best for flat data structures
- Nested objects become JSON strings
- Arrays become JSON strings

---

## Type System

### Supported Data Types

**In configuration:**
- `selector`: CSS selector (string)
- `attr`: HTML attribute name (string)
- `nth`: Element index (number)
- `default`: Default value (string)
- `regex`: Regular expression (string)
- `replace`: [pattern, replacement] (array)
- `trim`, `uppercase`, `lowercase`, `toNumber`, `toBoolean`, `stripHtml`: Flags (boolean)

**In output:**
- String: Text values
- Number: Numeric values (f64)
- Boolean: True/false values
- Array: Nested data structures

---

## Configuration Schema

The JSON Schema is auto-generated from Rust types:

```bash
cargo run --bin gen
```

This creates `krk-schema.json` for IDE autocomplete and validation.

---

## Best Practices

### 1. Respectful Scraping

```yaml
config:
  delay: 2000  # 2 seconds between requests
  timeout: 30
  retries: 2
  headers:
    User-Agent: "Karkinos Bot (+https://yoursite.com/bot)"
```

### 2. Development Workflow

```yaml
config:
  cacheDir: ./.cache
  useCache: true  # Use cache during development
  timeout: 10
```

### 3. Production Scraping

```yaml
config:
  timeout: 60
  retries: 3
  delay: 1000
  headers:
    User-Agent: "Your Bot Name/1.0"
```

### 4. Data Validation

```yaml
data:
  price:
    selector: .price
    default: "0"  # Always have a value
    regex: '\d+\.\d+'
    toNumber: true
```

---

## Performance Considerations

### Parallel Processing

- Uses Rayon for parallel data extraction
- Processes multiple items concurrently
- Automatic CPU core utilization

### Memory Usage

- Caches HTML in memory during processing
- Arc for efficient memory sharing
- Streaming CSV output

### Network Optimization

- Connection reuse within a session
- Configurable timeouts
- Smart retry logic

---

## Examples by Use Case

### E-commerce Product Scraping

```yaml
config:
  url: https://shop.example.com/products
  delay: 1000
data:
  products:
    selector: .product
    data:
      name:
        selector: .name
        stripHtml: true
      price:
        selector: .price
        regex: '\d+\.\d+'
        toNumber: true
      inStock:
        selector: .stock
        toBoolean: true
```

### News Aggregation

```yaml
config:
  urls:
    - https://news1.com
    - https://news2.com
  delay: 2000
  cacheDir: ./news-cache
  useCache: true
data:
  articles:
    selector: article
    data:
      headline:
        selector: h1
      summary:
        selector: .summary
        stripHtml: true
```

### API Data Extraction

```yaml
config:
  url: https://api.example.com/data
  headers:
    Authorization: "Bearer token123"
    Accept: "application/json"
  timeout: 60
  retries: 3
data:
  items:
    selector: .item
    data:
      id:
        selector: .id
      value:
        selector: .value
        toNumber: true
```

---

## Summary of Enhancements

| Feature | Status | Configuration |
|---------|--------|---------------|
| Custom Headers | ✅ | `headers` |
| Timeout Control | ✅ | `timeout` |
| Retry Logic | ✅ | `retries` |
| Proxy Support | ✅ | `proxy` |
| Rate Limiting | ✅ | `delay` |
| Multiple URLs | ✅ | `urls` |
| Response Caching | ✅ | `cacheDir`, `useCache` |
| Regex Extraction | ✅ | `regex` |
| Text Replacement | ✅ | `replace` |
| Case Conversion | ✅ | `uppercase`, `lowercase` |
| HTML Stripping | ✅ | `stripHtml` |
| Default Values | ✅ | `default` |
| Number Conversion | ✅ | `toNumber` |
| Boolean Conversion | ✅ | `toBoolean` |
| CSV Output | ✅ | `-f csv` |
| JSON Output | ✅ | Default |

All features are backward compatible with existing configurations!
