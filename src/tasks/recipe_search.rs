use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::Json as ResponseJson,
};
use chrono::Utc;
use dashmap::DashSet;
use std::collections::HashMap;
use std::sync::Arc;
use strsim::jaro_winkler;
use tokio::sync::RwLock;

use crate::firebase::FirebaseConfig;
use crate::structures::models::{
    AddRecipeToListRequest, CreateRecipeListFromInfoRequest, DishName, GenerateRecipeRequest, GeminiContent, GeminiPart,
    GeminiRequest, MatchInfo,
    RecipeData, SearchApiResponse, SearchQuery, SystemInstruction, GenerationConfig,
};
use crate::tasks::firestore::recipe::{get_recipe_data_by_id, insert_dishname, insert_recipe};
use crate::tasks::gemini::call_gemini_api;

#[derive(Clone, Debug)]
struct MatchResult {
    recipe_id: String,
    score: f64,
    matched_from: String,
}

#[derive(Clone)]
pub struct AppState {
    pub cache: Arc<RwLock<Vec<DishName>>>,
    pub generating: Arc<DashSet<String>>,
    pub firebase_config: Arc<FirebaseConfig>,
}

pub struct GenGuard {
    key: String,
    generating: Arc<DashSet<String>>,
}

impl GenGuard {
    pub fn new(key: String, generating: Arc<DashSet<String>>) -> Self {
        GenGuard { key, generating }
    }
}

impl Drop for GenGuard {
    fn drop(&mut self) {
        self.generating.remove(&self.key);
    }
}

fn normalize(s: &str) -> String {
    s.to_lowercase().replace(" ", "")
}

fn confidence_from_score(score: f64) -> Option<String> {
    if score > 0.95 {
        Some("exact".to_string())
    } else if score >= 0.85 {
        Some("high".to_string())
    } else {
        Some("low".to_string())
    }
}

fn find_best_match(input: &str, list: &[DishName]) -> Option<MatchResult> {
    let normalized_input = normalize(input);
    let mut best: Option<MatchResult> = None;

    for dish in list {
        let mut best_local_score = jaro_winkler(&normalized_input, &dish.normalized);
        let mut best_local_match = dish.normalized.clone();

        if best_local_score > 0.95 {
            if best.as_ref().map_or(true, |b| best_local_score > b.score) {
                best = Some(MatchResult {
                    recipe_id: dish.recipe_id.clone(),
                    score: best_local_score,
                    matched_from: best_local_match,
                });
            }
            continue;
        }

        for alias in dish.aliases.iter().take(5) {
            let score = jaro_winkler(&normalized_input, alias);
            if score > best_local_score {
                best_local_score = score;
                best_local_match = alias.clone();
                if score > 0.95 {
                    break;
                }
            }
        }

        if best.as_ref().map_or(true, |b| best_local_score > b.score) {
            best = Some(MatchResult {
                recipe_id: dish.recipe_id.clone(),
                score: best_local_score,
                matched_from: best_local_match,
            });
        }
    }

    match best {
        Some(result) if result.score > 0.85 => Some(result),
        _ => None,
    }
}

fn parse_quantity_unit(raw: &str) -> (String, String) {
    let parts: Vec<&str> = raw.split_whitespace().collect();
    if parts.len() >= 2 {
        (parts[0].to_string(), parts[1..].join(" "))
    } else {
        (raw.to_string(), "unit".to_string())
    }
}

async fn save_ingredients_to_grocery(
    firebase_config: &FirebaseConfig,
    userid: &str,
    list_id: &str,
    ingredients: &HashMap<String, String>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut saved_count = 0usize;

    for (ingredient_name, amount_raw) in ingredients {
        let item_id = format!("item_{}", uuid::Uuid::new_v4().simple());
        let (quantity, unit) = parse_quantity_unit(amount_raw);

        let post_url = format!(
            "{}/users/{}/grocery/{}/items?documentId={}",
            firebase_config.firestore_url(),
            userid,
            list_id,
            item_id
        );

        let payload = serde_json::json!({
            "fields": {
                "uid": { "stringValue": item_id.clone() },
                "name": { "stringValue": ingredient_name },
                "quantity": { "stringValue": quantity },
                "unit": { "stringValue": unit },
                "checked": { "booleanValue": false }
            }
        });

        let response = client.post(&post_url).json(&payload).send().await?;
        if response.status().is_success() {
            saved_count += 1;
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            eprintln!("⚠️ Failed to save ingredient '{}' to grocery: {} {}", ingredient_name, status, body);
        }
    }

    Ok(saved_count)
}

