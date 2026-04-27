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
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FirestoreBooleanField {
    pub booleanValue: bool,
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
pub struct FirestoreMapField {
    #[serde(rename = "mapValue")]
    pub map_value: FirestoreInnerMap,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreInnerMap {
    #[serde(default)]
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
    #[serde(default)]
    pub name: Option<FirestoreStringField>,
    #[serde(default)]
    pub time: Option<FirestoreStringField>,
    #[serde(default)]
    pub date: Option<FirestoreStringField>,
    #[serde(default)]
    pub expiry: Option<FirestoreStringField>,
    #[serde(default)]
    pub uid: Option<FirestoreStringField>,
    #[serde(default)]
    pub notify: Option<FirestoreBooleanField>,
    #[serde(default)]
    pub category: Option<FirestoreStringField>,
    #[serde(default)]
    pub quantity: Option<FirestoreStringField>,
    #[serde(default)]
    pub location: Option<FirestoreStringField>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Schedule {
    pub name: String,
    pub time: String,
    pub date: String,
    pub expiry: String, // Added expiry field
    pub uid: String,
    pub notify: bool,
    pub category: Option<String>,
    pub quantity: Option<String>,
    pub location: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FirestoreArrayField {
    #[serde(rename = "arrayValue")]
    pub array_value: Option<FirestoreArrayValues>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FirestoreArrayValues {
    #[serde(default)]
    pub values: Vec<FirestoreStringField>,
}
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RecipeField {
    #[serde(default)]
    pub name: Option<FirestoreStringField>,
    #[serde(default)]
    pub ingredients: Option<FirestoreArrayField>,
    #[serde(default)]
    pub steps: Option<FirestoreArrayField>,
    #[serde(default)]
    pub time: Option<FirestoreStringField>,
    #[serde(default)]
    pub uid: Option<FirestoreStringField>,
    #[serde(default)]
    pub image_url: Option<FirestoreStringField>,
    #[serde(default)]
    pub quantity: Option<FirestoreStringField>,
}

// Updated UserFields to include username and better recipe handling
#[derive(Serialize, Deserialize, Debug)]
pub struct UserFields {
    pub email: Option<FirestoreStringField>,
    pub username: Option<FirestoreStringField>,
    pub schedules: Option<FirestoreMapField>,
    pub savedrecipes: Option<FirestoreRecipeMapField>,
    pub hardware: Option<FirestoreHardwareMapField>,
    pub grocery: Option<FirestoreGroceryMapField>,
}

// New struct specifically for saved recipes map
#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreRecipeMapField {
    #[serde(rename = "mapValue")]
    pub map_value: FirestoreRecipeInnerMap,
}

// New struct for hardware map field (simpler structure - single map with arrays)
#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreHardwareMapField {
    #[serde(rename = "mapValue")]
    pub map_value: HardwareField,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HardwareField {
    #[serde(default)]
    pub fields: HardwareFieldsInner,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct HardwareFieldsInner {
    #[serde(default)]
    pub devices: Option<FirestoreArrayField>,
    #[serde(default)]
    pub requests: Option<FirestoreArrayField>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreRecipeInnerMap {
    #[serde(default)]
    pub fields: HashMap<String, FirestoreRecipeItem>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreRecipeItem {
    #[serde(rename = "mapValue")]
    pub map_value: FirestoreRecipeMap,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreRecipeMap {
    #[serde(default)]
    pub fields: RecipeField,
}

// Complete user document structure for single document response
#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreSingleUserResponse {
    pub name: String,
    pub fields: UserFields,
    #[serde(rename = "createTime")]
    pub create_time: String,
    #[serde(rename = "updateTime")]
    pub update_time: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct Recipe {
    pub uid: String,
    pub name: String,
    pub ingredients: Vec<String>,
    pub steps: Vec<String>,
    pub time: Option<String>,
    pub image_url: Option<String>,
    pub quantity: Option<String>,
}

// Hardware struct for API response
#[derive(Serialize, Debug, Clone)]
pub struct Hardware {
    pub devices: Vec<String>,
    pub requests: Vec<String>,
}

// Clean structs for API response
#[derive(Serialize, Debug)]
pub struct User {
    pub username: Option<String>,
    pub email: Option<String>,
    pub schedules: Vec<Schedule>,
    pub saved_recipes: Vec<Recipe>,
    pub hardware: Option<Hardware>,
}

#[derive(Serialize)]
pub struct FirestoreChangeSchedule {
    pub name: Option<String>,
    pub time: Option<String>,
    pub date: Option<String>,
    pub expiry: Option<String>,
    pub notify: Option<bool>,
    pub category: Option<String>,
    pub quantity: Option<String>,
    pub location: Option<String>,
}
#[derive(serde::Deserialize)]
pub struct AddScheduleRequest {
    pub name: String,
    pub time: String,
    pub date: String,
    pub expiry: String,
    pub notify: bool,
    pub category: Option<String>,
    pub quantity: String,
    pub location: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct AddRecipeRequest {
    pub name: String,
    pub ingredients: Vec<String>,
    pub steps: Vec<String>,
    pub time: Option<String>,
    pub image_url: Option<String>,
    pub quantity: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GroceryToScheduleRequest {
    pub date: String,       // YYYY-MM-DD
    pub time: String,       // HH-MM
    pub category: Option<String>,
    pub expiry: String,     // YYYY-MM-DD
    pub location: Option<String>,
}

// ===== GROCERY SYSTEM STRUCTURES =====

// Firestore grocery item field
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GroceryItemField {
    #[serde(default)]
    pub name: Option<FirestoreStringField>,
    #[serde(default)]
    pub quantity: Option<FirestoreStringField>,
    #[serde(default)]
    pub unit: Option<FirestoreStringField>,
    #[serde(default)]
    pub checked: Option<FirestoreBooleanField>,
    #[serde(default)]
    pub uid: Option<FirestoreStringField>,
}

// API response grocery item
#[derive(Serialize, Debug, Clone)]
pub struct GroceryItem {
    pub uid: String,
    pub name: String,
    pub quantity: String,
    pub unit: String,
    pub checked: bool,
}

// API response grocery list
#[derive(Serialize, Debug, Clone)]
pub struct GroceryList {
    pub list_id: String,
    pub list_name: String,
    pub items: Vec<GroceryItem>,
}

// Firestore grocery item wrapper
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FirestoreGroceryItem {
    #[serde(rename = "mapValue")]
    pub map_value: FirestoreGroceryItemMap,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FirestoreGroceryItemMap {
    pub fields: GroceryItemField,
}

// Firestore inner map for items within a list
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FirestoreGroceryListInnerMap {
    #[serde(default)]
    pub fields: HashMap<String, FirestoreGroceryItem>,
}

// Firestore grocery list item (contains items)
#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreGroceryListItem {
    #[serde(rename = "mapValue")]
    pub map_value: FirestoreGroceryListMap,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreGroceryListMap {
    pub fields: GroceryListField,
}

// The fields that define a grocery list
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GroceryListField {
    #[serde(default)]
    pub list_name: Option<FirestoreStringField>,
    #[serde(default)]
    pub items: Option<FirestoreGroceryItemsMapField>,
}

// Wrapper for the items map within a grocery list
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FirestoreGroceryItemsMapField {
    #[serde(rename = "mapValue")]
    pub map_value: FirestoreGroceryListInnerMap,
}

// Firestore top-level grocery map (maps list_id -> GroceryList)
#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreGroceryMapField {
    #[serde(rename = "mapValue")]
    pub map_value: FirestoreGroceryInnerMap,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FirestoreGroceryInnerMap {
    #[serde(default)]
    pub fields: HashMap<String, FirestoreGroceryListItem>,
}

// Request: Create a new grocery list
#[derive(serde::Deserialize)]
pub struct CreateGroceryListRequest {
    pub list_name: String,
}

// Request: Add item to grocery list
#[derive(serde::Deserialize)]
pub struct AddGroceryItemRequest {
    pub name: String,
    pub quantity: String,
    pub unit: String,
}

// Request: Update grocery item (toggle checked, etc)
#[derive(serde::Deserialize, serde::Serialize)]
pub struct UpdateGroceryItemRequest {
    pub checked: Option<bool>,
    pub quantity: Option<String>,
    pub unit: Option<String>,
}

// ===== PEOPLE PREFERENCES STRUCTURES =====

#[derive(Serialize, Debug, Clone)]
pub struct PersonProfile {
    pub person_id: String,
    pub name: String,
    pub preference: Vec<String>,
    pub allergy: Vec<String>,
}

#[derive(serde::Deserialize)]
pub struct CreatePersonRequest {
    pub name: String,
    #[serde(default)]
    pub preference: Vec<String>,
    #[serde(default)]
    pub allergy: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct UpdatePersonRequest {
    pub name: Option<String>,
    pub preference: Option<Vec<String>>,
    pub allergy: Option<Vec<String>>,
}
