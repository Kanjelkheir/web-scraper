use std::env::args;
use std::fs;
use std::io::{ErrorKind, Read};
use std::process;
use colored::Colorize;
use reqwest::Client;
use tokio::fs as other_fs;
use std::path::Path;
use url::Url;

use trpl::Html;
use std::str;

fn main() {
    let arguments: Vec<String> = args().collect();

    match arguments.len() {
        1 => {
            eprintln!("{}", "1 or more arguments required".red());
            process::exit(1);
        },
        2 => {
            let arg = &arguments[1];

            if arg.contains(".txt") {
                let file = fs::File::open(arg);
                match file {
                    Ok(mut file) => {
                        let mut contents = String::new();
                        file.read_to_string(&mut contents).unwrap();

                        let mut urls = Vec::new();
                        for line in contents.lines() {
                            if line.contains("http") && line.contains(".") {
                                urls.push(line.to_string());
                            }
                        }

                        for url in urls {
                            trpl::run(async {
                                let client = Client::new();
                                let title = get_title(&url).await;
                                let description = get_description(&url).await;

                                match extract_favicon(&client, &url).await {
                                    Ok(favicon) => {
                                        let output_filename = format!("favicon_{}.{}",
                                            url.replace(|c: char| !c.is_alphanumeric(), "_"),
                                            favicon.file_extension
                                        );
                                        match other_fs::write(&output_filename, &favicon.data).await {
                                            Ok(_) => {
                                                println!("{}", format!("Favicon saved as {}", output_filename).green());
                                            },
                                            Err(e) => {
                                                eprintln!("{}", format!("Error saving favicon to {}: {}", output_filename, e).red());
                                            }
                                        }
                                    },
                                    Err(e) => {
                                        eprintln!("{}", format!("Error extracting favicon for {}: {}", url, e).red());
                                    }
                                }

                                println!("{url} title is {}\nDescription is: {}", title, description);
                            })
                        }
                    },
                    Err(error) => match error.kind() {
                        ErrorKind::NotFound => {
                            eprintln!("{}", "File not found".red());
                            process::exit(2);
                        },
                        _ => {
                            eprintln!("{}", format!("Unknown error reading file: {}", error).red());
                            process::exit(3);
                        },
                    },
                }
            } else {
                let url = arg;
                if url.contains("http") && url.contains(".") {
                    trpl::run(async {
                        let client = Client::new();
                        let title = get_title(&url).await;
                        let desc = get_description(&url).await;

                        match extract_favicon(&client, &url).await {
                            Ok(favicon) => {
                                let path_string = format!("favicon.{}", favicon.file_extension);
                                match other_fs::write(&path_string, &favicon.data).await {
                                    Ok(_) => {
                                        println!("{}", format!("Favicon saved as {}", path_string).green());
                                    },
                                    Err(e) => {
                                        eprintln!("{}", format!("Error saving favicon to {}: {}", path_string, e).red());
                                    }
                                }
                            },
                            Err(e) => {
                                eprintln!("{}", format!("Error extracting favicon: {}", e).red());
                            }
                        }

                        println!("{url} title is {}\nDescription is: {}", title, desc);
                    })
                } else {
                    eprintln!("{}", "Invalid URL format. Must contain 'http' and a '.'".red());
                    process::exit(1);
                }
            }
        },
        _ => {
            eprintln!("{}", "Too many arguments provided. Expected 1 or 2.".red());
            process::exit(1);
        },
    }
}

#[derive(Clone)]
struct FaviconStruct {
    url: String,
    file_extension: String,
    data: Vec<u8>,
}

async fn get_title(url: &str) -> String {
    let response = match trpl::get(url).await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("{}", format!("Error fetching title for {}: {}", url, e).red());
            return "Error fetching title".to_string();
        }
    };

    let response_text = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            eprintln!("{}", format!("Error reading response text for title {}: {}", url, e).red());
            return "Error reading title response".to_string();
        }
    };

    Html::parse(&response_text).select_first("title")
        .map(|title| title.inner_html())
        .unwrap_or_else(|| "No title found".to_string())
}

