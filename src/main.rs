use anyhow::{anyhow, Context, Result};
use arboard::{Clipboard, ImageData};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use clap::Parser;
use image::{ImageBuffer, Rgba};
use log::{debug, error, info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    time::Duration,
};

// --- Configuration ---

const APP_NAME: &str = "clippy-clippy";
const CONFIG_FILE_NAME: &str = "config.yaml";

#[derive(Deserialize, Debug)]
struct Config {
    api_url: String,
    api_token: String,
    model_name: Option<String>,
    max_tokens: Option<u32>,
    request_timeout_seconds: Option<u64>,
}

fn get_config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("Could not find configuration directory"))?
        .join(APP_NAME);

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .with_context(|| format!("Failed to create config directory at {:?}", config_dir))?;
        info!("Created config directory at: {:?}", config_dir);
    }

    Ok(config_dir.join(CONFIG_FILE_NAME))
}

fn load_config(config_path: &Path) -> Result<Config> {
    if !config_path.exists() {
        // Create a default config file if it doesn't exist
        let default_config_content = r#"# Configuration for clippy-clippy
# Get API URL and Token from your OpenAI-compatible provider
api_url: "https://api.openai.com/v1/chat/completions"
api_token: "YOUR_API_TOKEN_HERE"

# Optional: Specify the model name (defaults to gpt-4-vision-preview if unset)
# model_name: "gpt-4-vision-preview"

# Optional: Set max tokens for the response (defaults to 1024 if unset)
# max_tokens: 1024

# Optional: Set HTTP request timeout in seconds (defaults to 60 if unset)
# request_timeout_seconds: 60
"#;
        fs::write(config_path, default_config_content)
            .with_context(|| format!("Failed to write default config file to {:?}", config_path))?;
        return Err(anyhow!(
            "Configuration file created at {:?}. Please edit it with your API details.",
            config_path
        ));
    }

    let config_content = fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config file from {:?}", config_path))?;

    let config: Config = serde_yaml::from_str(&config_content)
        .with_context(|| format!("Failed to parse YAML config file at {:?}", config_path))?;

    if config.api_token == "YOUR_API_TOKEN_HERE" || config.api_token.is_empty() {
        return Err(anyhow!(
            "Please replace 'YOUR_API_TOKEN_HERE' with your actual API token in {:?}",
            config_path
        ));
    }

    // Apply defaults if not present in the config file
    let config = Config {
        model_name: config.model_name.or(Some("gpt-4-vision-preview".to_string())),
        max_tokens: config.max_tokens.or(Some(1024)),
        request_timeout_seconds: config.request_timeout_seconds.or(Some(60)),
        ..config // Keep other fields as they were
    };


    Ok(config)
}

// --- Command Line Arguments ---

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Generate GitHub Flavored Markdown output (e.g., for tables)
    #[arg(short, long)]
    markdown: bool,

    /// Optional: Path to the configuration file
    #[arg(long)]
    config: Option<PathBuf>,
}

// --- OpenAI API Interaction ---

#[derive(Serialize)]
struct ApiRequest<'a> {
    model: &'a str,
    messages: Vec<Message<'a>>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct Message<'a> {
    role: &'a str,
    content: Vec<Content<'a>>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum Content<'a> {
    #[serde(rename = "text")]
    Text { text: &'a str },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl<'a> },
}

#[derive(Serialize)]
struct ImageUrl<'a> {
    url: Cow<'a, str>, // Use Cow for efficiency, avoids allocation if url is static
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<&'a str>, // Optional: can be "low", "high", "auto"
}

#[derive(Deserialize, Debug)]
struct ApiResponse {
    choices: Vec<Choice>,
    #[serde(default)] // Handle cases where 'usage' might be missing
    usage: Option<Usage>,
    #[serde(default)] // Capture potential errors from the API
    error: Option<ApiError>,
}

#[derive(Deserialize, Debug)]
struct ApiError {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
}


#[derive(Deserialize, Debug)]
struct Choice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ResponseMessage {
    content: Option<String>, // Make content optional, API might return null
}

#[derive(Deserialize, Debug, Default)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

