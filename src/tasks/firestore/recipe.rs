use crate::firebase::FirebaseConfig;
use crate::structures::models::{DishName, RecipeData};
use std::collections::HashMap;
use serde_json::{json, Value};
use reqwest::Client;

/// Normalize string for storage: lowercase, remove spaces
fn normalize_alias(s: &str) -> String {
    s.to_lowercase().replace(" ", "")
}

/// Get recipe by ID from Firestore
pub async fn get_recipe_by_id(
    firebase_config: &FirebaseConfig,
    recipe_id: String,
) -> Result<Value, Box<dyn std::error::Error>> {
    let url = format!(
        "{}/recipes/{}",
        firebase_config.firestore_url(),
        recipe_id
    );
    
    let response = Client::new()
        .get(&url)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err("Recipe not found".into());
    }
    
    let data = response.json::<Value>().await?;
    Ok(data)
}

/// Load a recipe document and convert it into frontend-friendly recipe data.
pub async fn get_recipe_data_by_id(
    firebase_config: &FirebaseConfig,
    recipe_id: String,
) -> Result<RecipeData, Box<dyn std::error::Error>> {
    let document = get_recipe_by_id(firebase_config, recipe_id.clone()).await?;
    let fields = document
        .get("fields")
        .and_then(|v| v.as_object())
        .ok_or("recipe document missing fields")?;

    let name = firestore_string_field(fields, "name").unwrap_or_default();
    let ingredients = parse_entry_array(
        fields.get("ingredientsEntries"),
        "name",
        "amount",
    )
    .or_else(|| parse_map_field(fields.get("ingredients")));
    let nutrition = parse_entry_array(
        fields.get("nutrientsEntries"),
        "name",
        "value",
    )
    .or_else(|| parse_map_field(fields.get("nutrients")));

    Ok(RecipeData {
        id: document
            .get("name")
            .and_then(|v| v.as_str())
            .and_then(|path| path.rsplit('/').next())
            .unwrap_or(&recipe_id)
            .to_string(),
        name,
        ingredients,
        nutrition,
    })
}

/// Find recipe by normalized name in Firestore
pub async fn find_by_normalized(
    firebase_config: &FirebaseConfig,
    normalized: &str,
) -> Result<Option<Value>, Box<dyn std::error::Error>> {
    let url = format!(
        "{}/dishnames?where=normalized%3D%3D%22{}%22",
        firebase_config.firestore_url(),
        normalized
    );
    
    let response = Client::new()
        .get(&url)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Ok(None);
    }
    
    let data = response.json::<Value>().await?;
    
    // Check if any documents were returned
    if let Some(documents) = data.get("documents").and_then(|d| d.as_array()) {
        if !documents.is_empty() {
            return Ok(Some(documents[0].clone()));
        }
    }
    
    Ok(None)
}

/// Insert a new recipe into Firestore
/// Returns the document ID
pub async fn insert_recipe(
    firebase_config: &FirebaseConfig,
    name: String,
    ingredients: std::collections::HashMap<String, String>,
    calories: Option<String>,
    nutrients: Option<std::collections::HashMap<String, String>>,
    mut aliases: Vec<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    // Pre-normalize aliases and limit to 5
    aliases = aliases
        .into_iter()
        .take(5)
        .map(|a| normalize_alias(&a))
        .collect();
    
    let recipe_id = uuid::Uuid::new_v4().to_string();
    let url = format!(
        "{}/recipes?documentId={}&key={}",
        firebase_config.firestore_url(),
        recipe_id,
        firebase_config.api_key
    );
    
    let nutrients_map = nutrients.unwrap_or_default();
    let ingredients_raw_json = serde_json::to_string(&ingredients).unwrap_or_else(|_| "{}".to_string());
    let nutrients_raw_json = serde_json::to_string(&nutrients_map).unwrap_or_else(|_| "{}".to_string());

    let recipe_data = json!({
        "fields": {
            "name": { "stringValue": name },
            "normalized": { "stringValue": name.to_lowercase().replace(" ", "") },
            "ingredients": { "mapValue": { "fields": serialize_hashmap(&ingredients) } },
            "ingredientsEntries": {
                "arrayValue": {
                    "values": serialize_kv_entries(&ingredients, "name", "amount")
                }
            },
            "ingredientsRawJson": { "stringValue": ingredients_raw_json },
            "calories": { "stringValue": calories.unwrap_or_default() },
            "nutrients": {
                "mapValue": {
                    "fields": serialize_hashmap(&nutrients_map)
                }
            },
            "nutrientsEntries": {
                "arrayValue": {
                    "values": serialize_kv_entries(&nutrients_map, "name", "value")
                }
            },
            "nutrientsRawJson": { "stringValue": nutrients_raw_json },
            "aliases": {
                "arrayValue": {
                    "values": aliases.into_iter().map(|a| json!({ "stringValue": a })).collect::<Vec<_>>()
                }
            },
            "createdAt": { "timestampValue": chrono::Utc::now().to_rfc3339() },
            "source": { "stringValue": "gemini" }
        }
    });
    
    let response = Client::new()
        .post(&url)
        .json(&recipe_data)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "No body".to_string());
        eprintln!("Firestore insert_recipe failed - Status: {}, Body: {}", status, body);
        return Err("Failed to insert recipe".into());
    }
    
    Ok(recipe_id)
}