async fn generate_recipe_with_gemini(
    name: &str,
) -> Result<(HashMap<String, String>, HashMap<String, String>), Box<dyn std::error::Error>> {
    let prompt = format!(
        "You are a cooking assistant. Generate ONE recipe JSON for dish name: {}. Return only JSON with keys exactly: dishName, ingredients, calories, nutrients. ingredients and nutrients must be objects of string to string.",
        name
    );

    let request = GeminiRequest {
        system_instruction: Some(SystemInstruction {
            parts: vec![GeminiPart {
                text: Some("Return strict JSON only. No markdown.".to_string()),
                inline_data: None,
            }],
        }),
        contents: vec![GeminiContent {
            parts: vec![GeminiPart {
                text: Some(prompt),
                inline_data: None,
            }],
        }],
        generation_config: GenerationConfig {
            temperature: 0.4,
            max_output_tokens: 1500,
            top_p: 0.8,
            top_k: 20,
            response_mime_type: None,
            response_schema: None,
        },
    };

    let raw = call_gemini_api(&request).await?;
    let parsed: serde_json::Value = serde_json::from_str(&raw)?;

    fn value_to_string(v: &serde_json::Value) -> String {
        match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => String::new(),
            _ => v.to_string(),
        }
    }

    let ingredients = parsed
        .get("ingredients")
        .and_then(|v| v.as_object())
        .ok_or("missing ingredients object")?
        .iter()
        .map(|(k, v)| (k.clone(), value_to_string(v)))
        .collect::<HashMap<_, _>>();

    let mut nutrients = parsed
        .get("nutrients")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| (k.clone(), value_to_string(v)))
                .collect::<HashMap<String, String>>()
        })
        .unwrap_or_default();

    if let Some(calories) = parsed.get("calories") {
        nutrients.insert("calories".to_string(), value_to_string(calories));
    }

    eprintln!(
        "🧪 Gemini parsed recipe '{}' -> ingredients={}, nutrients={}",
        name,
        ingredients.len(),
        nutrients.len()
    );

    Ok((ingredients, nutrients))
}

pub async fn get_recipe_by_id_handler(
    Path(recipe_id): Path<String>,
    State(state): State<AppState>,
) -> Result<ResponseJson<SearchApiResponse>, StatusCode> {
    match get_recipe_data_by_id(&state.firebase_config, recipe_id.clone()).await {
        Ok(recipe) => Ok(ResponseJson(SearchApiResponse {
            status: "found".to_string(),
            data: Some(recipe),
            match_info: None,
            retry_after: None,
        })),
        Err(e) => {
            eprintln!("⚠️ Failed to load recipe by id '{}': {}", recipe_id, e);
            Ok(ResponseJson(SearchApiResponse {
                status: "failed".to_string(),
                data: None,
                match_info: None,
                retry_after: None,
            }))
        }
    }
}

pub async fn add_recipe_to_list_handler(
    Path(recipe_id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<AddRecipeToListRequest>,
) -> Result<ResponseJson<serde_json::Value>, StatusCode> {
    if payload.userid.trim().is_empty() || payload.list_id.trim().is_empty() {
        return Ok(ResponseJson(serde_json::json!({
            "status": "failed",
            "message": "userid and list_id are required"
        })));
    }

    let recipe = match get_recipe_data_by_id(&state.firebase_config, recipe_id.clone()).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("⚠️ Failed to load recipe '{}' for add-to-list: {}", recipe_id, e);
            return Ok(ResponseJson(serde_json::json!({
                "status": "failed",
                "message": "recipe_not_found"
            })));
        }
    };

    let ingredients = recipe.ingredients.clone().unwrap_or_default();
    if ingredients.is_empty() {
        return Ok(ResponseJson(serde_json::json!({
            "status": "failed",
            "message": "recipe_has_no_ingredients"
        })));
    }

    let saved_count = match save_ingredients_to_grocery(
        &state.firebase_config,
        &payload.userid,
        &payload.list_id,
        &ingredients,
    )
    .await
    {
        Ok(count) => count,
        Err(e) => {
            eprintln!(
                "⚠️ Failed to add recipe '{}' to list '{}': {}",
                recipe_id, payload.list_id, e
            );
            return Ok(ResponseJson(serde_json::json!({
                "status": "failed",
                "message": "failed_to_add_recipe_to_list"
            })));
        }
    };

    if saved_count == 0 {
        return Ok(ResponseJson(serde_json::json!({
            "status": "failed",
            "message": "no_items_added",
            "data": {
                "recipe_id": recipe_id,
                "list_id": payload.list_id,
                "userid": payload.userid
            }
        })));
    }

    eprintln!(
        "✅ Added recipe '{}' to grocery list '{}': items_added={}",
        recipe_id, payload.list_id, saved_count
    );

    Ok(ResponseJson(serde_json::json!({
        "status": "success",
        "message": "recipe_added_to_list",
        "data": {
            "recipe_id": recipe_id,
            "recipe_name": recipe.name,
            "userid": payload.userid,
            "list_id": payload.list_id,
            "items_added": saved_count
        }
    })))
}

