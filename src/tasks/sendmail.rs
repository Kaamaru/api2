use std::collections::HashMap;

use crate::structures::structs::Schedule;
use crate::tasks::firestore::firestore_get_users_and_schedules;
use crate::{
    firebase::{self, FirebaseConfig},
    structures::structs::{FeedbackDocument, FeedbackFields, FirestoreStringField},
};
use lettre::message::{header::ContentType, Mailbox, Message};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{SmtpTransport, Transport};

pub async fn do_expiration_mail() {
    println!("Fetching data from Firestore...");

    // let users_map : HashMap<String, Vec<Schedule>> = firestore_get_users_and_schedules().await;


}

async fn send_mail(target: &str) {
    let my_email = "archnimaru@gmail.com";
    let app_password = "leje iyps unzu idte"; // No trailing space!
                                              // let target = "theetawat.arun@gmail.com";
                                              // let target = "peaparanut@gmail.com";

    let subject = "CookFetch Notification!";
    let body = " 
    
    ";

    let from = Mailbox::new(None, my_email.parse().unwrap());
    let to = Mailbox::new(Some("Receiver Name".to_string()), target.parse().unwrap());

    let email = Message::builder()
        .from(from)
        .to(to)
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(String::from(body))
        .unwrap();

    let creds = Credentials::new(my_email.to_string(), app_password.to_string());

    let mailer = SmtpTransport::relay("smtp.gmail.com")
        .unwrap()
        .credentials(creds)
        .build();

    match mailer.send(&email) {
        Ok(_) => println!("✅ Email sent successfully!"),
        Err(e) => eprintln!("❌ Failed to send email: {e}"),
    }
}
