use crate::{
    structures::structs::{
        AddGroceryItemRequest, CreateGroceryListRequest, GroceryItem, GroceryList,
        GroceryItemField, GroceryListField, UpdateGroceryItemRequest,
    },
    FirebaseConfig,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    Router,
};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

fn generate_unique_id() -> String {
    let now = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let uuid = Uuid::new_v4().simple();
    format!("{}_{}", uuid, now)
}

// ===== GET Endpoints =====

/// Get all grocery lists for a user
pub async fn get_grocery_lists(
    Path(userid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    println!("get_grocery_lists for user: {}", userid);

    let url = format!("{}/users/{}", firebase_config.firestore_url(), userid);
    let resp = match reqwest::get(&url).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to fetch user: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch user"})),
            )
                .into_response();
        }
    };

    let text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("❌ Failed to read response: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to read response"})),
            )
                .into_response();
        }
    };

    // Parse the response to find grocery field
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(user_data) => {
            if let Some(fields) = user_data.get("fields") {
                if let Some(grocery_field) = fields.get("grocery") {
                    if let Some(map_value) = grocery_field.get("mapValue") {
                        if let Some(grocery_fields) = map_value.get("fields") {
                            let mut lists = Vec::new();

                            // Iterate through each grocery list
                            if let Some(obj) = grocery_fields.as_object() {
                                for (list_id, list_data) in obj.iter() {
                                    if let Some(list_map_value) =
                                        list_data.get("mapValue").and_then(|m| m.get("fields"))
                                    {
                                        let list_name = list_map_value
                                            .get("list_name")
                                            .and_then(|n| n.get("stringValue"))
                                            .and_then(|s| s.as_str())
                                            .unwrap_or("Unnamed List")
                                            .to_string();

                                        let items = parse_grocery_items(list_map_value);

                                        lists.push(GroceryList {
                                            list_id: list_id.clone(),
                                            list_name,
                                            items,
                                        });
                                    }
                                }
                            }

                            return (
                                StatusCode::OK,
                                Json(json!({
                                    "success": true,
                                    "lists": lists,
                                    "count": lists.len()
                                })),
                            )
                                .into_response();
                        }
                    }
                }
            }

            // No grocery lists found - return empty
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "lists": [],
                    "count": 0
                })),
            )
                .into_response()
        }
        Err(e) => {
            eprintln!("❌ Failed to parse user data: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse user data"})),
            )
                .into_response()
        }
    }
}

/// Get a single grocery list
pub async fn get_grocery_list(
    Path((userid, list_id)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    println!("get_grocery_list: user={}, list_id={}", userid, list_id);

    let url = format!("{}/users/{}", firebase_config.firestore_url(), userid);
    let resp = match reqwest::get(&url).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to fetch user: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch user"})),
            )
                .into_response();
        }
    };

    let text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("❌ Failed to read response: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to read response"})),
            )
                .into_response();
        }
    };

    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(user_data) => {
            if let Some(fields) = user_data.get("fields") {
                if let Some(grocery_field) = fields.get("grocery") {
                    if let Some(map_value) = grocery_field.get("mapValue") {
                        if let Some(grocery_fields) = map_value.get("fields") {
                            if let Some(list_data) = grocery_fields.get(&list_id) {
                                if let Some(list_map_value) =
                                    list_data.get("mapValue").and_then(|m| m.get("fields"))
                                {
                                    let list_name = list_map_value
                                        .get("list_name")
                                        .and_then(|n| n.get("stringValue"))
                                        .and_then(|s| s.as_str())
                                        .unwrap_or("Unnamed List")
                                        .to_string();

                                    let items = parse_grocery_items(list_map_value);

                                    return (
                                        StatusCode::OK,
                                        Json(json!({
                                            "success": true,
                                            "list": {
                                                "list_id": list_id,
                                                "list_name": list_name,
                                                "items": items,
                                            }
                                        })),
                                    )
                                        .into_response();
                                }
                            }
                        }
                    }
                }
            }

            (
                StatusCode::NOT_FOUND,
                Json(json!({"error": "Grocery list not found"})),
            )
                .into_response()
        }
        Err(e) => {
            eprintln!("❌ Failed to parse user data: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse user data"})),
            )
                .into_response()
        }
    }
}

