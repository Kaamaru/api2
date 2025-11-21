use axum::{Json, extract::State, response::IntoResponse};
use serde::Deserialize;


// esp will make a req to api
#[derive(Deserialize)]
pub struct HardwareAuthenticate{
    pub userid: String,
}

// POST /hardware/connect + body
pub async fn connect_device_to_user(Json(payload) : Json<HardwareAuthenticate>) -> impl IntoResponse{
    println!("Connecting device for user: {}", payload.userid);
    // Here, you would add logic to associate the device with the user in your database
    Json(serde_json::json!({
        "status": "success",
        "message": format!("Device connected for user {}", payload.userid)
    }))
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
