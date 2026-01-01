fn is_jpg_or_png(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".png")
}

use crate::tasks::hardware::tractor::*;
use axum::body::Bytes;
use axum::{extract::Multipart, http::StatusCode, response::IntoResponse};
use std::fs::{self, File};
use std::io::Write;
use std::vec;
pub async fn dev_get() -> impl IntoResponse {
    "Dev Tools Endpoint"
}
pub const MODEL_PATH: &str = "models/mobilenetv2-7.onnx";

pub async fn dev_multipart(mut multipart: Multipart) -> impl IntoResponse {
    // Iterate over the fields in the multipart form
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let file_name = field.file_name().map(|s| s.to_string());
        let content_type = field.content_type().map(|s| s.to_string());
        let data = field.bytes().await.unwrap();

        println!(
            "Name: {}, Filename: {:?}, Content-Type: {:?}",
            name, file_name, content_type
        );

        // Check if the field is an image and has a filename
        if let Some(ref fname) = file_name {
            if is_jpg_or_png(fname) {
                let path = format!("uploads/dev/multipart/{}", fname);
                match File::create(&path) {
                    Ok(mut file) => {
                        if let Err(e) = file.write_all(&data) {
                            eprintln!("Failed to write file: {}", e);
                            return StatusCode::INTERNAL_SERVER_ERROR;
                        }
                        println!("Image saved to {}", path);
                    }
                    Err(e) => {
                        eprintln!("Failed to create file: {}", e);
                        return StatusCode::INTERNAL_SERVER_ERROR;
                    }
                }
            }
        }
    }
    StatusCode::CREATED
}

// Verify the correct path to RunnableOnnxModel based on your crate structure
// use crate::task::hardware::tractor::RunnableOnnxModel;

pub async fn dev_classify(body: Bytes) -> impl IntoResponse {
    let dir = "uploads/dev/classify";
    if let Err(e) = fs::create_dir_all(dir) {
        eprintln!("Failed to create dir: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    let data = body;
    if data.is_empty() {
        return StatusCode::BAD_REQUEST;
    }

    let ext = if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "jpg"
    } else if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        "png"
    } else {
        eprintln!("Unknown image format");
        return StatusCode::UNSUPPORTED_MEDIA_TYPE;
    };

    let path = format!("{}/{}.{}", dir, uuid::Uuid::new_v4(), ext);
    match fs::File::create(&path) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(&data) {
                eprintln!("Failed to write file: {}", e);
                return StatusCode::INTERNAL_SERVER_ERROR;
            }
            println!("Image saved: {}", path);

            dev_classify_image(format!("{}", path)).await;

            StatusCode::CREATED
        }
        Err(e) => {
            eprintln!("Failed to create file: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

pub async fn dev_classify_image(image_path: String) {
    println!("Classifying image: {}", image_path);
    let model = RunnableOnnxModel::create_tract_model(MODEL_PATH);

    let image_list = vec![create_input_image(image_path.as_str())];
    match model.run_model(image_list){
        Ok(result) => {
            println!("Classification result for {}: {:?}", image_path, result);
        }
        Err(e) => {
            eprintln!("Error running model: {}", e);
        }
    };
}
