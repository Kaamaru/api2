use crate::{
    structures::structs::{AddRecipeRequest, AddScheduleRequest},
    FirebaseConfig,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::patch,
    Router,
};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

// Request payload structure
#[derive(Debug, Serialize, Deserialize)]
pub struct PatchScheduleRequest {
    pub fields: RequestFields,
}

#[derive(Debug, Serialize, Deserialize)]
struct RequestFields {
    schedules: SchedulesField,
}

#[derive(Debug, Serialize, Deserialize)]
struct SchedulesField {
    #[serde(rename = "mapValue")]
    map_value: SchedulesMapValue,
}

#[derive(Debug, Serialize, Deserialize)]
struct SchedulesMapValue {
    fields: std::collections::HashMap<String, MealField>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MealField {
    #[serde(rename = "mapValue")]
    map_value: MealMapValue,
}

#[derive(Debug, Serialize, Deserialize)]
struct MealMapValue {
    fields: MealData,
}

#[derive(Debug, Serialize, Deserialize)]
struct MealData {
    name: Option<StringValue>,
    expiry: Option<StringValue>,
    date: Option<StringValue>,
    time: Option<StringValue>,
    category: Option<StringValue>,
    location: Option<StringValue>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StringValue {
    #[serde(rename = "stringValue")]
    string_value: String,
}
fn generate_unique_id() -> String {
    let now = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let uuid = Uuid::new_v4().simple(); // random UUID
    format!("{}_{}", uuid, now)
}
pub async fn change_get() -> String {
    "Change your life".to_string()
}
pub async fn delete_get() -> String {
    "Delete yourself.".to_string()
}
pub async fn add_get() -> String {
    "Add someone...".to_string()
}

pub async fn firestore_add_schedule(
    Path(uuid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<AddScheduleRequest>,
) -> impl IntoResponse {
    println!("firestore_add_schedule");

    let client = Client::new();

    let new_id = format!("schedule_{}", generate_unique_id());
    let post_url = format!(
        "{}/users/{}/schedules?documentId={}",
        firebase_config.firestore_url(),
        uuid,
        new_id
    );
    println!("POST URL: {}", post_url);

    let mut schedule_fields = json!({
        "uid": { "stringValue": new_id.clone() },
        "name": { "stringValue": request.name },
        "time": { "stringValue": request.time },
        "date": { "stringValue": request.date },
        "expiry": { "stringValue": request.expiry },
        "notify": { "booleanValue": request.notify },
        "category": { "stringValue": request.category.clone().unwrap_or_default() },
        "quantity": { "stringValue": request.quantity }
    });

    if let Some(category) = &request.category {
        schedule_fields
            .as_object_mut()
            .unwrap()
            .insert("category".to_string(), json!({"stringValue": category}));
    }

    if let Some(location) = &request.location {
        schedule_fields
            .as_object_mut()
            .unwrap()
            .insert("location".to_string(), json!({"stringValue": location}));
    }

    let firestore_payload = json!({
        "fields": schedule_fields
    });

    let patch_response = match client
        .post(&post_url)
        .json(&firestore_payload)
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to patch schedule: {}", e);
            return Json(json!({ "error": "Failed to patch schedule" })).into_response();
        }
    };

    if !patch_response.status().is_success() {
        let status = patch_response.status();
        let body = patch_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Firestore PATCH failed: {status}\n{body}");
        return Json(json!({ "error": "Firestore update failed", "details": body }))
            .into_response();
    }