// ===== CREATE Endpoints =====

/// Create a new grocery list
pub async fn create_grocery_list(
    Path(userid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<CreateGroceryListRequest>,
) -> impl IntoResponse {
    println!("create_grocery_list: user={}", userid);

    let client = Client::new();
    let list_id = format!("grocery_{}", generate_unique_id());

    let patch_url = format!(
        "{}/users/{}?updateMask.fieldPaths=grocery.{}",
        firebase_config.firestore_url(),
        userid,
        list_id
    );

    let firestore_payload = json!({
        "fields": {
            "grocery": {
                "mapValue": {
                    "fields": {
                        list_id.clone(): {
                            "mapValue": {
                                "fields": {
                                    "list_name": { "stringValue": request.list_name },
                                    "items": {
                                        "mapValue": {
                                            "fields": {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let patch_response = match client.patch(&patch_url).json(&firestore_payload).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to patch grocery list: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create grocery list"})),
            )
                .into_response();
        }
    };

    if !patch_response.status().is_success() {
        let status = patch_response.status();
        let body = patch_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Firestore PATCH failed: {status}\n{body}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create grocery list", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": format!("Grocery list '{}' created", request.list_name),
            "list": {
                "list_id": list_id,
                "list_name": request.list_name,
                "items": []
            }
        })),
    )
        .into_response()
}

// ===== ADD Item Endpoint =====

/// Add item to grocery list
pub async fn add_grocery_item(
    Path((userid, list_id)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<AddGroceryItemRequest>,
) -> impl IntoResponse {
    println!("add_grocery_item: user={}, list_id={}", userid, list_id);

    let client = Client::new();
    let item_id = format!("item_{}", generate_unique_id());

    let patch_url = format!(
        "{}/users/{}?updateMask.fieldPaths=grocery.{}.items.{}",
        firebase_config.firestore_url(),
        userid,
        list_id,
        item_id
    );

    let firestore_payload = json!({
        "fields": {
            "grocery": {
                "mapValue": {
                    "fields": {
                        list_id.clone(): {
                            "mapValue": {
                                "fields": {
                                    "items": {
                                        "mapValue": {
                                            "fields": {
                                                item_id.clone(): {
                                                    "mapValue": {
                                                        "fields": {
                                                            "uid": { "stringValue": item_id.clone() },
                                                            "name": { "stringValue": request.name },
                                                            "quantity": { "stringValue": request.quantity },
                                                            "unit": { "stringValue": request.unit },
                                                            "checked": { "booleanValue": false }
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
                }
            }
        }
    });

    let patch_response = match client.patch(&patch_url).json(&firestore_payload).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to patch grocery item: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to add grocery item"})),
            )
                .into_response();
        }
    };

    if !patch_response.status().is_success() {
        let status = patch_response.status();
        let body = patch_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Firestore PATCH failed: {status}\n{body}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to add grocery item", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": format!("Item '{}' added to list", request.name),
            "item": {
                "uid": item_id,
                "name": request.name,
                "quantity": request.quantity,
                "unit": request.unit,
                "checked": false
            }
        })),
    )
        .into_response()
}

// ===== UPDATE Item Endpoint =====

/// Update grocery item (toggle checked, update quantity)
pub async fn update_grocery_item(
    Path((userid, list_id, item_id)): Path<(String, String, String)>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<UpdateGroceryItemRequest>,
) -> impl IntoResponse {
    println!(
        "update_grocery_item: user={}, list_id={}, item_id={}",
        userid, list_id, item_id
    );

    let client = Client::new();
    let mut field_paths = Vec::new();
    let mut item_fields = serde_json::Map::new();

    // Build item fields based on what was provided
    if let Some(checked) = request.checked {
        field_paths.push(format!("grocery.{}.items.{}.checked", list_id, item_id));
        item_fields.insert("checked".to_string(), json!({"booleanValue": checked}));
    }

    if let Some(quantity) = &request.quantity {
        field_paths.push(format!("grocery.{}.items.{}.quantity", list_id, item_id));
        item_fields.insert("quantity".to_string(), json!({"stringValue": quantity}));
    }

    if field_paths.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "No fields to update"})),
        )
            .into_response();
    }

    let patch_url = format!(
        "{}/users/{}?updateMask.fieldPaths={}",
        firebase_config.firestore_url(),
        userid,
        field_paths.join("&updateMask.fieldPaths=")
    );

    // Build nested structure EXACTLY like add_grocery_item does
    // item_fields has the leaf values (checked, quantity, etc)
    
    // Wrap item in its map value
    let mut item_entry = serde_json::Map::new();
    item_entry.insert(item_id.clone(), json!({"mapValue": {"fields": item_fields}}));
    
    // items field contains the item_entry
    let items_value = json!({
        "mapValue": {
            "fields": serde_json::Value::Object(item_entry)
        }
    });
    
    // Now build list level
    let mut list_fields = serde_json::Map::new();
    list_fields.insert("items".to_string(), items_value);
    
    // List ID is the key
    let mut list_entry = serde_json::Map::new();
    list_entry.insert(list_id.clone(), json!({"mapValue": {"fields": list_fields}}));

    let firestore_payload = json!({
        "fields": {
            "grocery": {
                "mapValue": {
                    "fields": serde_json::Value::Object(list_entry)
                }
            }
        }
    });

    eprintln!("📤 UPDATE payload: {}", firestore_payload);

    let patch_response = match client.patch(&patch_url).json(&firestore_payload).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to patch grocery item: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to update grocery item"})),
            )
                .into_response();
        }
    };

    if !patch_response.status().is_success() {
        let status = patch_response.status();
        let body = patch_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Firestore PATCH failed: {status}\n{body}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to update grocery item"})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Grocery item updated",
            "updated_fields": serde_json::to_value(&request).unwrap_or(json!({}))
        })),
    )
        .into_response()
}

// ===== DELETE Endpoints =====

/// Delete entire grocery list
pub async fn delete_grocery_list(
    Path((userid, list_id)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    println!("delete_grocery_list: user={}, list_id={}", userid, list_id);

    let client = Client::new();
    let patch_url = format!(
        "{}/users/{}?updateMask.fieldPaths=grocery.{}",
        firebase_config.firestore_url(),
        userid,
        list_id
    );

    // Send empty mapValue to delete the list
    let firestore_payload = json!({
        "fields": {
            "grocery": {
                "mapValue": {
                    "fields": {}
                }
            }
        }
    });

    eprintln!("📤 DELETE LIST payload: {}", firestore_payload);

    let patch_response = match client.patch(&patch_url).json(&firestore_payload).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to delete list: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to delete list"})),
            )
                .into_response();
        }
    };

    if !patch_response.status().is_success() {
        let body = patch_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Delete list failed: {}", body);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to delete list"})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": format!("Grocery list '{}' deleted", list_id)
        })),
    )
        .into_response()
}

