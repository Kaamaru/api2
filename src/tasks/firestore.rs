use crate::{
    firebase::{self, FirebaseConfig},
    structures::structs::{FeedbackDocument, FeedbackFields, FirestoreStringField},
};
use axum::{
    extract::{Json, Path},
    http::{response, HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::IntoResponse,
    routing::get,
    Json as AxumJson, Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fmt::format;

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
    let response: FeedbackDocument = call_firestore_get_feedback_devread(&uuid)
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