async fn encode_image_to_base64(image_data: ImageData<'_>) -> Result<String> {
    info!(
        "Encoding image ({}x{}) to PNG and then Base64...",
        image_data.width, image_data.height
    );

    // Create an ImageBuffer from the raw RGBA data provided by arboard
    // Note: arboard gives BGRA on windows, RGBA on macos/linux(x11)
    // We assume RGBA here, might need platform-specific handling if BGRA causes issues.
    let image_buffer: Option<ImageBuffer<Rgba<u8>, _>> =
        ImageBuffer::from_raw(
            image_data.width as u32,
            image_data.height as u32,
            image_data.bytes.into_owned(), // Convert Cow<[u8]> to Vec<u8>
        );

    let image_buffer = image_buffer
         .ok_or_else(|| anyhow!("Failed to create image buffer from clipboard data. Data length might not match dimensions, or format might not be RGBA8."))?;

    // Encode the image buffer to PNG format in memory
    let mut png_bytes: Vec<u8> = Vec::new();
    // Use a BufWriter for potentially better performance with large images
    // let writer = BufWriter::new(Cursor::new(&mut png_bytes));
    let mut cursor = Cursor::new(&mut png_bytes);

    image_buffer
        .write_to(&mut cursor, image::ImageOutputFormat::Png)
        .context("Failed to encode image to PNG format")?;

    // Encode the PNG bytes to Base64
    let base64_string = BASE64_STANDARD.encode(&png_bytes);
    debug!("Base64 encoding complete, length: {}", base64_string.len());

    Ok(format!("data:image/png;base64,{}", base64_string))
}

async fn call_openai_api(
    config: &Config,
    base64_image: &str,
    generate_markdown: bool,
) -> Result<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(
            config.request_timeout_seconds.unwrap_or(60), // Already has default from load_config
        ))
        .build()
        .context("Failed to build HTTP client")?;

    let model = config.model_name.as_deref().unwrap_or("gpt-4-vision-preview");

    info!("Using '{}' model for image to text...", model);

    let prompt_text = if generate_markdown {
        "Extract all text from this image accurately. If the image contains tabular data, a list, code, or other structured content, format the output as GitHub Flavored Markdown. Pay attention to formatting details like spacing in tables. Don't use any image related markdown.  Otherwise, return the plain text. Output *only* the extracted text or markdown content and nothing else. Do not include any introductory phrases or explanations.  For bullet points, use hyphens instead of bullet characters, like a normal markdown."
    } else {
        "Extract all text content from this image accurately. Output *only* the extracted text and nothing else. Do not include any introductory phrases."
    };

    let request_payload = ApiRequest {
        model,
        messages: vec![Message {
            role: "user",
            content: vec![
                Content::Text { text: prompt_text },
                Content::ImageUrl {
                    image_url: ImageUrl {
                        url: Cow::Borrowed(base64_image),
                        detail: Some("high"), // Request high detail for better OCR
                    },
                },
            ],
        }],
        max_tokens: config.max_tokens.unwrap_or(1024), // Should have default
    };

    info!("Sending request to API endpoint: {}", config.api_url);
    debug!("Request payload model: {}", request_payload.model);

    let response = client
        .post(&config.api_url)
        .bearer_auth(&config.api_token)
        .json(&request_payload)
        .send()
        .await
        .context("Failed to send request to the API")?;

    let status = response.status();
    debug!("API response status: {}", status);

    // Read the response body text first for better error reporting
    let response_text = response
        .text()
        .await
        .context("Failed to read API response body")?;


    if !status.is_success() {
        // Attempt to parse the error response if possible
         match serde_json::from_str::<ApiResponse>(&response_text) {
             Ok(api_response) if api_response.error.is_some() => {
                 let api_error = api_response.error.unwrap(); // Safe due to check
                 error!("API Error Response: Type: {}, Message: {}", api_error.error_type, api_error.message);
                 return Err(anyhow!("API request failed with status {}: {} ({})", status, api_error.message, api_error.error_type));
             }
             _ => {
                 // If parsing fails or no structured error, return the raw text
                 error!("API Error Response Body: {}", response_text);
                 return Err(anyhow!(
                     "API request failed with status {}. Response body: {}",
                     status,
                     response_text
                 ));
             }
         }
    }

    // Now parse the successful response
    let api_response: ApiResponse = serde_json::from_str(&response_text)
        .with_context(|| format!("Failed to parse successful JSON response from API. Body: {}", response_text))?;


    if let Some(api_error) = api_response.error {
        error!("API returned success status but included an error object: Type: {}, Message: {}", api_error.error_type, api_error.message);
        return Err(anyhow!("API indicated an error despite success status: {} ({})", api_error.message, api_error.error_type));
    }

    if let Some(usage) = api_response.usage {
         info!(
             "API usage: Prompt tokens={}, Completion tokens={}, Total tokens={}",
             usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
         );
     } else {
         warn!("API response did not include usage information.");
     }


    if let Some(choice) = api_response.choices.into_iter().next() {
        info!("Successfully received response from API.");
        debug!(
            "Finish reason: {:?}",
            choice.finish_reason.unwrap_or_else(|| "N/A".to_string())
        );
        // Handle potential null content from API
        match choice.message.content {
            Some(text) => Ok(text),
            None => {
                warn!("API response choice message content was null.");
                Ok(String::new()) // Return empty string if content is null
            }
        }

    } else {
        warn!("API response did not contain any choices/content, although status was success.");
        Err(anyhow!(
            "API response did not contain any choices/content."
        ))
    }
}

