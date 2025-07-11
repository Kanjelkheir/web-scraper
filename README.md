# Web Scraper

A command-line web scraper written in Rust that extracts page titles, descriptions, and favicons from websites.

## Features

- Extract webpage titles and meta descriptions
- Download and save favicons
- Process individual URLs or batch process from a text file
- Asynchronous operation for improved performance
- Colorized terminal output

## Installation

Make sure you have Rust and Cargo installed. If not, follow the [official Rust installation guide](https://www.rust-lang.org/tools/install).

```bash
# Clone the repository
git clone https://github.com/yourusername/web-scraper.git
cd web-scraper

# Build the project
cargo build --release
```

## Usage

### Scrape a single website

```bash
cargo run -- https://example.com
```

This will:
1. Extract the webpage title and description
2. Download the favicon and save it as `favicon.ico` (or appropriate extension)
3. Display results in the terminal

### Process multiple URLs from a text file

```bash
cargo run -- urls.txt
```

Where `urls.txt` contains one URL per line. The program will:
1. Process each URL sequentially
2. Save favicons with unique filenames based on the URL
3. Display results for each URL in the terminal

## Dependencies

- reqwest: HTTP client for making requests
- tokio: Asynchronous runtime
- url: URL parsing
- bytes: Byte manipulation
- colored: Terminal text coloring
- scraper: HTML parsing

## Error Handling

The scraper includes robust error handling for:
- Invalid URLs
- Missing files
- Network errors
- Invalid HTML
- Missing favicons

## License

[MIT License](LICENSE)