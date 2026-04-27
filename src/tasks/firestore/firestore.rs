use crate::{
    firebase::{self, FirebaseConfig},
    structures::structs::{
        FeedbackDocument, FeedbackFields, FirestoreMapField, FirestoreRecipeMapField,
        FirestoreScheduleDetail, FirestoreScheduleItem, FirestoreSingleUserResponse,
        FirestoreStringField, FirestoreUserDocument, FirestoreUserResponse, GroceryItem,
        GroceryList, Hardware, HardwareField, PersonProfile, Recipe, RecipeField, Schedule, User,
        FirestoreHardwareMapField,
    },
};
use axum::{
    extract::{Json, Path, State},
    http::{response, HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::get,
    Json as AxumJson, Router,
};
use chrono::{Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, fmt::format};

use chrono_tz::Asia::Bangkok;

fn extract_doc_id(name: &str) -> String {
    name.rsplit('/').next().unwrap_or_default().to_string()
}

fn get_doc_string_field(doc: &serde_json::Value, key: &str) -> Option<String> {
    doc.get("fields")
        .and_then(|f| f.get(key))
        .and_then(|v| v.get("stringValue"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}

fn get_doc_bool_field(doc: &serde_json::Value, key: &str) -> Option<bool> {
    doc.get("fields")
        .and_then(|f| f.get(key))
        .and_then(|v| v.get("booleanValue"))
        .and_then(|b| b.as_bool())
}

fn get_doc_string_array_field(doc: &serde_json::Value, key: &str) -> Vec<String> {
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

async fn resolve_user_document(
    client: &Client,
    firebase_config: &FirebaseConfig,
    identifier: &str,
) -> Result<(String, Value), String> {
    let direct_url = format!("{}/users/{}", firebase_config.firestore_url(), identifier);

    if let Ok(response) = client.get(&direct_url).send().await {
        if response.status().is_success() {
            let user_doc: Value = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse user doc: {e}"))?;
            let resolved_uid = user_doc
                .get("name")
                .and_then(|n| n.as_str())
                .map(extract_doc_id)
                .unwrap_or_else(|| identifier.to_string());
            return Ok((resolved_uid, user_doc));
        }
    }

    let users_url = format!("{}/users", firebase_config.firestore_url());
    let response = client
        .get(&users_url)
        .send()
        .await
        .map_err(|e| format!("Failed to list users: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to list users {status}: {body}"));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse users response: {e}"))?;

    let docs = payload
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    for doc in docs {
        let doc_id = doc
            .get("name")
            .and_then(|n| n.as_str())
            .map(extract_doc_id)
            .unwrap_or_default();

        let email_matches = get_doc_string_field(&doc, "email")
            .map(|value| value == identifier)
            .unwrap_or(false);
        let username_matches = get_doc_string_field(&doc, "username")
            .map(|value| value == identifier)
            .unwrap_or(false);
        let uid_matches = get_doc_string_field(&doc, "uid")
            .map(|value| value == identifier)
            .unwrap_or(false);

        if email_matches || username_matches || uid_matches || doc_id == identifier {
            return Ok((doc_id, doc));
        }
    }

    Err("User not found or invalid UID".to_string())
}

async fn fetch_user_schedules_subcollection(
    client: &Client,
    firebase_config: &FirebaseConfig,
    uid: &str,
) -> Result<Vec<Schedule>, String> {
    let url = format!("{}/users/{}/schedules", firebase_config.firestore_url(), uid);
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch schedules: {e}"))?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(vec![]);
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to fetch schedules {status}: {body}"));
    }

    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse schedules response: {e}"))?;

    let docs = payload
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let schedules = docs
        .iter()
        .map(|doc| {
            let uid = get_doc_string_field(doc, "uid").unwrap_or_else(|| {
                doc.get("name")
                    .and_then(|n| n.as_str())
                    .map(extract_doc_id)
                    .unwrap_or_default()
            });

            Schedule {
                name: get_doc_string_field(doc, "name").unwrap_or_default(),
                time: get_doc_string_field(doc, "time").unwrap_or_default(),
                date: get_doc_string_field(doc, "date").unwrap_or_default(),
                expiry: get_doc_string_field(doc, "expiry").unwrap_or_default(),
                uid,
                notify: get_doc_bool_field(doc, "notify").unwrap_or(false),
                category: get_doc_string_field(doc, "category"),
                quantity: get_doc_string_field(doc, "quantity"),
                location: get_doc_string_field(doc, "location"),
            }
        })
        .collect();

    Ok(schedules)
}

async fn fetch_user_savedrecipes_subcollection(
    client: &Client,
    firebase_config: &FirebaseConfig,
    uid: &str,
) -> Result<Vec<Recipe>, String> {
    let url = format!("{}/users/{}/savedrecipes", firebase_config.firestore_url(), uid);
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch saved recipes: {e}"))?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(vec![]);
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to fetch recipes {status}: {body}"));
    }

    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse saved recipes response: {e}"))?;

    let docs = payload
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let recipes = docs
        .iter()
        .map(|doc| {
            let uid = get_doc_string_field(doc, "uid").unwrap_or_else(|| {
                doc.get("name")
                    .and_then(|n| n.as_str())
                    .map(extract_doc_id)
                    .unwrap_or_default()
            });

            Recipe {
                uid,
                name: get_doc_string_field(doc, "name").unwrap_or_default(),
                ingredients: get_doc_string_array_field(doc, "ingredients"),
                steps: get_doc_string_array_field(doc, "steps"),
                time: get_doc_string_field(doc, "time"),
                image_url: get_doc_string_field(doc, "image_url"),
                quantity: get_doc_string_field(doc, "quantity"),
            }
        })
        .collect();

    Ok(recipes)
}

fn parse_grocery_item_doc(doc: &Value) -> GroceryItem {
    let uid = get_doc_string_field(doc, "uid").unwrap_or_else(|| {
        doc.get("name")
            .and_then(|n| n.as_str())
            .map(extract_doc_id)
            .unwrap_or_default()
    });

    GroceryItem {
        uid,
        name: get_doc_string_field(doc, "name").unwrap_or_default(),
        quantity: get_doc_string_field(doc, "quantity").unwrap_or_default(),
        unit: get_doc_string_field(doc, "unit").unwrap_or_default(),
        checked: get_doc_bool_field(doc, "checked").unwrap_or(false),
    }
}

async fn fetch_grocery_items_subcollection(
    client: &Client,
    firebase_config: &FirebaseConfig,
    uid: &str,
    list_id: &str,
) -> Result<Vec<GroceryItem>, String> {
    let url = format!(
        "{}/users/{}/grocery/{}/items",
        firebase_config.firestore_url(),
        uid,
        list_id
    );

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch grocery items: {e}"))?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(vec![]);
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to fetch grocery items {status}: {body}"));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse grocery items response: {e}"))?;

    let docs = payload
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    Ok(docs.iter().map(parse_grocery_item_doc).collect())
}