pub async fn add_recipe_info_to_list_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateRecipeListFromInfoRequest>,
) -> Result<ResponseJson<serde_json::Value>, StatusCode> {
    if payload.userid.trim().is_empty() {
        return Ok(ResponseJson(serde_json::json!({
            "status": "failed",
            "message": "userid is required"
        })));
    }

    if payload.ingredients.is_empty() {
        return Ok(ResponseJson(serde_json::json!({
            "status": "failed",
            "message": "ingredients are required"
        })));
    }

    let list_name = payload
        .list_name
        .clone()
        .unwrap_or_else(|| format!("Recipe - {}", payload.recipe_name));

    let list_id = format!(
        "grocery_{}_{}",
        uuid::Uuid::new_v4().simple(),
        Utc::now().format("%Y%m%dT%H%M%SZ")
    );

    let create_url = format!(
        "{}/users/{}/grocery?documentId={}",
        state.firebase_config.firestore_url(),
        payload.userid,
        list_id
    );

    let create_payload = serde_json::json!({
        "fields": {
            "list_name": { "stringValue": list_name.clone() }
        }
    });

    let client = reqwest::Client::new();
    let create_response = match client.post(&create_url).json(&create_payload).send().await {
        Ok(response) => response,
        Err(e) => {
            eprintln!("⚠️ Failed to create grocery list from recipe info: {}", e);
            return Ok(ResponseJson(serde_json::json!({
                "status": "failed",
                "message": "failed_to_create_list"
            })));
        }
    };

    if !create_response.status().is_success() {
        let status = create_response.status();
        let body = create_response.text().await.unwrap_or_default();
        eprintln!("⚠️ Create list failed: {}\n{}", status, body);
        return Ok(ResponseJson(serde_json::json!({
            "status": "failed",
            "message": "failed_to_create_list",
            "details": body
        })));
    }

    let saved_count = match save_ingredients_to_grocery(
        &state.firebase_config,
        &payload.userid,
        &list_id,
        &payload.ingredients,
    )
    .await
    {
        Ok(count) => count,
        Err(e) => {
            eprintln!(
                "⚠️ Failed to seed new list '{}' with recipe info: {}",
                list_id, e
            );
            return Ok(ResponseJson(serde_json::json!({
                "status": "failed",
                "message": "failed_to_seed_new_list"
            })));
        }
    };

    if saved_count == 0 {
        return Ok(ResponseJson(serde_json::json!({
            "status": "failed",
            "message": "no_items_added",
            "data": {
                "list_id": list_id,
                "userid": payload.userid
            }
        })));
    }

    eprintln!(
        "✅ Created grocery list '{}' from recipe info '{}': items_added={}",
        list_id,
        payload.recipe_name,
        saved_count
    );

    Ok(ResponseJson(serde_json::json!({
        "status": "success",
        "message": "recipe_list_created_from_info",
        "data": {
            "recipe_name": payload.recipe_name,
            "userid": payload.userid,
            "list_id": list_id,
            "list_name": list_name,
            "items_added": saved_count,
            "nutrition_received": payload.nutrition.as_ref().map(|n| !n.is_empty()).unwrap_or(false)
        }
    })))
}

pub async fn search_recipe_handler(
    Query(params): Query<SearchQuery>,
    State(state): State<AppState>,
) -> Result<ResponseJson<SearchApiResponse>, StatusCode> {
    let normalized_query = normalize(&params.name);

    {
        let cache = state.cache.read().await;
        if let Some(match_result) = find_best_match(&normalized_query, &cache) {
            return Ok(ResponseJson(SearchApiResponse {
                status: "found".to_string(),
                data: Some(RecipeData {
                    id: match_result.recipe_id.clone(),
                    name: params.name.clone(),
                    ingredients: None,
                    nutrition: None,
                }),
                match_info: Some(MatchInfo {
                    matched_from: match_result.matched_from,
                    score: match_result.score,
                    confidence: confidence_from_score(match_result.score),
                }),
                retry_after: None,
            }));
        }
    }

    if state.generating.contains(&normalized_query) {
        return Ok(ResponseJson(SearchApiResponse {
            status: "generating".to_string(),
            data: None,
            match_info: None,
            retry_after: Some(2),
        }));
    }

    Ok(ResponseJson(SearchApiResponse {
        status: "not_found".to_string(),
        data: None,
        match_info: None,
        retry_after: None,
    }))
}

