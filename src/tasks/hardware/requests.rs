use axum::{
    extract::{Path, State},
    response::{IntoResponse, Json},
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::FirebaseConfig;


pub async fn add_device_record(
    Path((userid, hardware_secrets)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let client = Client::new();
    
    let patch_url = format!(
        "{}/users/{}?updateMask.fieldPaths=hardware.devices",
        firebase_config.firestore_url(),
        userid
    );

    // Get current devices first to append
    let get_url = format!("{}/users/{}", firebase_config.firestore_url(), userid);
    
    match client.get(&get_url).send().await {
        Ok(resp) => {
            if let Ok(body) = resp.text().await {
                // Parse existing hardware to get current devices
                if let Ok(current_data) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(fields) = current_data.get("fields") {
                        let mut devices = Vec::new();
                        
                        if let Some(hardware) = fields.get("hardware") {
                            if let Some(map_value) = hardware.get("mapValue") {
                                if let Some(hw_fields) = map_value.get("fields") {
                                    if let Some(devices_array) = hw_fields.get("devices") {
                                        if let Some(array_value) = devices_array.get("arrayValue") {
                                            if let Some(values) = array_value.get("values") {
                                                if let Ok(vals) = serde_json::from_value::<Vec<serde_json::Value>>(values.clone()) {
                                                    devices = vals;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Add new device
                        devices.push(json!({
                            "stringValue": hardware_secrets
                        }));
                        
                        // Send update
                        let update_body = json!({
                            "fields": {
                                "hardware": {
                                    "mapValue": {
                                        "fields": {
                                            "devices": {
                                                "arrayValue": {
                                                    "values": devices
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        });
                        
                        match client.patch(&patch_url)
                            .header("Content-Type", "application/json")
                            .json(&update_body)
                            .send()
                            .await
                        {
                            Ok(_) => Json(json!({"success": true, "message": "Device added"})).into_response(),
                            Err(e) => {
                                eprintln!("Failed to update device: {}", e);
                                Json(json!({"success": false, "error": "Failed to update"})).into_response()
                            }
                        }
                    } else {
                        Json(json!({"success": false, "error": "Invalid user data"})).into_response()
                    }
                } else {
                    Json(json!({"success": false, "error": "Failed to parse user data"})).into_response()
                }
            } else {
                Json(json!({"success": false, "error": "Failed to read response"})).into_response()
            }
        }
        Err(e) => {
            eprintln!("Failed to fetch user: {}", e);
            Json(json!({"success": false, "error": "Failed to fetch user"})).into_response()
        }
    }
}

pub async fn add_request_record(
    Path((userid, hardware_secrets)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let client = Client::new();
    
    let patch_url = format!(
        "{}/users/{}?updateMask.fieldPaths=hardware.requests",
        firebase_config.firestore_url(),
        userid
    );

    // Get current requests first to append
    let get_url = format!("{}/users/{}", firebase_config.firestore_url(), userid);
    
    match client.get(&get_url).send().await {
        Ok(resp) => {
            if let Ok(body) = resp.text().await {
                // Parse existing hardware to get current requests
                if let Ok(current_data) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(fields) = current_data.get("fields") {
                        let mut requests = Vec::new();
                        
                        if let Some(hardware) = fields.get("hardware") {
                            if let Some(map_value) = hardware.get("mapValue") {
                                if let Some(hw_fields) = map_value.get("fields") {
                                    if let Some(requests_array) = hw_fields.get("requests") {
                                        if let Some(array_value) = requests_array.get("arrayValue") {
                                            if let Some(values) = array_value.get("values") {
                                                if let Ok(vals) = serde_json::from_value::<Vec<serde_json::Value>>(values.clone()) {
                                                    requests = vals;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Add new request
                        requests.push(json!({
                            "stringValue": hardware_secrets
                        }));
                        
                        // Send update
                        let update_body = json!({
                            "fields": {
                                "hardware": {
                                    "mapValue": {
                                        "fields": {
                                            "requests": {
                                                "arrayValue": {
                                                    "values": requests
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        });
                        
                        match client.patch(&patch_url)
                            .header("Content-Type", "application/json")
                            .json(&update_body)
                            .send()
                            .await
                        {
                            Ok(_) => Json(json!({"success": true, "message": "Request added"})).into_response(),
                            Err(e) => {
                                eprintln!("Failed to update request: {}", e);
                                Json(json!({"success": false, "error": "Failed to update"})).into_response()
                            }
                        }
                    } else {
                        Json(json!({"success": false, "error": "Invalid user data"})).into_response()
                    }
                } else {
                    Json(json!({"success": false, "error": "Failed to parse user data"})).into_response()
                }
            } else {
                Json(json!({"success": false, "error": "Failed to read response"})).into_response()
            }
        }
        Err(e) => {
            eprintln!("Failed to fetch user: {}", e);
            Json(json!({"success": false, "error": "Failed to fetch user"})).into_response()
        }
    }
}