async fn fetch_user_grocery_subcollection(
    client: &Client,
    firebase_config: &FirebaseConfig,
    uid: &str,
) -> Result<Vec<GroceryList>, String> {
    let url = format!("{}/users/{}/grocery", firebase_config.firestore_url(), uid);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch grocery lists: {e}"))?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(vec![]);
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to fetch grocery lists {status}: {body}"));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse grocery lists response: {e}"))?;

    let docs = payload
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let mut lists = Vec::new();
    for doc in docs {
        let list_id = doc
            .get("name")
            .and_then(|n| n.as_str())
            .map(extract_doc_id)
            .unwrap_or_default();

        if list_id.is_empty() {
            continue;
        }

        let items = fetch_grocery_items_subcollection(client, firebase_config, uid, &list_id)
            .await?;

        lists.push(GroceryList {
            list_id,
            list_name: get_doc_string_field(&doc, "list_name")
                .unwrap_or_else(|| "Unnamed List".to_string()),
            items,
        });
    }

    Ok(lists)
}

async fn fetch_user_people_subcollection(
    client: &Client,
    firebase_config: &FirebaseConfig,
    uid: &str,
) -> Result<Vec<PersonProfile>, String> {
    let url = format!("{}/users/{}/people", firebase_config.firestore_url(), uid);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch people: {e}"))?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(vec![]);
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Failed to fetch people {status}: {body}"));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse people response: {e}"))?;

    let docs = payload
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let people = docs
        .iter()
        .map(|doc| PersonProfile {
            person_id: doc
                .get("name")
                .and_then(|n| n.as_str())
                .map(extract_doc_id)
                .unwrap_or_default(),
            name: get_doc_string_field(doc, "name").unwrap_or_default(),
            preference: get_doc_string_array_field(doc, "preference"),
            allergy: get_doc_string_array_field(doc, "allergy"),
        })
        .collect();

    Ok(people)
}