/// Delete single grocery item
pub async fn delete_grocery_item(
    Path((userid, list_id, item_id)): Path<(String, String, String)>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    println!(
        "delete_grocery_item: user={}, list_id={}, item_id={}",
        userid, list_id, item_id
    );

    let client = Client::new();

    // FIRST: Verify item exists by fetching the list
    let get_url = format!("{}/users/{}", firebase_config.firestore_url(), userid);
    let resp = match reqwest::get(&get_url).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to verify item: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify item"})),
            )
                .into_response();
        }
    };

    let text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("❌ Failed to read response: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify item"})),
            )
                .into_response();
        }
    };

    // Check if item exists
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(user_data) => {
            let item_exists = user_data
                .get("fields")
                .and_then(|f| f.get("grocery"))
                .and_then(|g| g.get("mapValue"))
                .and_then(|m| m.get("fields"))
                .and_then(|f| f.get(&list_id))
                .and_then(|l| l.get("mapValue"))
                .and_then(|m| m.get("fields"))
                .and_then(|f| f.get("items"))
                .and_then(|i| i.get("mapValue"))
                .and_then(|m| m.get("fields"))
                .and_then(|f| f.get(&item_id))
                .is_some();

            if !item_exists {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({"error": "Item not found", "success": false})),
                )
                    .into_response();
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to parse user data: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to verify item"})),
            )
                .into_response();
        }
    }

    // SECOND: Item exists - proceed with deletion
    let patch_url = format!(
        "{}/users/{}?updateMask.fieldPaths=grocery.{}.items.{}",
        firebase_config.firestore_url(),
        userid,
        list_id,
        item_id
    );

    // Send empty mapValue to delete the item
    let mut item_entry = serde_json::Map::new();
    // Empty map deletes the item

    let mut items_map = serde_json::Map::new();
    items_map.insert(
        "items".to_string(),
        json!({"mapValue": {"fields": item_entry}}),
    );

    let mut list_entry = serde_json::Map::new();
    list_entry.insert(
        list_id.clone(),
        json!({"mapValue": {"fields": items_map}}),
    );

    let firestore_payload = json!({
        "fields": {
            "grocery": {
                "mapValue": {
                    "fields": list_entry
                }
            }
        }
    });

    eprintln!("📤 DELETE ITEM payload: {}", firestore_payload);

    let patch_response = match client.patch(&patch_url).json(&firestore_payload).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to delete item: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to delete item"})),
            )
                .into_response();
        }
    };

    if !patch_response.status().is_success() {
        let body = patch_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Delete item failed: {}", body);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to delete item"})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": format!("Grocery item '{}' deleted", item_id)
        })),
    )
        .into_response()
}

