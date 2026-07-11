use std::collections::HashMap;

use crate::structures::structs::{Schedule, FirestoreUserResponse};
use crate::tasks::firestore::notify::firestore_get_notified_users_schedules;
use crate::tasks::line::notify_line_expiring;
use crate::{
    firebase::FirebaseConfig,
    structures::structs::{FeedbackDocument, FeedbackFields, FirestoreStringField},
};
use chrono::{self, TimeZone};
use lettre::message::{header::ContentType, Mailbox, Message};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport};
use reqwest::Client;
use serde_json::{json, Value};
use std::env;

pub async fn do_expiration_mail() {
    println!("🔔 Starting expiration mail check...");
    
    dotenv::dotenv().ok();

    // Get the notified users and schedules
    let users = match firestore_get_notified_users_schedules().await {
        Ok(users_list) => {println!("Fetched users: {:?}", users_list); users_list},
        Err(e) => {
            eprintln!("❌ Failed to fetch notified users: {}", e);
            return;
        }
    };

    for user_data in users {
        let user_id = match user_data.get("user_id").and_then(|v| v.as_str()) {
            Some(id) => {println!("{}", id); id},
            None => continue,
        };

        let schedules = match user_data.get("schedules") {
            Some(Value::Array(scheds)) => scheds,
            _ => continue,
        };

        let mut expiring_items = Vec::new();

        for schedule in schedules {
            // Check if has expiry and if it's expiring soon
            if let Some(expiry_str) = schedule.get("expiry").and_then(|v| v.as_str()) {
                if expiry_str.is_empty() {
                    continue;
                }

                // Try parsing expiry date in multiple formats
                let expiry_date = if let Ok(date) = chrono::DateTime::parse_from_rfc3339(expiry_str) {
                    date.with_timezone(&chrono::Utc)
                } else if let Ok(date) = chrono::NaiveDate::parse_from_str(expiry_str, "%Y-%m-%d") {
                    // Parse as date only and convert to datetime at midnight
                    chrono::Utc.from_utc_datetime(
                        &chrono::NaiveDateTime::new(date, chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                    )
                } else {
                    continue;
                };

                let now = chrono::Utc::now();
                let time_until_expiry = expiry_date - now;

                // Alert if expiring within 24 hours or already expired
                if time_until_expiry.num_hours() <= 24 && time_until_expiry.num_seconds() >= 0 {
                    expiring_items.push(schedule.clone());
                }
            }
        }

        if !expiring_items.is_empty() {
            println!("📧 Found {} expiring items for {}", expiring_items.len(), user_id);
            println!("📧 Sending expiration alert to {}", user_id);
            send_mail(user_id, &expiring_items).await;
        } else {
            println!("⏭️ No expiring items found for {}", user_id);
        }

        // Also push a LINE reminder to any user who has linked their LINE account.
        // Reuses the same expiring-items query (schedules already filtered to the
        // notified, soon-to-expire set) rather than issuing a separate query.
        let line_user_id = user_data
            .get("line_user_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if !line_user_id.is_empty() && !schedules.is_empty() {
            match notify_line_expiring(line_user_id, schedules).await {
                Ok(()) => println!("📱 Sent LINE expiry reminder for {}", user_id),
                Err(e) => eprintln!("❌ Failed to send LINE reminder for {}: {}", user_id, e),
            }
        }
    }

    println!("✅ Expiration mail check completed!");
}

async fn send_mail(target: &str, expiring_items: &[Value]) {
    println!("✉️ Preparing to send email to {}", target);
    let my_email = match env::var("COOKFETCH_NOTIFY_USERNAME") {
        Ok(email) => email,
        Err(_) => {
            eprintln!("❌ COOKFETCH_NOTIFY_USERNAME not found in .env");
            return;
        }
    };
    // Read app password from .env
    let app_password = match env::var("COOKFETCH_NOTIFY_APP_PASSWORD") {
        Ok(pwd) => pwd,
        Err(_) => {
            eprintln!("❌ COOKFETCH_NOTIFY_APP_PASSWORD not found in .env");
            return;
        }
    };

    let subject = "🔔 CookFetch - Items Expiring Soon!";
    
    // Build email body with expiring items
    let mut body = String::from(
        "Hi there!\n\n\
        Your following items are expiring soon:\n\n"
    );

    for item in expiring_items {
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
        let date = item.get("date").and_then(|v| v.as_str()).unwrap_or("N/A");
        let time = item.get("time").and_then(|v| v.as_str()).unwrap_or("N/A");
        let expiry = item.get("expiry").and_then(|v| v.as_str()).unwrap_or("N/A");
        let category = item.get("category").and_then(|v| v.as_str()).unwrap_or("Uncategorized");

        body.push_str(&format!(
            "🍽️  {}\n\
            📅 Date: {}\n\
            ⏰ Time: {}\n\
            ❗ Expiry: {}\n\
            🏷️  Category: {}\n\n",
            name, date, time, expiry, category
        ));
    }

    body.push_str(
        "Please consume or discard these items soon!\n\n\
        Best regards,\n\
        CookFetch Team 🍳"
    );

    let from = Mailbox::new(None, my_email.parse().unwrap());
    let to = Mailbox::new(Some("CookFetch User".to_string()), target.parse().unwrap());

    let email = Message::builder()
        .from(from)
        .to(to)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body)
        .unwrap();

    let creds = Credentials::new(my_email.to_string(), app_password);

    let mailer = SmtpTransport::relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    match mailer.send(&email) {
        Ok(_) => println!("✅ Expiration email sent successfully to {}!", target),
        Err(e) => eprintln!("❌ Failed to send email to {}: {e}", target),
    }
}
