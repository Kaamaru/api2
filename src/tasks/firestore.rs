use crate::{
    firebase::{self, FirebaseConfig},
    structures::structs::{
        FeedbackDocument, FeedbackFields, FirestoreScheduleDetail, FirestoreScheduleItem,
        FirestoreStringField, FirestoreUserResponse, Schedule,
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
use std::{collections::HashMap, fmt::format};

use chrono_tz::Asia::Bangkok;

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
                            is_soon,
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