pub async fn generate_recipe_handler(
    State(state): State<AppState>,
    Json(payload): Json<GenerateRecipeRequest>,
) -> Result<ResponseJson<SearchApiResponse>, StatusCode> {
    let normalized_query = normalize(&payload.name);

    {
        let cache = state.cache.read().await;
        if let Some(match_result) = find_best_match(&normalized_query, &cache) {
            return Ok(ResponseJson(SearchApiResponse {
                status: "found".to_string(),
                data: Some(RecipeData {
                    id: match_result.recipe_id,
                    name: payload.name.clone(),
                    ingredients: None,
                    nutrition: None,
                }),
                match_info: Some(MatchInfo {
                    matched_from: match_result.matched_from,
                    score: match_result.score,
                    confidence: confidence_from_score(match_result.score),
                }),
                retry_after: None,
            }));
        }
    }

    if state.generating.contains(&normalized_query) {
        return Ok(ResponseJson(SearchApiResponse {
            status: "generating".to_string(),
            data: None,
            match_info: None,
            retry_after: Some(2),
        }));
    }

    let _guard = GenGuard::new(normalized_query.clone(), state.generating.clone());
    state.generating.insert(normalized_query.clone());

    let (ingredients, nutrients) = match generate_recipe_with_gemini(&payload.name).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("⚠️ Gemini generation failed: {}", e);
            return Ok(ResponseJson(SearchApiResponse {
                status: "failed".to_string(),
                data: None,
                match_info: None,
                retry_after: None,
            }));
        }
    };

    let calories = nutrients.get("calories").cloned();

    let recipe_id = match insert_recipe(
        &state.firebase_config,
        payload.name.clone(),
        ingredients.clone(),
        calories,
        Some(nutrients.clone()),
        vec![],
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            eprintln!("⚠️ Failed to insert recipe into Firestore: {}", e);
            return Ok(ResponseJson(SearchApiResponse {
                status: "failed".to_string(),
                data: None,
                match_info: None,
                retry_after: None,
            }));
        }
    };

    let dishname_id = match insert_dishname(
        &state.firebase_config,
        recipe_id.clone(),
        payload.name.clone(),
        normalized_query.clone(),
        vec![],
    )
    .await
    {
        Ok(id) => id,
        Err(e) => {
            eprintln!("⚠️ Failed to insert dish name into Firestore: {}", e);
            String::new()
        }
    };

    if !dishname_id.is_empty() {
        eprintln!("✅ Dishname persisted to Firestore: dishname_id='{}', recipe_id='{}'", dishname_id, recipe_id);
    }

    {
        let mut cache = state.cache.write().await;
        cache.push(DishName {
            recipe_id: recipe_id.clone(),
            normalized: normalized_query,
            aliases: vec![],
        });
    }

    eprintln!(
        "✅ Recipe persisted to Firestore: recipe_id='{}', name='{}', ingredients={}, nutrients={}",
        recipe_id,
        payload.name,
        ingredients.len(),
        nutrients.len()
    );

    if payload.save_to_grocery.unwrap_or(false) {
        if let (Some(userid), Some(list_id)) = (payload.userid.as_deref(), payload.list_id.as_deref()) {
            if let Err(e) = save_ingredients_to_grocery(&state.firebase_config, userid, list_id, &ingredients).await {
                eprintln!("⚠️ Failed to save generated ingredients to grocery: {}", e);
            }
        } else {
            return Ok(ResponseJson(SearchApiResponse {
                status: "failed".to_string(),
                data: None,
                match_info: None,
                retry_after: None,
            }));
        }
    }

    Ok(ResponseJson(SearchApiResponse {
        status: "found".to_string(),
        data: Some(RecipeData {
            id: recipe_id,
            name: payload.name,
            ingredients: Some(ingredients),
            nutrition: Some(nutrients),
        }),
        match_info: None,
        retry_after: None,
    }))
}