//
pub async fn firestore_get_users_and_schedules(
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let client = Client::new();
    let url = format!("{}/users", firebase_config.firestore_url());

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            return Json(json!({"error": format!("Failed to call Firestore: {}", e)})).into_response();
        }
    };

    let users_payload: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            return Json(json!({"error": format!("Failed to parse Firestore response: {}", e)})).into_response();
        }
    };

    let documents = users_payload
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let mut users_map: HashMap<String, Vec<Schedule>> = HashMap::new();

    for user_doc in documents {
        let uid = user_doc
            .get("name")
            .and_then(|n| n.as_str())
            .map(extract_doc_id)
            .unwrap_or_default();
        if uid.is_empty() {
            continue;
        }

        let user_email = get_doc_string_field(&user_doc, "email").unwrap_or_default();
        if user_email.is_empty() {
            continue;
        }

        let schedules = match fetch_user_schedules_subcollection(&client, &firebase_config, &uid).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("⚠️ Failed to fetch schedules for {}: {}", uid, e);
                vec![]
            }
        };

        users_map.insert(user_email, schedules);
    }

    Json(users_map).into_response()
}

pub async fn handle_get_data(State(firebase_config): State<FirebaseConfig>) {
    let url = format!("{}/users", firebase_config.firestore_url());

    let response = reqwest::get(&url).await.expect("Failed to send request");
    let _ = response.text().await.expect("Failed to read response text");
}
pub async fn handle_post_feedback(
    _headers: HeaderMap,
    State(firebase_config): State<FirebaseConfig>,
    Json(payload): Json<FeedbackDocument>,
) {
    let url = format!("{}/feedback", firebase_config.firestore_url());

    let apirequestbody = serde_json::to_string(&payload).expect("Failed to serialize payload");

    call_firestore_post_feedback(&apirequestbody, &url).await;
}
async fn call_firestore_post_feedback(apirequestbody: &String, firestore_url: &str) {
    let url = format!("{}/feedbacks", firestore_url);
    let client = Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(apirequestbody.clone())
        .send()
        .await;

    match response {
        Ok(resp) => {}
        Err(e) => {}
    }
}
pub async fn handle_get_feedback_devread(
    Path(uuid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
) -> String {
    let firestore_url = format!("{}/feedbacks/{}", firebase_config.firestore_url(), uuid);
    let response: FeedbackDocument = call_firestore_get_feedback_devread(&uuid, &firestore_url)
        .await
        .unwrap_or(FeedbackDocument {
            fields: FeedbackFields {
                name: FirestoreStringField {
                    stringValue: "Error".to_string(),
                },
                status: FirestoreStringField {
                    stringValue: "Error".to_string(),
                },
                message: FirestoreStringField {
                    stringValue: "Error".to_string(),
                },
            },
        });
    uuid
}

pub async fn call_firestore_get_feedback_devread(
    uuid: &str,
    firestore_url: &str,
) -> Result<FeedbackDocument, Box<dyn std::error::Error>> {
    let client = Client::new();
    let resp = client
        .get(firestore_url)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(format!("Request failed with status: {}", resp.status()).into());
    }

    let json_text = resp.text().await?;
    let feedback: FeedbackDocument = serde_json::from_str(&json_text)?;

    Ok(feedback)
}

