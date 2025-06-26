use crate::{
    firebase::{self, FirebaseConfig},
    structures::structs::{
        FeedbackDocument, FeedbackFields, FirestoreMapField, FirestoreRecipeMapField,
        FirestoreScheduleDetail, FirestoreScheduleItem, FirestoreSingleUserResponse,
        FirestoreStringField, FirestoreUserDocument, FirestoreUserResponse, Recipe, RecipeField,
        Schedule, User,
    },
};
use axum::{
    extract::{Json, Path},
    http::{response, HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::get,
    Json as AxumJson, Router,
};
use chrono::{Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, fmt::format};

use chrono_tz::Asia::Bangkok;

pub async fn firestore_change_schedule(){
    
}
pub async fn firestore_change_recipe(){
    
}

//
pub async fn firestore_get_users_and_schedules()
//  -> HashMap<String, Vec<Schedule>>
{
    let url = format!("{}/users", firebase::firestore_url());

    let resp = reqwest::get(&url).await.expect("Failed to call Firestore");
    let text = resp.text().await.expect("Failed to read body");

    let parsed: FirestoreUserResponse =
        serde_json::from_str(&text).expect("Failed to parse Firestore response");

    let now = Bangkok
        .from_utc_datetime(&chrono::Utc::now().naive_utc())
        .naive_local();

    let mut users_map: HashMap<String, Vec<Schedule>> = HashMap::new();

    for doc in parsed.documents {
        if let Some(fields) = doc.fields {
            if let Some(email) = fields.email {
                let user_email = email.stringValue.clone();
                let mut schedule_list = Vec::new();

                if let Some(schedules) = fields.schedules {
                    for (_key, item) in schedules.map_value.fields {
                        let detail = item.map_value.fields;

                        let date =
                            match NaiveDate::parse_from_str(&detail.date.stringValue, "%Y-%m-%d") {
                                Ok(d) => d,
                                Err(_) => {
                                    eprintln!(
                                        "⚠️ Invalid date format: {}",
                                        detail.date.stringValue
                                    );
                                    continue;
                                }
                            };

                        let time =
                            match NaiveTime::parse_from_str(&detail.time.stringValue, "%H-%M") {
                                Ok(t) => t,
                                Err(_) => {
                                    eprintln!(
                                        "⚠️ Invalid time format: {}",
                                        detail.time.stringValue
                                    );
                                    continue;
                                }
                            };

                        let schedule_time = NaiveDateTime::new(date, time);
                        let time_diff = schedule_time - now;
                        let is_soon = time_diff.num_days() >= 0 && time_diff.num_days() < 3;

                        // Debug print (optional)
                        println!("time_difference: {:?}", time_diff);

                        schedule_list.push(Schedule {
                            name: detail.name.stringValue.clone(),
                            time: detail.time.stringValue.clone(),
                            date: detail.date.stringValue.clone(),
                            expiry: detail.date.stringValue.clone(),
                        });
                    }
                }

                users_map.insert(user_email, schedule_list);
            }
        }
    }

    // users_map
}

pub async fn handle_get_data() {
    let url = format!("{}/users", firebase::firestore_url());
    println!("{}", url);

    let response = reqwest::get(&url).await.expect("Failed to send request");
    println!(
        "{:?}",
        response.text().await.expect("Failed to read response text")
    );
}
pub async fn handle_post_feedback(headers: HeaderMap, Json(payload): Json<FeedbackDocument>) {
    let url = format!("{}/feedback", firebase::firestore_url());

    println!("{}!", payload.fields.message.stringValue);

    let apirequestbody = serde_json::to_string(&payload).expect("Failed to serialize payload");

    println!("Request Body: {:?}", apirequestbody);

    call_firestore_post_feedback(&apirequestbody).await;
}
async fn call_firestore_post_feedback(apirequestbody: &String) {
    let url = format!("{}/feedbacks", firebase::firestore_url());
    let client = Client::new();
    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(apirequestbody.clone())
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                println!("Feedback posted successfully!");
            } else {
                println!("Failed to post feedback: {}", resp.status());
            }
        }
        Err(e) => {
            println!("Error posting feedback: {}", e);
        }
    }
}
pub async fn handle_get_feedback_devread(Path(uuid): Path<String>) -> String {
    let response: FeedbackDocument =
        call_firestore_get_feedback_devread(&uuid)
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
    println!("name: {}", response.fields.name.stringValue);
    println!("message: {}", response.fields.message.stringValue);
    println!("status: {}", response.fields.status.stringValue);
    uuid
}