// ===== Helper Functions =====

// ===== CONVERSION Endpoint =====

/// Convert checked grocery item to schedule
pub async fn convert_grocery_to_schedule(
    Path((userid, list_id, item_id)): Path<(String, String, String)>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<crate::structures::structs::GroceryToScheduleRequest>,
) -> impl IntoResponse {
    println!(
        "convert_grocery_to_schedule: user={}, list_id={}, item_id={}",
        userid, list_id, item_id
    );

    let client = Client::new();

    // STEP 1: Fetch the grocery item details
    let get_url = format!("{}/users/{}", firebase_config.firestore_url(), userid);
    let resp = match reqwest::get(&get_url).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to fetch user: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch grocery item"})),
            )
                .into_response();
        }
    };

    let text = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("❌ Failed to read response: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch grocery item"})),
            )
                .into_response();
        }
    };

    // Parse and extract grocery item
    let user_data = match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("❌ Failed to parse user data: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse grocery item"})),
            )
                .into_response();
        }
    };

    let item_data = user_data
        .get("fields")
        .and_then(|f| f.get("grocery"))
        .and_then(|g| g.get("mapValue"))
        .and_then(|m| m.get("fields"))
        .and_then(|f| f.get(&list_id))
        .and_then(|l| l.get("mapValue"))
        .and_then(|m| m.get("fields"))
        .and_then(|f| f.get("items"))
        .and_then(|i| i.get("mapValue"))
        .and_then(|m| m.get("fields"))
        .and_then(|f| f.get(&item_id))
        .and_then(|i| i.get("mapValue"))
        .and_then(|m| m.get("fields"));

    if item_data.is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "Grocery item not found"})),
        )
            .into_response();
    }

    let item_map = item_data.unwrap();
    let item_name = item_map
        .get("name")
        .and_then(|n| n.get("stringValue"))
        .and_then(|s| s.as_str())
        .unwrap_or("Unknown")
        .to_string();

    let item_quantity = item_map
        .get("quantity")
        .and_then(|q| q.get("stringValue"))
        .and_then(|s| s.as_str())
        .unwrap_or("1")
        .to_string();

    let item_unit = item_map
        .get("unit")
        .and_then(|u| u.get("stringValue"))
        .and_then(|s| s.as_str())
        .unwrap_or("")
        .to_string();

    // STEP 2: Create a schedule with the grocery item details
    let schedule_id = format!("schedule_{}", generate_unique_id());
    let schedule_patch_url = format!(
        "{}/users/{}?updateMask.fieldPaths=schedules.{}",
        firebase_config.firestore_url(),
        userid,
        schedule_id
    );

    let quantity_str = if item_unit.is_empty() {
        item_quantity.clone()
    } else {
        format!("{} {}", item_quantity, item_unit)
    };

    let schedule_payload = json!({
        "fields": {
            "schedules": {
                "mapValue": {
                    "fields": {
                        schedule_id.clone(): {
                            "mapValue": {
                                "fields": {
                                    "uid": { "stringValue": schedule_id.clone() },
                                    "name": { "stringValue": item_name },
                                    "quantity": { "stringValue": quantity_str },
                                    "date": { "stringValue": request.date },
                                    "time": { "stringValue": request.time },
                                    "expiry": { "stringValue": request.expiry },
                                    "category": { "stringValue": request.category.clone().unwrap_or_else(|| "grocery".to_string()) },
                                    "notify": { "booleanValue": false }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let schedule_response = match client
        .patch(&schedule_patch_url)
        .json(&schedule_payload)
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to create schedule: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create schedule"})),
            )
                .into_response();
        }
    };

    if !schedule_response.status().is_success() {
        let status = schedule_response.status();
        let body = schedule_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Schedule creation failed: {}\n{}", status, body);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create schedule", "details": body})),
        )
            .into_response();
    }

    // STEP 3: Mark grocery item as checked
    let grocery_patch_url = format!(
        "{}/users/{}?updateMask.fieldPaths=grocery.{}.items.{}.checked",
        firebase_config.firestore_url(),
        userid,
        list_id,
        item_id
    );

    let mut item_fields = serde_json::Map::new();
    item_fields.insert("checked".to_string(), json!({"booleanValue": true}));

    let mut item_entry = serde_json::Map::new();
    item_entry.insert(
        item_id.clone(),
        json!({"mapValue": {"fields": item_fields}}),
    );

    let items_value = json!({
        "mapValue": {
            "fields": serde_json::Value::Object(item_entry)
        }
    });

    let mut list_fields = serde_json::Map::new();
    list_fields.insert("items".to_string(), items_value);

    let mut list_entry = serde_json::Map::new();
    list_entry.insert(
        list_id.clone(),
        json!({"mapValue": {"fields": list_fields}}),
    );

    let grocery_payload = json!({
        "fields": {
            "grocery": {
                "mapValue": {
                    "fields": serde_json::Value::Object(list_entry)
                }
            }
        }
    });

    let grocery_response = match client
        .patch(&grocery_patch_url)
        .json(&grocery_payload)
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to update grocery item: {}", e);
            // Schedule was created but grocery item marking failed - not ideal but partial success
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Schedule created but failed to mark item as checked",
                    "schedule_id": schedule_id
                })),
            )
                .into_response();
        }
    };

    if !grocery_response.status().is_success() {
        let status = grocery_response.status();
        let body = grocery_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Grocery item update failed: {}\\n{}", status, body);
        // Again, schedule created but grocery marker failed
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "error": "Schedule created but failed to mark item as checked",
                "schedule_id": schedule_id,
                "details": body
            })),
        )
            .into_response();
    }

    // SUCCESS: Both operations completed
    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": "Grocery item converted to schedule",
            "schedule": {
                "schedule_id": schedule_id,
                "name": item_name,
                "quantity": quantity_str,
                "date": request.date,
                "time": request.time,
                "expiry": request.expiry,
                "category": request.category.unwrap_or_else(|| "grocery".to_string())
            },
            "grocery_item": {
                "item_id": item_id,
                "checked": true
            }
        })),
    )
        .into_response()
}

/// Parse grocery items from Firestore data
fn parse_grocery_items(list_data: &serde_json::Value) -> Vec<GroceryItem> {
    let mut items = Vec::new();

    if let Some(items_field) = list_data.get("items") {
        if let Some(map_value) = items_field.get("mapValue") {
            if let Some(item_fields) = map_value.get("fields") {
                if let Some(obj) = item_fields.as_object() {
                    for (item_id, item_data) in obj.iter() {
                        if let Some(item_map) = item_data.get("mapValue").and_then(|m| m.get("fields"))
                        {
                            let name = item_map
                                .get("name")
                                .and_then(|n| n.get("stringValue"))
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string();

                            let quantity = item_map
                                .get("quantity")
                                .and_then(|q| q.get("stringValue"))
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string();

                            let unit = item_map
                                .get("unit")
                                .and_then(|u| u.get("stringValue"))
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string();

                            let checked = item_map
                                .get("checked")
                                .and_then(|c| c.get("booleanValue"))
                                .and_then(|b| b.as_bool())
                                .unwrap_or(false);

                            items.push(GroceryItem {
                                uid: item_id.clone(),
                                name,
                                quantity,
                                unit,
                                checked,
                            });
                        }
                    }
                }
            }
        }
    }

    items
}
