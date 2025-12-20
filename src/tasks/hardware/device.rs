use axum::body::Bytes;
use axum::http::HeaderMap;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use firestore::*;
use serde::Deserialize;
use tokio::fs;

// esp will make a req to api
#[derive(Deserialize)]
pub struct HardwareAuthenticate {
    pub userid: String,
}

// POST /hardware/connect/hardware_secrets  + body
pub async fn connect_device_to_user(
    Path(hardware_secrets): Path<String>,
    Json(payload): Json<HardwareAuthenticate>,
) -> impl IntoResponse {
    println!("Connecting device for user: {}", payload.userid);
    println!("Connecting device for hardware: {}", hardware_secrets);
    // Here, you would add logic to associate the device with the user in your database
    Json(serde_json::json!({
        "status": "success",
        "message": format!("Device connected for user {}", payload.userid)
    }))
}

pub async fn recieve_image_bytes_esp32(
    Path(hardware_secrets): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
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
        Ok(_) => Json(serde_json::json!({
            "success": true,
            "message": "Image bytes received and saved",
            "path": full_path,
            "bytes": body.len(),
            "content_type": content_type,
        })),
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