// --- Main Execution Logic ---

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (e.g., RUST_LOG=info, clippy_clippy=debug)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let cli = Cli::parse();

    // Determine config path
    let config_path = if let Some(path) = cli.config {
        if !path.exists() {
            return Err(anyhow!("Specified config file does not exist: {:?}", path));
        }
        path
    } else {
        get_config_path()?
    };
    info!("Using configuration file: {:?}", config_path);

    // Load configuration (handles creation/error message if first run)
    let config = match load_config(&config_path) {
         Ok(cfg) => cfg,
         Err(e) => {
             // Check if the error is the specific "please edit" message
             if e.to_string().contains("Please edit it") || e.to_string().contains("Please replace 'YOUR_API_TOKEN_HERE'") {
                 eprintln!("{}", e); // Print the helpful message to stderr
                 // Return Ok here so the program exits cleanly with status 0 after creating/flagging the config
                 // Users expect a non-error exit code for informational messages like this.
                 return Ok(());
             } else {
                 // For other loading errors, return the error to exit with non-zero status
                 return Err(e).context(format!("Failed to load configuration from {:?}", config_path));
             }
         }
     };


    // Initialize clipboard
    // This might fail in a headless CI environment!
    let mut clipboard = match Clipboard::new() {
        Ok(cb) => cb,
        Err(e) => {
            error!("Failed to initialize clipboard: {}. This might happen in environments without a graphical session (like some CI runners).", e);
            return Err(anyhow!("Failed to initialize clipboard: {}", e));
        }
    };
    info!("Clipboard initialized.");

    // Check for image in clipboard
    match clipboard.get_image() {
        Ok(image_data) => {
            info!(
                "Image detected in clipboard ({}x{})",
                image_data.width,
                image_data.height
            );

            // Simple check for empty image data which can happen sometimes
            if image_data.width == 0 || image_data.height == 0 || image_data.bytes.is_empty() {
                 warn!("Clipboard provided image data but it appears empty (0 width/height or no bytes). Skipping.");
                 println!("📋 Clipboard image data is empty. Nothing to process.");
                 return Ok(());
            }


            // Encode image to base64
            let base64_image_data = encode_image_to_base64(image_data)
                .await // <-- Ensure await here too (was already correct, but good to double-check)
                .context("Failed to encode clipboard image")?;

            // Call the API
            info!("⏳ Processing image with AI..."); // User feedback
            let extracted_text = call_openai_api(&config, &base64_image_data, cli.markdown)
                .await
                .context("Failed to get text from API")?;

            println!("{}", extracted_text);

            Ok(())
        }
        Err(arboard::Error::ContentNotAvailable) => {
            info!("No image found in the clipboard.");
            warn!("📋 No image found in the clipboard. Copy an image and try again.");
            Ok(()) // Not an error state, just nothing to do
        }
        Err(e) => {
            // Handle other potential clipboard errors
            error!("Error checking clipboard for image: {}", e);
             if e.to_string().contains("failed to initialize") || e.to_string().contains("no available clipboard provider"){
                 eprintln!("Error: Could not access the system clipboard. Ensure a clipboard manager is running or the environment supports it.");
             }
            Err(anyhow!("Failed to get image from clipboard: {}", e))
        }
    }
}
