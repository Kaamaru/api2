use crate::{
    structures::structs::{CreatePersonRequest, PersonProfile, UpdatePersonRequest},
    FirebaseConfig,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::Utc;
use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;

fn generate_unique_id() -> String {
    let now = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let uuid = Uuid::new_v4().simple();
    format!("{}_{}", uuid, now)
}

fn extract_document_id(document_name: &str) -> String {
    document_name
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string()
}

fn get_doc_string_field(doc: &Value, key: &str) -> Option<String> {
    doc.get("fields")
        .and_then(|f| f.get(key))
        .and_then(|v| v.get("stringValue"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}

fn get_doc_string_array_field(doc: &Value, key: &str) -> Vec<String> {
    doc.get("fields")
        .and_then(|f| f.get(key))
        .and_then(|v| v.get("arrayValue"))
        .and_then(|a| a.get("values"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    item.get("stringValue")
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn parse_person_doc(doc: &Value) -> PersonProfile {
    let person_id = doc
        .get("name")
        .and_then(|n| n.as_str())
        .map(extract_document_id)
        .unwrap_or_default();

    PersonProfile {
        person_id,
        name: get_doc_string_field(doc, "name").unwrap_or_default(),
        preference: get_doc_string_array_field(doc, "preference"),
        allergy: get_doc_string_array_field(doc, "allergy"),
    }
}

pub async fn get_people_profiles(
    Path(userid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let client = Client::new();
    let url = format!("{}/users/{}/people", firebase_config.firestore_url(), userid);

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to fetch people profiles: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to fetch people profiles"})),
            )
                .into_response();
        }
    };

    if resp.status() == StatusCode::NOT_FOUND {
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "people": [],
                "count": 0
            })),
        )
            .into_response();
    }

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to fetch people profiles", "details": body})),
        )
            .into_response();
    }

    let data: Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to parse people response: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to parse people response"})),
            )
                .into_response();
        }
    };

    let documents: Vec<Value> = data
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let people: Vec<PersonProfile> = documents.iter().map(parse_person_doc).collect();

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "people": people,
            "count": people.len()
        })),
    )
        .into_response()
}

pub async fn create_person_profile(
    Path(userid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<CreatePersonRequest>,
) -> impl IntoResponse {
    if request.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "name is required"})),
        )
            .into_response();
    }

    let client = Client::new();
    let person_id = format!("person_{}", generate_unique_id());
    let post_url = format!(
        "{}/users/{}/people?documentId={}",
        firebase_config.firestore_url(),
        userid,
        person_id
    );

    let preference_values: Vec<Value> = request
        .preference
        .iter()
        .map(|x| json!({ "stringValue": x }))
        .collect();
    let allergy_values: Vec<Value> = request
        .allergy
        .iter()
        .map(|x| json!({ "stringValue": x }))
        .collect();

    let payload = json!({
        "fields": {
            "name": { "stringValue": request.name },
            "preference": { "arrayValue": { "values": preference_values } },
            "allergy": { "arrayValue": { "values": allergy_values } }
        }
    });

    let response = match client.post(&post_url).json(&payload).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to create person profile: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create person profile"})),
            )
                .into_response();
        }
    };

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to create person profile", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "person": {
                "person_id": person_id,
                "name": request.name,
                "preference": request.preference,
                "allergy": request.allergy
            }
        })),
    )
        .into_response()
}

pub async fn update_person_profile(
    Path((userid, person_id)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
    Json(request): Json<UpdatePersonRequest>,
) -> impl IntoResponse {
    let mut fields = serde_json::Map::new();
    let mut update_paths: Vec<String> = Vec::new();

    if let Some(name) = &request.name {
        fields.insert("name".to_string(), json!({"stringValue": name}));
        update_paths.push("name".to_string());
    }

    if let Some(preference) = &request.preference {
        fields.insert(
            "preference".to_string(),
            json!({
                "arrayValue": {
                    "values": preference.iter().map(|x| json!({"stringValue": x})).collect::<Vec<_>>()
                }
            }),
        );
        update_paths.push("preference".to_string());
    }

    if let Some(allergy) = &request.allergy {
        fields.insert(
            "allergy".to_string(),
            json!({
                "arrayValue": {
                    "values": allergy.iter().map(|x| json!({"stringValue": x})).collect::<Vec<_>>()
                }
            }),
        );
        update_paths.push("allergy".to_string());
    }

    if update_paths.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "No fields to update"})),
        )
            .into_response();
    }

    let patch_url = format!(
        "{}/users/{}/people/{}?updateMask.fieldPaths={}",
        firebase_config.firestore_url(),
        userid,
        person_id,
        update_paths.join("&updateMask.fieldPaths=")
    );

    let payload = json!({ "fields": Value::Object(fields) });

    let client = Client::new();
    let response = match client.patch(&patch_url).json(&payload).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to update person profile: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to update person profile"})),
            )
                .into_response();
        }
    };

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to update person profile", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "person_id": person_id,
            "updated_fields": request
        })),
    )
        .into_response()
}

pub async fn delete_person_profile(
    Path((userid, person_id)): Path<(String, String)>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let client = Client::new();
    let delete_url = format!(
        "{}/users/{}/people/{}",
        firebase_config.firestore_url(),
        userid,
        person_id
    );

    let response = match client.delete(&delete_url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to delete person profile: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to delete person profile"})),
            )
                .into_response();
        }
    };

    if response.status() == StatusCode::NOT_FOUND {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"success": false, "error": "Person profile not found"})),
        )
            .into_response();
    }

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "Failed to delete person profile", "details": body})),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": format!("Person '{}' deleted", person_id)
        })),
    )
        .into_response()
}