async fn get_description(url: &str) -> String {
    let response = match trpl::get(url).await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("{}", format!("Error fetching description for {}: {}", url, e).red());
            return "Error fetching description".to_string();
        }
    };

    let response_text = match response.text().await {
        Ok(text) => text,
        Err(e) => {
            eprintln!("{}", format!("Error reading response text for description {}: {}", url, e).red());
            return "Error reading description response".to_string();
        }
    };

    Html::parse(&response_text).select_first("meta[name=description]")
        .and_then(|desc| desc.attr("content"))
        .unwrap_or("No description available")
        .to_string()
}

async fn extract_favicon(client: &Client, url_str: &str) -> Result<FaviconStruct, Box<dyn std::error::Error>> {
    // First, check if the URL is already a direct link to an image
    let favicon_url_to_fetch = if url_str.ends_with(".ico") || url_str.ends_with(".png") || url_str.ends_with(".svg") {
        url_str.to_string()
    } else {
        // If not a direct image URL, try to find favicon in HTML first
        let parsed_url = Url::parse(url_str)?;
        let base_domain = format!("{}://{}", parsed_url.scheme(), parsed_url.host_str().unwrap_or_default());

        // Try to get the HTML content of the page
        let html_response = client.get(url_str).send().await?;
        if html_response.status().is_success() {
            let html_text = html_response.text().await?;
            let html = trpl::Html::parse(&html_text);

            // Try different favicon selectors in order of preference
            let favicon_selectors = [
                "link[rel='icon']",
                "link[rel='shortcut icon']",
                "link[rel='apple-touch-icon']",
                "link[rel='apple-touch-icon-precomposed']",
                "link[rel='fluid-icon']",
                "link[rel='mask-icon']"
            ];

            for selector in favicon_selectors {
                if let Some(favicon_element) = html.select_first(selector) {
                    if let Some(href) = favicon_element.attr("href") {
                        // Convert relative URLs to absolute
                        if href.starts_with("http") {
                            println!("Found favicon in HTML with selector {}: {}", selector, href);
                            return fetch_favicon(client, href).await;
                        } else if href.starts_with("//") {
                            // Protocol-relative URL
                            let abs_url = format!("{}:{}", parsed_url.scheme(), href);
                            println!("Found favicon in HTML with selector {}: {}", selector, abs_url);
                            return fetch_favicon(client, &abs_url).await;
                        } else if href.starts_with("/") {
                            // Root-relative URL
                            let abs_url = format!("{}{}", base_domain, href);
                            println!("Found favicon in HTML with selector {}: {}", selector, abs_url);
                            return fetch_favicon(client, &abs_url).await;
                        } else {
                            // Fully relative URL
                            let abs_url = format!("{}/{}", base_domain, href);
                            println!("Found favicon in HTML with selector {}: {}", selector, abs_url);
                            return fetch_favicon(client, &abs_url).await;
                        }
                    }
                }
            }
        }

        // Fallback to standard favicon.ico location
        let base_url = format!("{}/favicon.ico", base_domain);
        base_url
    };

    // If we get here, we're using the fallback favicon.ico or direct URL
    fetch_favicon(client, &favicon_url_to_fetch).await
}