    // Optional: Return confirmation
    Json(json!({
        "success": true,
        "message": format!("Schedule '{}' added successfully", request.name),
        "schedule": {
            "uid": new_id,
            "name": request.name,
            "time": request.time,
            "date": request.date,
            "expiry": request.expiry,
            "quantity": request.quantity,
            "category": request.category,
            "location": request.location,
            "notify": request.notify,
        },
        "updated_at": chrono::Utc::now().to_rfc3339()
    }))
    .into_response()
}
#[axum::debug_handler]
pub async fn firestore_add_recipe(
    Path(uuid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<AddRecipeRequest>,
) -> impl IntoResponse {
    let client = Client::new();
    let new_id = format!("recipe_{}", generate_unique_id());

    let post_url = format!(
        "{}/users/{}/savedrecipes?documentId={}",
        firebase_config.firestore_url(),
        uuid,
        new_id
    );

    // Convert ingredients and steps to Firestore arrayValue format
    let ingredients_array: Vec<Value> = request
        .ingredients
        .iter()
        .map(|ingredient| json!({"stringValue": ingredient}))
        .collect();

    let steps_array: Vec<Value> = request
        .steps
        .iter()
        .map(|step| json!({"stringValue": step}))
        .collect();

    // Build the recipe fields - this is the correct structure
    let mut recipe_fields = json!({
        "uid": { "stringValue": new_id.clone() },
                                  "name": { "stringValue": request.name },
                                  "ingredients": {
                                      "arrayValue": {
                                          "values": ingredients_array
                                      }
                                  },
                                  "steps": {
                                      "arrayValue": {
                                          "values": steps_array
                                      }
                                  }
    });

    // Add time field if provided
    if let Some(time) = &request.time {
        recipe_fields
            .as_object_mut()
            .unwrap()
            .insert("time".to_string(), json!({"stringValue": time}));
    }

    // Add image_url field if provided
    if let Some(image_url) = &request.image_url {
        recipe_fields
            .as_object_mut()
            .unwrap()
            .insert("image_url".to_string(), json!({"stringValue": image_url}));
    }

    // Add quantity field if provided
    if let Some(quantity) = &request.quantity {
        recipe_fields
            .as_object_mut()
            .unwrap()
            .insert("quantity".to_string(), json!({"stringValue": quantity}));
    }

    let firestore_payload = json!({
        "fields": recipe_fields
    });

    let patch_response = match client
        .post(&post_url)
        .json(&firestore_payload)
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to patch recipe: {}", e);
            return Json(json!({ "error": "Failed to add recipe" })).into_response();
        }
    };

    if !patch_response.status().is_success() {
        let status = patch_response.status();
        let body = patch_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Firestore PATCH failed: {status}\n{body}");
        return Json(json!({ "error": "Firestore update failed", "details": body }))
            .into_response();
    }

    // Respond with confirmation
    Json(json!({
        "success": true,
        "message": format!("Recipe '{}' added successfully", request.name),
               "recipe": {
                   "name": request.name,
                   "ingredients": request.ingredients,
                   "steps": request.steps,
                   "time": request.time
               },
               "updated_at": chrono::Utc::now().to_rfc3339()
    }))
    .into_response()
}

pub async fn firestore_change_schedule(
    Path(uuid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(payload): Json<PatchScheduleRequest>,
) -> impl IntoResponse {
    let client = Client::new();

    let mut updated_count = 0usize;

    for (meal_key, meal_value) in payload.fields.schedules.map_value.fields {
        let mut meal_fields: serde_json::Map<String, Value> = serde_json::Map::new();
        let mut update_paths: Vec<String> = Vec::new();

        if let Some(name) = meal_value.map_value.fields.name {
            update_paths.push("name".to_string());
            meal_fields.insert("name".to_string(), json!({"stringValue": name.string_value}));
        }
        if let Some(expiry) = meal_value.map_value.fields.expiry {
            update_paths.push("expiry".to_string());
            meal_fields.insert(
                "expiry".to_string(),
                json!({"stringValue": expiry.string_value}),
            );
        }
        if let Some(date) = meal_value.map_value.fields.date {
            update_paths.push("date".to_string());
            meal_fields.insert("date".to_string(), json!({"stringValue": date.string_value}));
        }
        if let Some(time) = meal_value.map_value.fields.time {
            update_paths.push("time".to_string());
            meal_fields.insert("time".to_string(), json!({"stringValue": time.string_value}));
        }

        if let Some(category) = meal_value.map_value.fields.category {
            update_paths.push("category".to_string());
            meal_fields.insert(
                "category".to_string(),
                json!({"stringValue": category.string_value}),
            );
        }

        if let Some(location) = meal_value.map_value.fields.location {
            update_paths.push("location".to_string());
            meal_fields.insert(
                "location".to_string(),
                json!({"stringValue": location.string_value}),
            );
        }

        if update_paths.is_empty() {
            continue;
        }

        let patch_url = format!(
            "{}/users/{}/schedules/{}?updateMask.fieldPaths={}",
            firebase_config.firestore_url(),
            uuid,
            meal_key,
            update_paths.join("&updateMask.fieldPaths=")
        );

        let firestore_payload = json!({
            "fields": Value::Object(meal_fields)
        });

        let response = match client.patch(&patch_url).json(&firestore_payload).send().await {
            Ok(response) => response,
            Err(e) => {
                eprintln!("Failed to update schedule {}: {}", meal_key, e);
                return Json(json!({ "error": "Failed to update schedule" })).into_response();
            }
        };

        if !response.status().is_success() {
            let error_body = response.text().await.unwrap_or_default();
            eprintln!("Error updating schedule {}: {}", meal_key, error_body);
            return Json(json!({ "error": "Failed to update schedule", "details": error_body }))
                .into_response();
        }

        updated_count += 1;
    }

    // Return success response
    Json(json!({
        "success": true,
        "message": "Schedule updated successfully",
        "uuid": uuid,
        "updated_count": updated_count,
        "updated_at": chrono::Utc::now().to_rfc3339()
    }))
    .into_response()
}

pub async fn firestore_delete_schedule(
    Path((uuid, schedule_key)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let delete_url = format!(
        "{}/users/{}/schedules/{}",
        firebase_config.firestore_url(),
        uuid,
        schedule_key
    );
    let client = Client::new();

    let update_response = match client.delete(&delete_url).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("Failed to update schedule: {}", e);
            return Json(json!({ "error": "Failed to delete schedule" })).into_response();
        }
    };

    if update_response.status() == StatusCode::NOT_FOUND {
        return Json(json!({ "error": "Schedule not found" })).into_response();
    }

    if !update_response.status().is_success() {
        eprintln!(
            "⚠️ Firestore update responded with status: {}",
            update_response.status()
        );
        let error_body = update_response.text().await.unwrap_or_default();
        eprintln!("Error body: {}", error_body);
        return Json(json!({ "error": "Failed to delete schedule" })).into_response();
    }

    let response_body = match update_response.text().await {
        Ok(body) => {
            println!(
                "✅ Schedule '{}' deleted successfully for user {}",
                schedule_key, uuid
            );
            body
        }
        Err(e) => {
            eprintln!("Failed to get response body: {}", e);
            return Json(json!({ "error": "Failed to delete schedule" })).into_response();
        }
    };

    // Return success response
    Json(json!({
        "success": true,
        "message": format!("Schedule '{}' deleted successfully", schedule_key),
        "uuid": uuid,
        "deleted_schedule": schedule_key,
        "deleted_at": chrono::Utc::now().to_rfc3339()
    }))
    .into_response()
}