// Add this handler function to your firestore.rs file

pub async fn handle_get_user_by_uid(
    Path(uid): Path<String>,
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let client = Client::new();
    let (resolved_uid, firestore_response) = match resolve_user_document(&client, &firebase_config, &uid).await {
        Ok(result) => result,
        Err(e) => {
            eprintln!("❌ Failed to resolve user: {}", e);
            return Json(json!({ "error": "User not found or invalid UID" })).into_response();
        }
    };

    let email = get_doc_string_field(&firestore_response, "email").unwrap_or_default();
    let username = get_doc_string_field(&firestore_response, "username").unwrap_or_default();

    let schedules = match fetch_user_schedules_subcollection(&client, &firebase_config, &resolved_uid).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Failed to fetch schedules: {}", e);
            vec![]
        }
    };

    let saved_recipes = match fetch_user_savedrecipes_subcollection(&client, &firebase_config, &resolved_uid).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Failed to fetch saved recipes: {}", e);
            vec![]
        }
    };

    let grocery = match fetch_user_grocery_subcollection(&client, &firebase_config, &resolved_uid).await {
        Ok(lists) => lists,
        Err(e) => {
            eprintln!("❌ Failed to fetch grocery lists: {}", e);
            vec![]
        }
    };

    let people = match fetch_user_people_subcollection(&client, &firebase_config, &resolved_uid).await {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("❌ Failed to fetch people profiles: {}", e);
            vec![]
        }
    };

    let hardware = if let Some(hardware_field) = firestore_response
        .get("fields")
        .and_then(|f| f.get("hardware"))
        .and_then(|h| h.get("mapValue"))
        .and_then(|m| m.get("fields"))
    {
        let devices = hardware_field
            .get("devices")
            .and_then(|d| d.get("arrayValue"))
            .and_then(|a| a.get("values"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.get("stringValue").and_then(|s| s.as_str()).map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let requests = hardware_field
            .get("requests")
            .and_then(|d| d.get("arrayValue"))
            .and_then(|a| a.get("values"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.get("stringValue").and_then(|s| s.as_str()).map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Some(Hardware { devices, requests })
    } else {
        None
    };

    // Return JSON response matching your original format
    Json(json!({
        "email": email,
        "username": username,
        "savedrecipes": saved_recipes.into_iter().map(|recipe| json!({
            "uid": recipe.uid,
            "name": recipe.name,
            "ingredients": recipe.ingredients,
            "steps": recipe.steps,
            "time": recipe.time,
            "image_url": recipe.image_url,
            "quantity": recipe.quantity

        })).collect::<Vec<_>>(),
        "grocery": grocery.into_iter().map(|list| json!({
            "list_id": list.list_id,
            "list_name": list.list_name,
            "items": list.items.into_iter().map(|item| json!({
                "uid": item.uid,
                "name": item.name,
                "quantity": item.quantity,
                "unit": item.unit,
                "checked": item.checked,
            })).collect::<Vec<_>>()

        })).collect::<Vec<_>>(),
        "people": people.into_iter().map(|person| json!({
            "person_id": person.person_id,
            "name": person.name,
            "preference": person.preference,
            "allergy": person.allergy
        })).collect::<Vec<_>>(),
        "schedules": schedules.into_iter().map(|schedule| json!({
            "name": schedule.name,
            "time": schedule.time,
            "date": schedule.date,
            "expiry": schedule.expiry,
            "uid": schedule.uid,
            "category": schedule.category,
            "quantity": schedule.quantity,
            "location": schedule.location

        })).collect::<Vec<_>>(),
        "hardware": hardware.map(|hw| json!({
            "devices": hw.devices,
            "requests": hw.requests
        })).unwrap_or(json!(null))
    }))
    .into_response()
}

// Helper function to convert Firestore format to clean format
fn convert_firestore_user_to_clean(firestore_user: FirestoreSingleUserResponse, uid: &str) -> User {
    let fields = firestore_user.fields;

    // Extract username and email
    let username = fields.username.map(|u| u.stringValue);
    let email = fields.email.map(|e| e.stringValue);

    // Convert schedules
    let schedules = if let Some(schedule_map) = fields.schedules {
        convert_firestore_schedules_to_clean(schedule_map)
    } else {
        Vec::new()
    };

    // Convert saved recipes
    let saved_recipes = if let Some(recipe_map) = fields.savedrecipes {
        convert_firestore_recipes_to_clean(recipe_map)
    } else {
        Vec::new()
    };

    // Convert hardware
    let hardware = fields.hardware.map(|hardware_map| convert_firestore_hardware_to_clean(hardware_map));

    User {
        username,
        email,
        schedules,
        saved_recipes,
        hardware,
    }
}

// Helper function to convert Firestore schedules to clean format
fn convert_firestore_schedules_to_clean(schedule_map: FirestoreMapField) -> Vec<Schedule> {
    let mut schedules = Vec::new();
    let now = Local::now().with_timezone(&Bangkok);

    for (key, schedule_item) in schedule_map.map_value.fields {
        let schedule_detail = schedule_item.map_value.fields;

        // Safely extract optional string fields with defaults
        let name = schedule_detail
            .name
            .as_ref()
            .map(|n| n.stringValue.clone())
            .unwrap_or_default();
        let time = schedule_detail
            .time
            .as_ref()
            .map(|t| t.stringValue.clone())
            .unwrap_or_default();
        let date = schedule_detail
            .date
            .as_ref()
            .map(|d| d.stringValue.clone())
            .unwrap_or_default();
        let expiry = schedule_detail
            .expiry
            .as_ref()
            .map(|e| e.stringValue.clone())
            .unwrap_or_default();
        let uid = schedule_detail
            .uid
            .as_ref()
            .map(|u| u.stringValue.clone())
            .unwrap_or_default();
        let notify = schedule_detail
            .notify
            .as_ref()
            .map(|n| n.booleanValue)
            .unwrap_or(false);
        let category = schedule_detail
            .category
            .as_ref()
            .map(|c| c.stringValue.clone());
        let quantity = schedule_detail
            .quantity
            .as_ref()
            .map(|c| c.stringValue.clone());
        let location = schedule_detail
            .location
            .as_ref()
            .map(|l| l.stringValue.clone());

        // Check if schedule is soon (within next 30 minutes)

        schedules.push(Schedule {
            name,
            time,
            date,
            expiry,
            uid,
            notify,
            category,
            quantity,
            location,
        });
    }

    schedules
}

// Helper function to convert Firestore recipes to clean format
fn convert_firestore_recipes_to_clean(recipe_map: FirestoreRecipeMapField) -> Vec<Recipe> {
    let mut recipes = Vec::new();

    for (recipe_id, recipe_item) in recipe_map.map_value.fields {
        let recipe_fields = recipe_item.map_value.fields;

        let name = recipe_fields
            .name
            .as_ref()
            .map(|n| n.stringValue.clone())
            .unwrap_or_default();
        let time = recipe_fields.time.as_ref().map(|t| t.stringValue.clone());
        let image_url = recipe_fields
            .image_url
            .as_ref()
            .map(|t| t.stringValue.clone());

        // Extract ingredients array safely
        let ingredients = recipe_fields
            .ingredients
            .as_ref()
            .and_then(|ing| ing.array_value.as_ref())
            .map(|a| a.values.iter().map(|v| v.stringValue.clone()).collect())
            .unwrap_or_default();

        // Extract steps array safely
        let steps = recipe_fields
            .steps
            .as_ref()
            .and_then(|st| st.array_value.as_ref())
            .map(|a| a.values.iter().map(|v| v.stringValue.clone()).collect())
            .unwrap_or_default();

        let uid = recipe_fields
            .uid
            .as_ref()
            .map(|u| u.stringValue.clone())
            .unwrap_or_default();

        let quantity = recipe_fields
            .quantity
            .as_ref()
            .map(|q| q.stringValue.clone());

        recipes.push(Recipe {
            uid: recipe_id,
            name,
            ingredients,
            steps,
            time,
            image_url,
            quantity,
        });
    }

    recipes
}

// Helper function to convert Firestore hardware to clean format
fn convert_firestore_hardware_to_clean(hardware_map: FirestoreHardwareMapField) -> Hardware {
    let hardware_fields = hardware_map.map_value.fields;

    // Extract devices array safely
    let devices = hardware_fields
        .devices
        .as_ref()
        .and_then(|dev| dev.array_value.as_ref())
        .map(|a| a.values.iter().map(|v| v.stringValue.clone()).collect())
        .unwrap_or_default();

    // Extract requests array safely
    let requests = hardware_fields
        .requests
        .as_ref()
        .and_then(|req| req.array_value.as_ref())
        .map(|a| a.values.iter().map(|v| v.stringValue.clone()).collect())
        .unwrap_or_default();

    Hardware { devices, requests }
}

// Helper function to check if schedule is soon (reuse your existing logic)
fn check_if_schedule_is_soon(
    date_str: &str,
    time_str: &str,
    now: &chrono::DateTime<chrono_tz::Tz>,
) -> bool {
    // Parse the date
    let schedule_date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => return false,
    };

    // Parse the time (assuming format like "10-00" means "10:00")
    let time_parts: Vec<&str> = time_str.split('-').collect();
    if time_parts.len() != 2 {
        return false;
    }

    let hour = match time_parts[0].parse::<u32>() {
        Ok(h) => h,
        Err(_) => return false,
    };

    let minute = match time_parts[1].parse::<u32>() {
        Ok(m) => m,
        Err(_) => return false,
    };

    let schedule_time = match NaiveTime::from_hms_opt(hour, minute, 0) {
        Some(time) => time,
        None => return false,
    };

    // Combine date and time
    let schedule_datetime = schedule_date.and_time(schedule_time);
    let schedule_bangkok = Bangkok.from_local_datetime(&schedule_datetime).single();

    match schedule_bangkok {
        Some(schedule_tz) => {
            let diff = schedule_tz - *now;
            diff <= Duration::minutes(30) && diff >= Duration::zero()
        }
        None => false,
    }
}

pub async fn handle_get_user_summary(
    State(firebase_config): State<FirebaseConfig>,
) -> impl IntoResponse {
    let uid = "HUKkTGyzLHCwyioElWBK";
    let client = Client::new();
    let user_url = format!("{}/users/{}", firebase_config.firestore_url(), uid);

    let resp = match client.get(&user_url).send().await {
        Ok(r) => r,
        Err(_) => return Json(json!({ "error": "No user data found" })),
    };

    if !resp.status().is_success() {
        return Json(json!({ "error": "No user data found" }));
    }

    let user_doc: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => return Json(json!({ "error": "No user data found" })),
    };

    let saved_recipes = match fetch_user_savedrecipes_subcollection(&client, &firebase_config, uid).await {
        Ok(v) => v,
        Err(_) => vec![],
    };

    Json(json!({
        "email": get_doc_string_field(&user_doc, "email").unwrap_or_default(),
        "savedrecipes": saved_recipes.into_iter().map(|r| json!({
            "id": r.uid,
            "name": r.name,
            "ingredients": r.ingredients,
            "steps": r.steps
        })).collect::<Vec<_>>()
    }))
}