async fn fetch_favicon(client: &Client, favicon_url: &str) -> Result<FaviconStruct, Box<dyn std::error::Error>> {
    println!("\n--- Attempting to fetch favicon from: {} ---", favicon_url);
    let response = client.get(favicon_url).send().await?;

    // Store the important response data before consuming the response
    let final_url = response.url().to_string();
    let status = response.status();

    println!("Final URL (after redirects): {}", final_url);
    println!("HTTP Status: {}", status);

    if !status.is_success() {
        return Err(format!(
            "Failed to fetch favicon: HTTP status {} for URL {}",
            status,
            final_url
        ).into());
    }

    // Get content-type before consuming the response
    let content_type = response.headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    println!("Content-Type: {}", content_type);

    // Now consume the response to get the bytes
    let data = response.bytes().await?.to_vec();
    println!("Downloaded data size: {} bytes", data.len());

    let mut file_extension = "bin".to_string();

    // If content is empty, error out
    if data.is_empty() {
        return Err("Empty response received for favicon".into());
    }

    if content_type.contains("image/png") {
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            file_extension = "png".to_string();
        } else {
            // Be less strict about magic bytes, some servers misreport content types
            file_extension = "png".to_string();
            println!("Warning: Content-Type is image/png but magic bytes don't match exactly");
        }
    } else if content_type.contains("image/x-icon") || content_type.contains("image/vnd.microsoft.icon") {
        // ICO files can have different headers
        file_extension = "ico".to_string();
    } else if content_type.contains("image/svg+xml") {
        if std::str::from_utf8(&data).map(|s| s.contains("<svg")).unwrap_or(false) {
            file_extension = "svg".to_string();
        } else {
            // Be less strict with SVG detection
            file_extension = "svg".to_string();
            println!("Warning: Content-Type is image/svg+xml but <svg> tag not found near start");
        }
    } else if content_type.contains("image/jpeg") {
        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            file_extension = "jpg".to_string();
        } else {
            // Be less strict about JPEG magic bytes
            file_extension = "jpg".to_string();
            println!("Warning: Content-Type is image/jpeg but magic bytes don't match exactly");
        }
    } else if content_type.contains("image/gif") {
        if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
            file_extension = "gif".to_string();
        } else {
            // Be less strict about GIF magic bytes
            file_extension = "gif".to_string();
            println!("Warning: Content-Type is image/gif but magic bytes don't match exactly");
        }
    } else if content_type.contains("text/html") || content_type.contains("application/json") {
        // It's likely an error page, but we'll try to detect image format by magic bytes as a last resort
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            file_extension = "png".to_string();
            println!("Warning: Content-Type is {}, but detected PNG by magic bytes", content_type);
        } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            file_extension = "jpg".to_string();
            println!("Warning: Content-Type is {}, but detected JPEG by magic bytes", content_type);
        } else if std::str::from_utf8(&data).map(|s| s.contains("<svg")).unwrap_or(false) {
            file_extension = "svg".to_string();
            println!("Warning: Content-Type is {}, but detected SVG content", content_type);
        } else {
            return Err(format!("Expected image, but received {} content. This is likely an error page or API response.", content_type).into());
        }
    } else {
        // Try to determine extension from URL
        let final_url_parsed = Url::parse(&final_url)?;
        if let Some(ext) = final_url_parsed
            .path_segments()
            .and_then(|segments| segments.last())
            .and_then(|filename| Path::new(filename).extension())
            .and_then(|ext_os_str| ext_os_str.to_str())
        {
            let lower_ext = ext.to_lowercase();
            if ["png", "ico", "svg", "jpg", "jpeg", "gif"].contains(&lower_ext.as_str()) {
                file_extension = lower_ext;
            }
        }

        // If still "bin", try to determine by magic bytes
        if file_extension == "bin" {
            if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                file_extension = "png".to_string();
            } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
                file_extension = "jpg".to_string();
            } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
                file_extension = "gif".to_string();
            } else if std::str::from_utf8(&data).map(|s| s.contains("<svg")).unwrap_or(false) {
                file_extension = "svg".to_string();
            } else {
                file_extension = "ico".to_string(); // Default to ico as a last resort for unknown image types
            }
        }
    }

    println!("Detected File Extension: {}", file_extension);

    Ok(FaviconStruct {
        url: final_url,
        file_extension,
        data,
    })
}

mod trpl {
    use reqwest::Response as ReqwestResponse;
    use tokio::runtime::Runtime;

    pub struct Response {
        pub inner: ReqwestResponse,
    }

    impl Response {
        pub async fn text(self) -> Result<String, reqwest::Error> {
            self.inner.text().await
        }
        pub async fn bytes(self) -> Result<bytes::Bytes, reqwest::Error> {
            self.inner.bytes().await
        }
    }

    pub struct Html(scraper::Html);

    impl Html {
        pub fn parse(html: &str) -> Self {
            Html(scraper::Html::parse_document(html))
        }
        pub fn select_first(&self, selector: &str) -> Option<scraper::ElementRef> {
            let selector = scraper::Selector::parse(selector).unwrap();
            self.0.select(&selector).next()
        }
    }

    pub async fn get(url: &str) -> Result<Response, reqwest::Error> {
        let reqwest_response = reqwest::get(url).await?;
        Ok(Response { inner: reqwest_response })
    }

    pub fn run<F: std::future::Future>(future: F) -> F::Output {
        let rt = Runtime::new().unwrap();
        rt.block_on(future)
    }
}
