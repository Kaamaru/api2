use axum::{
    extract::{Path, State, Json},
    response::IntoResponse,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::FirebaseConfig;


// Accept a JSON array of hardware codes to add to devices
pub async fn add_device_record(
    Path(userid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(codes): Json<Vec<String>>,
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
                        
                        // Append provided codes
                        for code in codes {
                            devices.push(json!({ "stringValue": code }));
                        }
                        
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
                            Ok(_) => Json(json!({"success": true, "message": "Devices updated"})).into_response(),
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

// Fetch devices array
pub async fn get_device_records(
    Path(userid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let client = Client::new();
    let get_url = format!("{}/users/{}", firebase_config.firestore_url(), userid);

    match client.get(&get_url).send().await {
        Ok(resp) => match resp.text().await {
            Ok(body) => {
                if let Ok(current_data) = serde_json::from_str::<serde_json::Value>(&body) {
                    let mut devices: Vec<String> = Vec::new();
                    if let Some(fields) = current_data.get("fields") {
                        if let Some(hardware) = fields.get("hardware") {
                            if let Some(map_value) = hardware.get("mapValue") {
                                if let Some(hw_fields) = map_value.get("fields") {
                                    if let Some(devices_array) = hw_fields.get("devices") {
                                        if let Some(array_value) = devices_array.get("arrayValue") {
                                            if let Some(values) = array_value.get("values") {
                                                if let Ok(vals) = serde_json::from_value::<Vec<serde_json::Value>>(values.clone()) {
                                                    for v in vals {
                                                        if let Some(sv) = v.get("stringValue").and_then(|x| x.as_str()) {
                                                            devices.push(sv.to_string());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Json(json!({"devices": devices}))
                } else {
                    Json(json!({"error": "Failed to parse user data"}))
                }
            }
            Err(_) => Json(json!({"error": "Failed to read response"})),
        },
        Err(_) => Json(json!({"error": "Failed to fetch user"})),
    }
}

// Delete provided codes from devices array
pub async fn delete_device_records(
    Path(userid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(codes): Json<Vec<String>>,
) -> impl IntoResponse {
    let client = Client::new();
    let get_url = format!("{}/users/{}", firebase_config.firestore_url(), userid);
    let patch_url = format!(
        "{}/users/{}?updateMask.fieldPaths=hardware.devices",
        firebase_config.firestore_url(),
        userid
    );

    match client.get(&get_url).send().await {
        Ok(resp) => match resp.text().await {
            Ok(body) => {
                if let Ok(current_data) = serde_json::from_str::<serde_json::Value>(&body) {
                    let mut devices: Vec<String> = Vec::new();
                    if let Some(fields) = current_data.get("fields") {
                        if let Some(hardware) = fields.get("hardware") {
                            if let Some(map_value) = hardware.get("mapValue") {
                                if let Some(hw_fields) = map_value.get("fields") {
                                    if let Some(devices_array) = hw_fields.get("devices") {
                                        if let Some(array_value) = devices_array.get("arrayValue") {
                                            if let Some(values) = array_value.get("values") {
                                                if let Ok(vals) = serde_json::from_value::<Vec<serde_json::Value>>(values.clone()) {
                                                    for v in vals {
                                                        if let Some(sv) = v.get("stringValue").and_then(|x| x.as_str()) {
                                                            devices.push(sv.to_string());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let to_remove: std::collections::HashSet<String> = codes.into_iter().collect();
                    let filtered: Vec<serde_json::Value> = devices
                        .into_iter()
                        .filter(|d| !to_remove.contains(d))
                        .map(|d| json!({"stringValue": d}))
                        .collect();

                    let update_body = json!({
                        "fields": {
                            "hardware": {
                                "mapValue": {
                                    "fields": {
                                        "devices": {
                                            "arrayValue": {
                                                "values": filtered
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });

                    match client
                        .patch(&patch_url)
                        .header("Content-Type", "application/json")
                        .json(&update_body)
                        .send()
                        .await
                    {
                        Ok(_) => Json(json!({"success": true})),
                        Err(_) => Json(json!({"success": false, "error": "Failed to update"})),
                    }
                } else {
                    Json(json!({"error": "Failed to parse user data"}))
                }
            }
            Err(_) => Json(json!({"error": "Failed to read response"})),
        },
        Err(_) => Json(json!({"error": "Failed to fetch user"})),
    }
}

// Accept a JSON array of hardware codes to add to requests
pub async fn add_request_record(
    Path(userid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(codes): Json<Vec<String>>,
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
                        
                        // Append provided codes
                        for code in codes {
                            requests.push(json!({ "stringValue": code }));
                        }
                        
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
                            Ok(_) => Json(json!({"success": true, "message": "Requests updated"})).into_response(),
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

// Fetch requests array
pub async fn get_request_records(
    Path(userid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let client = Client::new();
    let get_url = format!("{}/users/{}", firebase_config.firestore_url(), userid);

    match client.get(&get_url).send().await {
        Ok(resp) => match resp.text().await {
            Ok(body) => {
                if let Ok(current_data) = serde_json::from_str::<serde_json::Value>(&body) {
                    let mut requests: Vec<String> = Vec::new();
                    if let Some(fields) = current_data.get("fields") {
                        if let Some(hardware) = fields.get("hardware") {
                            if let Some(map_value) = hardware.get("mapValue") {
                                if let Some(hw_fields) = map_value.get("fields") {
                                    if let Some(requests_array) = hw_fields.get("requests") {
                                        if let Some(array_value) = requests_array.get("arrayValue") {
                                            if let Some(values) = array_value.get("values") {
                                                if let Ok(vals) = serde_json::from_value::<Vec<serde_json::Value>>(values.clone()) {
                                                    for v in vals {
                                                        if let Some(sv) = v.get("stringValue").and_then(|x| x.as_str()) {
                                                            requests.push(sv.to_string());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Json(json!({"requests": requests}))
                } else {
                    Json(json!({"error": "Failed to parse user data"}))
                }
            }
            Err(_) => Json(json!({"error": "Failed to read response"})),
        },
        Err(_) => Json(json!({"error": "Failed to fetch user"})),
    }
}

// Delete provided codes from requests array
pub async fn delete_request_records(
    Path(userid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(codes): Json<Vec<String>>,
) -> impl IntoResponse {
    let client = Client::new();
    let get_url = format!("{}/users/{}", firebase_config.firestore_url(), userid);
    let patch_url = format!(
        "{}/users/{}?updateMask.fieldPaths=hardware.requests",
        firebase_config.firestore_url(),
        userid
    );

    match client.get(&get_url).send().await {
        Ok(resp) => match resp.text().await {
            Ok(body) => {
                if let Ok(current_data) = serde_json::from_str::<serde_json::Value>(&body) {
                    let mut requests: Vec<String> = Vec::new();
                    if let Some(fields) = current_data.get("fields") {
                        if let Some(hardware) = fields.get("hardware") {
                            if let Some(map_value) = hardware.get("mapValue") {
                                if let Some(hw_fields) = map_value.get("fields") {
                                    if let Some(requests_array) = hw_fields.get("requests") {
                                        if let Some(array_value) = requests_array.get("arrayValue") {
                                            if let Some(values) = array_value.get("values") {
                                                if let Ok(vals) = serde_json::from_value::<Vec<serde_json::Value>>(values.clone()) {
                                                    for v in vals {
                                                        if let Some(sv) = v.get("stringValue").and_then(|x| x.as_str()) {
                                                            requests.push(sv.to_string());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let to_remove: std::collections::HashSet<String> = codes.into_iter().collect();
                    let filtered: Vec<serde_json::Value> = requests
                        .into_iter()
                        .filter(|d| !to_remove.contains(d))
                        .map(|d| json!({"stringValue": d}))
                        .collect();

                    let update_body = json!({
                        "fields": {
                            "hardware": {
                                "mapValue": {
                                    "fields": {
                                        "requests": {
                                            "arrayValue": {
                                                "values": filtered
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });

                    match client
                        .patch(&patch_url)
                        .header("Content-Type", "application/json")
                        .json(&update_body)
                        .send()
                        .await
                    {
                        Ok(_) => Json(json!({"success": true})),
                        Err(_) => Json(json!({"success": false, "error": "Failed to update"})),
                    }
                } else {
                    Json(json!({"error": "Failed to parse user data"}))
                }
            }
            Err(_) => Json(json!({"error": "Failed to read response"})),
        },
        Err(_) => Json(json!({"error": "Failed to fetch user"})),
    }
}

// Fetch both devices and requests in one call
pub async fn get_hardware_records(
    Path(userid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let client = Client::new();
    let get_url = format!("{}/users/{}", firebase_config.firestore_url(), userid);

    match client.get(&get_url).send().await {
        Ok(resp) => match resp.text().await {
            Ok(body) => {
                if let Ok(current_data) = serde_json::from_str::<serde_json::Value>(&body) {
                    let mut devices: Vec<String> = Vec::new();
                    let mut requests: Vec<String> = Vec::new();

                    if let Some(fields) = current_data.get("fields") {
                        if let Some(hardware) = fields.get("hardware") {
                            if let Some(map_value) = hardware.get("mapValue") {
                                if let Some(hw_fields) = map_value.get("fields") {
                                    // devices
                                    if let Some(devices_array) = hw_fields.get("devices") {
                                        if let Some(array_value) = devices_array.get("arrayValue") {
                                            if let Some(values) = array_value.get("values") {
                                                if let Ok(vals) = serde_json::from_value::<Vec<serde_json::Value>>(values.clone()) {
                                                    for v in vals {
                                                        if let Some(sv) = v.get("stringValue").and_then(|x| x.as_str()) {
                                                            devices.push(sv.to_string());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    // requests
                                    if let Some(requests_array) = hw_fields.get("requests") {
                                        if let Some(array_value) = requests_array.get("arrayValue") {
                                            if let Some(values) = array_value.get("values") {
                                                if let Ok(vals) = serde_json::from_value::<Vec<serde_json::Value>>(values.clone()) {
                                                    for v in vals {
                                                        if let Some(sv) = v.get("stringValue").and_then(|x| x.as_str()) {
                                                            requests.push(sv.to_string());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    Json(json!({"devices": devices, "requests": requests}))
                } else {
                    Json(json!({"error": "Failed to parse user data"}))
                }
            }
            Err(_) => Json(json!({"error": "Failed to read response"})),
        },
        Err(_) => Json(json!({"error": "Failed to fetch user"})),
    }
}
