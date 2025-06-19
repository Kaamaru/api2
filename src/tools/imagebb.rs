use reqwest::multipart;
use std::fs::File;
use std::path::Path;
use std::io::Read;



 // upload_to_imgbb("mahimahi1.jpg", None)
    //     .await
    //     .expect("Failed to upload image to imgbb");

pub async fn upload_to_imgbb(filepath: &str,expirationsecs: Option<u32>) -> Result<String, Box<dyn std::error::Error>> {
   
   let api_key: &str =  "b3e6c9b43817afe282d81d88516eb1df";
   let image_path: &str = filepath;
   
    let file_name = Path::new(image_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("upload.png");

    // Read the image file into memory
    let mut file = File::open(image_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Create multipart form with image binary
    let part = multipart::Part::bytes(buffer)
        .file_name(file_name.to_string())
        .mime_str("image/png")?;
    let form = multipart::Form::new().part("image", part);

    // Build the URL
    let mut url = format!("https://api.imgbb.com/1/upload?key={}", api_key);
    if let Some(exp) = expirationsecs {
        url.push_str(&format!("&expiration={}", exp));
    }

    // Send request
    let client = reqwest::Client::new();
    let response = client.post(&url)
        .multipart(form)
        .send()
        .await?
        .error_for_status()? // Ensures only 2xx are accepted
        .json::<serde_json::Value>()
        .await?;

    // Extract the URL
    if let Some(url) = response["data"]["url"].as_str() {
        Ok(url.to_string())
    } else {
        Err("Upload failed: no image URL returned".into())
    }
}
