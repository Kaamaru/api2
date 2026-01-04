use axum::body::Bytes;
use axum::http::HeaderMap;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use chrono::{Utc, Duration};
use firestore::*;
use serde::Deserialize;
use tokio::{fs, task};
use reqwest::Client;
use serde_json::json;
use axum::http::StatusCode;
use crate::FirebaseConfig;
use super::tractor::{RunnableOnnxModel, create_input_image};
const   LABELS_PATH: &str = "models/tractor_result_by_index.txt";

// esp will make a req to api
#[derive(Deserialize)]
pub struct HardwareAuthenticate {
    pub userid: String,
}

// Get userid from hardware_secrets by querying devices collection
async fn get_userid_from_hardware(
    hardware_secrets: &str,
    firebase_config: &FirebaseConfig,
) -> Result<String, String> {
    let client = Client::new();
    let device_url = format!(
        "{}/devices/{}",
        firebase_config.firestore_url(),
        hardware_secrets
    );

    let response = client
        .get(&device_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch device: {}", e))?;

    if !response.status().is_success() {
        return Err("Device not found".to_string());
    }

    let body = response
        .text()
        .await
        .map_err(|e| format!("Failed to read device response: {}", e))?;

    let device_doc: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse device document: {}", e))?;

    // Extract userid from fields.userid.stringValue structure
    let userid = device_doc
        .get("fields")
        .and_then(|f| f.get("userid"))
        .and_then(|u| u.get("stringValue"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "userid field not found in device".to_string())?;

    Ok(userid)
}

// Add schedule from classification result
async fn add_schedule_from_classification(
    userid: &str,
    category: &str,
    firebase_config: &FirebaseConfig,
) -> Result<(), String> {
    let client = Client::new();

    // Generate unique schedule ID
    let now = Utc::now();
    let new_id = format!(
        "schedule_{}",
        now.format("%Y%m%dT%H%M%SZ").to_string()
    );

    // Get today's date in YYYY-MM-DD format
    let date = now.format("%Y-%m-%d").to_string();
    
    // Get current time in HH-MM format (using current hour and minute)
    let time = now.format("%H-%M").to_string();
    
    // Expiry is tomorrow
    let expiry_date = now + Duration::days(1);
    let expiry = expiry_date.format("%Y-%m-%d").to_string();

    let patch_url = format!(
        "{}/users/{}?updateMask.fieldPaths=schedules.{}",
        firebase_config.firestore_url(),
        userid,
        new_id
    );

    // Build schedule payload
    let firestore_payload = json!({
        "fields": {
            "schedules": {
                "mapValue": {
                    "fields": {
                        new_id.clone(): {
                            "mapValue": {
                                "fields": {
                                    "uid": { "stringValue": new_id },
                                    "name": { "stringValue": category },
                                    "time": { "stringValue": time },
                                    "date": { "stringValue": date },
                                    "expiry": { "stringValue": expiry },
                                    "notify": { "booleanValue": false },
                                    "category": { "stringValue": category }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let response = client
        .patch(&patch_url)
        .json(&firestore_payload)
        .send()
        .await
        .map_err(|e| format!("Failed to patch schedule: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Firestore PATCH failed: {} - {}",
            status, body
        ));
    }

    Ok(())
}

// POST /hardware/connect/hardware_secrets  + body
pub async fn connect_device_to_user(
    Path(hardware_secrets): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(payload): Json<HardwareAuthenticate>,
) -> impl IntoResponse {
    let client = Client::new();
    let patch_url = format!(
        "{}/devices/{}?updateMask.fieldPaths=userid",
        firebase_config.firestore_url(),
        hardware_secrets
    );

    let firestore_payload = json!({
        "fields": {
            "userid": { "stringValue": payload.userid.clone() }
        }
    });

    let resp = match client.patch(&patch_url).json(&firestore_payload).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to PATCH Firestore: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Failed to update device" })),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_else(|_| "".to_string());
        eprintln!("Firestore PATCH failed: {} - {}", status, body);
        return (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": "Firestore update failed", "status": status.as_u16() })),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "message": format!("Device connected for user {}", payload.userid),
            "hardware_id": hardware_secrets,
            "userid": payload.userid
        })),
    )
        .into_response()
}   

pub async fn recieve_image_bytes_esp32(
    Path(hardware_secrets): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    println!(
        "Received image bytes from hardware: {}, size: {} bytes",
        hardware_secrets,
        body.len()
    );  
    // Determine content-type and file extension
    let content_type = headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream");

    let ext = match content_type {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        _ => "bin",
    };

    // Build destination path: uploads/<userid>/esp32_<timestamp>.<ext>
    let root = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "uploads".to_string());
    let user_dir = format!("{}/{}", root, hardware_secrets);

    if let Err(e) = fs::create_dir_all(&user_dir).await {
        return Json(serde_json::json!({
            "success": false,
            "error": format!("Failed to create directory: {}", e)
        }));
    }

    let ts = Utc::now().format("%Y%m%dT%H%M%S%.3fZ");
    let filename = format!("esp32_{}.{}", ts, ext);
    let full_path = format!("{}/{}", user_dir, filename);

    match fs::write(&full_path, &body).await {
        Ok(_) => {
            let image_for_classify = full_path.clone();

            let classification_result = task::spawn_blocking(move || {
                // Load the ONNX model
                let model = RunnableOnnxModel::create_tract_model("models/mobilenetv2-7.onnx");
                
                // Create tensor from image
                let input_image = create_input_image(&image_for_classify);
                
                // Run the model
                model.run_model(vec![input_image])
                    .map(|results| {
                        // Get the top result (confidence score and class index)
                        if let Some((confidence, class_index)) = results.first() {
                            let label = std::fs::read_to_string(LABELS_PATH)
                                .ok()
                                .and_then(|contents| {
                                    let idx = (*class_index).max(0) as usize;
                                    contents.lines().nth(idx-1).map(|s| s.trim().to_string())
                                });

                            let label_value = label.unwrap_or_else(|| "Unknown label".to_string());

                            (
                                format!("Class: {}, Confidence: {:.4}, Label: {}", class_index, confidence, label_value),
                                label_value,
                            )
                        } else {
                            ("No classification result".to_string(), "No label".to_string())
                        }
                    })
                    .map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| format!("Failed to join classification task: {}", e))
            .and_then(|res| res);

            let (classification, classification_result, classification_error) = match classification_result {
                Ok((display, label)) => (Some(display), Some(label), None),
                Err(err) => {
                    eprintln!("Image classification failed: {}", err);
                    (None, None, Some(err))
                }
            };

            println!("Found Type {}", classification_result.clone().unwrap());

            // Use classification result to create a schedule in Firestore
            let mut schedule_created = false;
            let mut schedule_error = None;

            if let Some(label) = &classification_result {
                // Get userid from hardware_secrets
                match get_userid_from_hardware(&hardware_secrets, &firebase_config).await {
                    Ok(userid) => {
                        // Add schedule with classification result as name and category
                        match add_schedule_from_classification(&userid, label, &firebase_config).await {
                            Ok(_) => {
                                schedule_created = true;
                                println!("✅ Schedule created for user {} with category {}", userid, label);
                            }
                            Err(err) => {
                                schedule_error = Some(err.clone());
                                eprintln!("Failed to create schedule: {}", err);
                            }
                        }
                    }
                    Err(err) => {
                        schedule_error = Some(err.clone());
                        eprintln!("Failed to get userid: {}", err);
                    }
                }
            }

            Json(serde_json::json!({
                "success": true,
                "message": "Image bytes received and saved",
                "path": full_path,
                "bytes": body.len(),
                "content_type": content_type,
                "classification": classification,
                "classification_result": classification_result,
                "classification_error": classification_error,
                "schedule_created": schedule_created,
                "schedule_error": schedule_error,
            }))
        }
        Err(e) => Json(serde_json::json!({
            "success": false,
            "error": format!("Failed to write file: {}", e)
        })),
    }
}

// pub async fn firestore_add_schedule(
//     Path(uuid): Path<String>,
//     Json(request): Json<AddScheduleRequest>,
// ) -> impl IntoResponse {
//     println!("firestore_add_schedule");

//     let client = Client::new();

//     let new_id = format!("schedule_{}",generate_unique_id());
//     // PATCH URL with updateMask to add schedules.itemX
//     let patch_url = format!(
//         "https://firestore.googleapis.com/v1/projects/uringredients/databases/(default)/documents/users/{}?updateMask.fieldPaths=schedules.{}",
//         uuid,
//         new_id
//     );
//     println!("PATCH URL: {}", patch_url);
//     // Firestore payload to insert new item into 'schedules'
//     let firestore_payload = json!({
//         "fields": {
//             "schedules": {
//                 "mapValue": {
//                     "fields": {
//                         new_id.clone(): {
//                             "mapValue": {
//                                 "fields": {
//                                     "uid": { "stringValue": new_id.clone() },
//                                     "name": { "stringValue": request.name },
//                                     "time": { "stringValue": request.time },
//                                     "date": { "stringValue": request.date },
//                                     "expiry": { "stringValue": request.expiry }
//                                 }
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//     });

//     let patch_response = match client
//         .patch(&patch_url)
//         .json(&firestore_payload)
//         .send()
//         .await
//     {
//         Ok(response) => response,
//         Err(e) => {
//             eprintln!("❌ Failed to patch schedule: {}", e);
//             return Json(json!({ "error": "Failed to patch schedule" })).into_response();
//         }
//     };

//     if !patch_response.status().is_success() {
//         let status = patch_response.status();
//         let body = patch_response.text().await.unwrap_or_default();
//         eprintln!("⚠️ Firestore PATCH failed: {status}\n{body}");
//         return Json(json!({ "error": "Firestore update failed", "details": body }))
//             .into_response();
//     }

//     // Optional: Return confirmation
//     Json(json!({
//         "success": true,
//         "message": format!("Schedwrwrwrwrwule '{}' added successfully", request.name),
//         "schedule": {
//             "name": request.name,
//             "time": request.time,
//             "date": request.date,
//             "expiry": request.expiry,
//         },
//         "updated_at": chrono::Utc::now().to_rfc3339()
//     }))
//     .into_response()
// }
