use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct TheRequest {
    pub text: String,
}
#[derive(Serialize)]

pub struct TheResponse {
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FirestoreStringField {
    pub stringValue: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FeedbackFields {
    pub name: FirestoreStringField,
    pub status: FirestoreStringField,
    pub message: FirestoreStringField,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct FeedbackDocument {
    pub fields: FeedbackFields,
}
// For schedule reading
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreUserResponse {
    pub documents: Vec<FirestoreUserDocument>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreUserDocument {
    pub fields: Option<UserFields>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserFields {
    pub email: Option<FirestoreStringField>,
    pub schedules: Option<FirestoreMapField>,
    pub savedrecipes: Option<FirestoreMapField>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreMapField {
    #[serde(rename = "mapValue")]
    pub map_value: FirestoreInnerMap,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreInnerMap {
    pub fields: HashMap<String, FirestoreScheduleItem>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreScheduleItem {
    #[serde(rename = "mapValue")]
    pub map_value: FirestoreScheduleMap,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreScheduleMap {
    pub fields: FirestoreScheduleDetail,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreScheduleDetail {
    pub name: FirestoreStringField,
    pub time: FirestoreStringField,
    pub date: FirestoreStringField,
}

#[derive(Debug, Clone)]
pub struct Schedule {
    pub name: String,
    pub time: String,
    pub date: String,
    pub is_soon: bool,
}



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FirestoreArrayField {
    #[serde(rename = "arrayValue")]
    pub array_value: Option<FirestoreArrayValues>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FirestoreArrayValues {
    pub values: Vec<FirestoreStringField>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecipeField {
    pub name: FirestoreStringField,
    pub ingredients: FirestoreArrayField,
    pub steps: FirestoreArrayField,
    pub time: Option<FirestoreStringField>,
}