pub async fn firestore_delete_recipe(
    Path((uuid, recipe_key)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let delete_url = format!(
        "{}/users/{}/savedrecipes/{}",
        firebase_config.firestore_url(),
        uuid,
        recipe_key
    );
    let client = Client::new();

    let update_response = match client.delete(&delete_url).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("Failed to update recipes: {}", e);
            return Json(json!({ "error": "Failed to delete recipe" })).into_response();
        }
    };

    if update_response.status() == StatusCode::NOT_FOUND {
        return Json(json!({ "error": "Recipe not found" })).into_response();
    }

    if !update_response.status().is_success() {
        eprintln!(
            "⚠️ Firestore update responded with status: {}",
            update_response.status()
        );
        let error_body = update_response.text().await.unwrap_or_default();
        eprintln!("Error body: {}", error_body);
        return Json(json!({ "error": "Failed to delete recipe" })).into_response();
    }

    let response_body = match update_response.text().await {
        Ok(body) => {
            println!(
                "✅ Recipe '{}' deleted successfully for user {}",
                recipe_key, uuid
            );
            body
        }
        Err(e) => {
            eprintln!("Failed to get response body: {}", e);
            return Json(json!({ "error": "Failed to delete recipe" })).into_response();
        }
    };

    // Return success response
    Json(json!({
        "success": true,
        "message": format!("Recipe '{}' deleted successfully", recipe_key),
        "uuid": uuid,
        "deleted_recipe": recipe_key,
        "deleted_at": chrono::Utc::now().to_rfc3339()
    }))
    .into_response()
}

pub async fn firestore_change_displayname(
    Path((uuid, newname)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let client = Client::new();
    
    // Build update URL with updateMask to only update displayName
    let update_url = format!(
        "{}?updateMask.fieldPaths=displayName",
        format!("{}/users/{}", firebase_config.firestore_url(), uuid)
    );

    // Create Firestore payload to update displayName
    let firestore_payload = json!({
        "fields": {
            "displayName": {
                "stringValue": newname
            }
        }
    });

    let response = match client
        .patch(&update_url)
        .json(&firestore_payload)
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            eprintln!("❌ Failed to update displayName: {}", e);
            return Json(json!({ "error": "Failed to update displayName" })).into_response();
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        eprintln!("⚠️ Firestore update failed: {status}\n{body}");
        return Json(json!({ "error": "Failed to update displayName", "details": body }))
            .into_response();
    }

    let body = match response.text().await {
        Ok(body) => {
            println!("✅ DisplayName updated successfully: {}", body);
            body
        }
        Err(e) => {
            eprintln!("Failed to get response body: {}", e);
            return Json(json!({ "error": "Failed to update displayName" })).into_response();
        }
    };

    // Return success response
    Json(json!({
        "success": true,
        "message": "Display name updated successfully",
        "uuid": uuid,
        "new_displayName": newname,
        "updated_at": chrono::Utc::now().to_rfc3339()
    }))
    .into_response()
}