/// Insert a dish name (search index entry) into Firestore
/// Returns the document ID
pub async fn insert_dishname(
    firebase_config: &FirebaseConfig,
    recipe_id: String,
    name: String,
    normalized: String,
    aliases: Vec<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let doc_id = uuid::Uuid::new_v4().to_string();
    let url = format!(
        "{}/dishnames?documentId={}&key={}",
        firebase_config.firestore_url(),
        doc_id,
        firebase_config.api_key
    );
    
    let dish_data = json!({
        "fields": {
            "recipeId": { "stringValue": recipe_id },
            "name": { "stringValue": name },
            "normalized": { "stringValue": normalized },
            "aliases": {
                "arrayValue": {
                    "values": aliases.into_iter().map(|a| json!({ "stringValue": a })).collect::<Vec<_>>()
                }
            },
            "createdAt": { "timestampValue": chrono::Utc::now().to_rfc3339() }
        }
    });
    
    let response = Client::new()
        .post(&url)
        .json(&dish_data)
        .send()
        .await?;
    
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| "No body".to_string());
        eprintln!("Firestore insert_dishname failed - Status: {}, Body: {}", status, body);
        return Err("Failed to insert dish name".into());
    }
    
    Ok(doc_id)
}

/// Load all dish names from Firestore for cache initialization
pub async fn load_all_dishnames(
    firebase_config: &FirebaseConfig,
) -> Result<Vec<DishName>, Box<dyn std::error::Error>> {
    let url = format!("{}/dishnames", firebase_config.firestore_url());
    
    let response = Client::new()
        .get(&url)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Ok(Vec::new()); // Return empty cache on error
    }
    
    let data = response.json::<Value>().await?;
    let mut dishnames = Vec::new();
    
    if let Some(documents) = data.get("documents").and_then(|d| d.as_array()) {
        for doc in documents {
            if let Some(fields) = doc.get("fields").and_then(|f| f.as_object()) {
                let recipe_id = fields
                    .get("recipeId")
                    .and_then(|r| r.get("stringValue"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                
                let normalized = fields
                    .get("normalized")
                    .and_then(|n| n.get("stringValue"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                
                // Load aliases, pre-normalize them
                let aliases = fields
                    .get("aliases")
                    .and_then(|a| a.get("arrayValue"))
                    .and_then(|av| av.get("values"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|val| {
                                val.get("stringValue")
                                    .and_then(|s| s.as_str())
                                    .map(|s| normalize_alias(s))
                            })
                            .take(5)  // Limit to 5 aliases
                            .collect()
                    })
                    .unwrap_or_default();
                
                if !recipe_id.is_empty() && !normalized.is_empty() {
                    dishnames.push(DishName {
                        recipe_id,
                        normalized,
                        aliases,
                    });
                }
            }
        }
    }
    
    Ok(dishnames)
}

/// Helper: Convert HashMap to Firestore map field format
fn serialize_hashmap(map: &std::collections::HashMap<String, String>) -> serde_json::Map<String, Value> {
    let mut result = serde_json::Map::new();
    for (key, value) in map {
        let safe_key = next_unique_key(&result, &sanitize_firestore_key(key));
        result.insert(
            safe_key,
            json!({
                "stringValue": value
            }),
        );
    }
    result
}

fn serialize_kv_entries(
    map: &std::collections::HashMap<String, String>,
    key_name: &str,
    value_name: &str,
) -> Vec<Value> {
    map.iter()
        .map(|(k, v)| {
            json!({
                "mapValue": {
                    "fields": {
                        key_name: { "stringValue": k },
                        value_name: { "stringValue": v }
                    }
                }
            })
        })
        .collect()
}

fn firestore_string_field(fields: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    fields
        .get(key)
        .and_then(|v| v.get("stringValue"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn parse_map_field(field: Option<&Value>) -> Option<HashMap<String, String>> {
    let fields = field
        .and_then(|v| v.get("mapValue"))
        .and_then(|v| v.get("fields"))
        .and_then(|v| v.as_object())?;

    let mut result = HashMap::new();
    for (key, value) in fields {
        if let Some(string_value) = value.get("stringValue").and_then(|v| v.as_str()) {
            result.insert(key.clone(), string_value.to_string());
        }
    }

    if result.is_empty() { None } else { Some(result) }
}

fn parse_entry_array(
    field: Option<&Value>,
    key_name: &str,
    value_name: &str,
) -> Option<HashMap<String, String>> {
    let values = field
        .and_then(|v| v.get("arrayValue"))
        .and_then(|v| v.get("values"))
        .and_then(|v| v.as_array())?;

    let mut result = HashMap::new();
    for item in values {
        let fields = item
            .get("mapValue")
            .and_then(|v| v.get("fields"))
            .and_then(|v| v.as_object());

        if let Some(fields) = fields {
            let key = fields
                .get(key_name)
                .and_then(|v| v.get("stringValue"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let value = fields
                .get(value_name)
                .and_then(|v| v.get("stringValue"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if !key.is_empty() {
                result.insert(key.to_string(), value.to_string());
            }
        }
    }

    if result.is_empty() { None } else { Some(result) }
}

fn sanitize_firestore_key(input: &str) -> String {
    let mut s = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            s.push(ch.to_ascii_lowercase());
        } else {
            s.push('_');
        }
    }

    while s.contains("__") {
        s = s.replace("__", "_");
    }

    s = s.trim_matches('_').to_string();
    if s.is_empty() {
        s = "field".to_string();
    }

    let first = s.chars().next().unwrap_or('_');
    if !(first.is_ascii_alphabetic() || first == '_') {
        s.insert(0, '_');
    }
    s
}

fn next_unique_key(existing: &serde_json::Map<String, Value>, base: &str) -> String {
    if !existing.contains_key(base) {
        return base.to_string();
    }

    let mut i = 2usize;
    loop {
        let candidate = format!("{}_{}", base, i);
        if !existing.contains_key(&candidate) {
            return candidate;
        }
        i += 1;
    }
}