pub async fn call_firestore_get_feedback_devread(
    uuid: &str,
) -> Result<FeedbackDocument, Box<dyn std::error::Error>> {
    let url = format!("{}/feedbacks/{}", firebase::firestore_url(), uuid.trim());

    let client = Client::new();
    let resp = client
        .get(&url)
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

pub async fn handle_get_user_by_uid(Path(uid): Path<String>) -> impl IntoResponse {
    let url = format!("{}/users/{}", crate::firebase::firestore_url(), uid);

    // Get access token for authorization

    let client = Client::new();
    let response = match client.get(&url).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("Failed to fetch user: {}", e);
            return Json(json!({ "error": "User not found or invalid UID" })).into_response();
        }
    };

    if !response.status().is_success() {
        eprintln!("⚠️ Firestore responded with status: {}", response.status());
        return Json(json!({ "error": "User not found or invalid UID" })).into_response();
    }

    let body = match response.text().await {
        Ok(body) => {
            println!("{:?}", body);
            body
        }
        Err(e) => {
            eprintln!("Failed to get response body: {}", e);
            return Json(json!({ "error": "User not found or invalid UID" })).into_response();
        }
    };

    let firestore_response: FirestoreSingleUserResponse = match serde_json::from_str(&body) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("❌ Failed to parse user doc: {}", e);
            return Json(json!({ "error": "User not found or invalid UID" })).into_response();
        }
    };

    // Convert Firestore format to clean format
    let user = convert_firestore_user_to_clean(firestore_response, &uid);

    // Return JSON response matching your original format
    Json(json!({
        "email": user.email.unwrap_or_default(),
        "username": user.username.unwrap_or_default(),
        "savedrecipes": user.saved_recipes.into_iter().map(|recipe| json!({
            "id": recipe.id,
            "name": recipe.name,
            "ingredients": recipe.ingredients,
            "steps": recipe.steps,
            "time": recipe.time
        })).collect::<Vec<_>>(),
        "schedules": user.schedules.into_iter().map(|schedule| json!({
            "name": schedule.name,
            "time": schedule.time,
            "date": schedule.date,
            "expiry": schedule.expiry
        })).collect::<Vec<_>>()
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

    User {
        username,
        email,
        schedules,
        saved_recipes,
    }
}

// Helper function to convert Firestore schedules to clean format
fn convert_firestore_schedules_to_clean(schedule_map: FirestoreMapField) -> Vec<Schedule> {
    let mut schedules = Vec::new();
    let now = Local::now().with_timezone(&Bangkok);

    for (key, schedule_item) in schedule_map.map_value.fields {
        let schedule_detail = schedule_item.map_value.fields;

        let name = schedule_detail.name.stringValue;
        let time = schedule_detail.time.stringValue;
        let date = schedule_detail.date.stringValue;
        let expiry = schedule_detail.expiry.stringValue;

        // Check if schedule is soon (within next 30 minutes)

        schedules.push(Schedule {
            name,
            time,
            date,
            expiry,
        });
    }

    schedules
}

// Helper function to convert Firestore recipes to clean format
fn convert_firestore_recipes_to_clean(recipe_map: FirestoreRecipeMapField) -> Vec<Recipe> {
    let mut recipes = Vec::new();

    for (recipe_id, recipe_item) in recipe_map.map_value.fields {
        let recipe_fields = recipe_item.map_value.fields;

        let name = recipe_fields.name.stringValue;
        let time = recipe_fields.time.map(|t| t.stringValue);

        // Extract ingredients array
        let ingredients = if let Some(ingredients_field) = recipe_fields.ingredients.array_value {
            ingredients_field
                .values
                .into_iter()
                .map(|v| v.stringValue)
                .collect()
        } else {
            Vec::new()
        };

        // Extract steps array
        let steps = if let Some(steps_field) = recipe_fields.steps.array_value {
            steps_field
                .values
                .into_iter()
                .map(|v| v.stringValue)
                .collect()
        } else {
            Vec::new()
        };

        recipes.push(Recipe {
            id: recipe_id,
            name,
            ingredients,
            steps,
            time,
        });
    }

    recipes
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

pub async fn handle_get_user_summary() -> impl IntoResponse {
    let url = format!(
        "{}/users/HUKkTGyzLHCwyioElWBK",
        crate::firebase::firestore_url()
    );
    let resp = reqwest::get(&url).await.expect("Failed to fetch");
    let body = resp.text().await.expect("Failed to read body");

    let parsed: FirestoreUserResponse =
        serde_json::from_str(&body).expect("Failed to parse FirestoreUserResponse");

    if let Some(doc) = parsed.documents.first() {
        if let Some(fields) = &doc.fields {
            let mut saved_recipes = Vec::new();

            // ✅ Parse savedrecipes
            if let Some(saved) = &fields.savedrecipes {
                for (id, recipe_data) in &saved.map_value.fields {
                    let result: Result<RecipeField, _> = serde_json::from_value(
                        serde_json::to_value(&recipe_data.map_value.fields).unwrap(),
                    );

                    if let Ok(recipe_fields) = result {
                        let name = recipe_fields.name.stringValue.clone();
                        let ingredients = recipe_fields
                            .ingredients
                            .array_value
                            .as_ref()
                            .map(|a| {
                                a.values
                                    .iter()
                                    .map(|v| v.stringValue.clone())
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();

                        let steps = recipe_fields
                            .steps
                            .array_value
                            .as_ref()
                            .map(|a| {
                                a.values
                                    .iter()
                                    .map(|v| v.stringValue.clone())
                                    .collect::<Vec<_>>()
                            })
                            .unwrap_or_default();

                        saved_recipes.push(json!({
                            "id": id,
                            "name": name,
                            "ingredients": ingredients,
                            "steps": steps
                        }));
                    } else {
                        eprintln!("❌ Failed to parse recipe: {}", id);
                    }
                }
            }

            return Json(json!({
                "email": fields.email.as_ref().map(|e| e.stringValue.clone()).unwrap_or_default(),
                "savedrecipes": saved_recipes
            }));
        }
    }

    // fallback
    Json(json!({ "error": "No user data found" }))
}
