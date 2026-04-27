use crate::firebase::{FirebaseConfig, GeminiConfig};
use crate::structures::models::{
    ChatRequest, ClassifyResponse, GeminiContent, GeminiPart, GeminiRequest, GeminiResponse,
    GenerationConfig, InlineImageData, SystemInstruction,
};
use crate::tasks::firestore::recipe::{insert_dishname, insert_recipe};
use crate::tasks::getimage::get_one_image;
use axum::{extract::{Json, Multipart}, http::StatusCode, response::Json as ResponseJson};
use image::{self, codecs::jpeg::JpegEncoder, imageops::FilterType};
use reqwest::Client;
use serde_json::Value;
use serde::Deserialize;

fn extract_doc_id(name: &str) -> String {
    name.rsplit('/').next().unwrap_or_default().to_string()
}

fn get_doc_string_field(doc: &Value, key: &str) -> Option<String> {
    doc.get("fields")
        .and_then(|f| f.get(key))
        .and_then(|v| v.get("stringValue"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
}

fn get_doc_string_array_field(doc: &Value, key: &str) -> Vec<String> {
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

async fn build_people_prompt_context(
    userid: &str,
    selected_people_ids: &[String],
) -> Result<String, ()> {
    if selected_people_ids.is_empty() {
        return Ok(String::new());
    }

    let firebase_config = FirebaseConfig::from_env();
    let url = format!(
        "{}/users/{}/people",
        firebase_config.firestore_url(),
        userid
    );

    let client = Client::new();
    let response = client.get(&url).send().await.map_err(|_| ())?;
    if !response.status().is_success() {
        return Ok(String::new());
    }

    let payload: Value = response.json().await.map_err(|_| ())?;
    let docs = payload
        .get("documents")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();

    let mut selected = Vec::new();
    for doc in docs {
        let person_id = doc
            .get("name")
            .and_then(|n| n.as_str())
            .map(extract_doc_id)
            .unwrap_or_default();

        if !selected_people_ids.iter().any(|id| id == &person_id) {
            continue;
        }

        let name = get_doc_string_field(&doc, "name").unwrap_or_else(|| "Unknown".to_string());
        let preference = get_doc_string_array_field(&doc, "preference");
        let allergy = get_doc_string_array_field(&doc, "allergy");

        selected.push((name, preference, allergy));
    }

    if selected.is_empty() {
        return Ok(String::new());
    }

    let mut lines = vec![
        "Additional user instructions based on selected people preferences:".to_string(),
    ];
    for (name, preference, allergy) in selected {
        let prefs = if preference.is_empty() {
            "none".to_string()
        } else {
            preference.join(", ")
        };
        let allergies = if allergy.is_empty() {
            "none".to_string()
        } else {
            allergy.join(", ")
        };
        lines.push(format!(
            "- {}: preferences [{}], allergies [{}]",
            name, prefs, allergies
        ));
    }

    Ok(lines.join("\n"))
}

#[derive(Deserialize)]
pub struct ClassifyParams {
    pub information: Option<String>,
}

#[allow(dead_code)]
pub async fn call_gemini_api(gemini_request: &GeminiRequest) -> Result<String, reqwest::Error> {
    let api_key = GeminiConfig::from_env().api_key;

    let url = GeminiConfig::from_env().base_url;

    let client = Client::new();

    let response = client
        .post(url)
        .header("x-goog-api-key", &api_key)
        .header("Content-Type", "application/json")
        .json(gemini_request)
        .send()
        .await?;

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        eprintln!("Gemini API error: {}", err_text);
        return Ok("API Error".to_string());
    }

    let gemini_response: GeminiResponse = response.json().await?;

    let response_text = gemini_response
        .candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.clone())
        .unwrap_or_else(|| "No response".to_string());

    // Remove markdown code fences if present
    let cleaned = if response_text.starts_with("```json") {
        response_text
            .strip_prefix("```json\n")
            .unwrap_or(&response_text)
            .strip_suffix("\n```")
            .unwrap_or(&response_text)
            .to_string()
    } else {
        response_text
    };

    Ok(cleaned)
}
pub async fn chat_handler_with_retry(
    Json(payload): Json<ChatRequest>,
) -> Result<ResponseJson<Value>, StatusCode> {
    // RETRY LOGIC DISABLED DUE TO 429 RATE LIMIT ERRORS
    // const MAX_RETRIES: u32 = 2;
    // let mut attempt = 0;

    // loop {
    //     attempt += 1;

    //     let result = try_generate_recipes(&payload).await;

    //     match result {
    //         Ok(json) => {
    //             // Check if we got complete recipes
    //             if let Some(dishes) = json.get("dishes").and_then(|d| d.as_array()) {
    //                 if dishes.len() == 3 {
    //                     return Ok(ResponseJson(json));
    //                 }
    //             }

    //             // If incomplete and we have retries left, try again
    //             if attempt < MAX_RETRIES {
    //                 println!("Incomplete response, retrying... (attempt {})", attempt + 1);
    //                 continue;
    //             }

    //             // Return what we have
    //             return Ok(ResponseJson(json));
    //         }
    //         Err(e) if attempt < MAX_RETRIES => {
    //             println!("Request failed, retrying... (attempt {})", attempt + 1);
    //             continue;
    //         }
    //         Err(e) => return Err(e),
    //     }
    // }

    // Single attempt without retry
    let result = try_generate_recipes(&payload).await;
    match result {
        Ok(json) => Ok(ResponseJson(json)),
        Err(e) => Err(e),
    }
}

// 2. Separate the generation logic
async fn try_generate_recipes(payload: &ChatRequest) -> Result<Value, StatusCode> {
    // ... your existing generation logic here ...
    let api_key = GeminiConfig::from_env().api_key;

    // Extract prompt text
    let prompt_text = match &payload.prompt {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Object(obj) => obj
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    };

    let people_context = if let (Some(userid), Some(selected_people_ids)) =
        (payload.userid.as_ref(), payload.selected_people_ids.as_ref())
    {
        build_people_prompt_context(userid, selected_people_ids)
            .await
            .unwrap_or_default()
    } else {
        String::new()
    };

    let additional_people_context = if people_context.is_empty() {
        String::new()
    } else {
        format!("\n{}\n", people_context)
    };

    let system_prompt = format!(
        r#"
You are NekoChef: a creative, reliable, friendly cooking assistant.

Goal: suggest dishes people can cook with the provided ingredients.

Ingredients: {ingredients}

Return exactly three distinct dishes. Each dish must:
- Have a clear "dishName".
- Include a "simplified" field: the most basic/common name for the dish (for image search).
- Include a "portions" object with ingredient amounts and units.
- Include a "preparationSteps" array of concise steps.
- Use common cooking knowledge; be imaginative but realistic; 2-3 servings.

Respond strictly as JSON in this shape:
{{
  "dishes": [
    {{
      "dishName": "Dish Title",
      "simplified": "basic dish type",
      "portions": {{
        "ingredient1": "amount with unit",
        "ingredient2": "amount with unit",
        "oil": "2 tablespoons",
        "salt": "1 teaspoon"
      }},
      "preparationSteps": [
        "Step 1...",
        "Step 2...",
        "...etc"
      ]
    }},
    ... 2 more dishes
  ]
}}

Output language: {language}.
Do not add extra text outside the JSON object.
{additional_people_context}
"#,
        ingredients = prompt_text,
    language = payload.prompt_language,
    additional_people_context = additional_people_context
    );

    let content_text = format!(
        "Ingredients: {}\nLanguage: {}",
        prompt_text, payload.prompt_language
    );

    // Use higher token limit based on ingredients count
    let ingredient_count = payload
        .prompt
        .as_str()
        .map(|s| s.split(',').count())
        .unwrap_or(5);

    let token_limit = std::cmp::max(5000, ingredient_count * 300);

    let gemini_request = GeminiRequest {
        system_instruction: Some(SystemInstruction {
            parts: vec![GeminiPart {
                text: Some(system_prompt),
                inline_data: None,
            }],
        }),
        contents: vec![GeminiContent {
            parts: vec![GeminiPart { 
                text: Some(content_text),
                inline_data: None,
            }],
        }],
        generation_config: GenerationConfig {
            temperature: 0.7,
            max_output_tokens: token_limit as u32,
            top_p: 0.8,
            top_k: 40,
        },
    };

    let client = Client::new();
    let url = GeminiConfig::from_env().base_url;

    let response = client
        .post(url)
        .header("x-goog-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&gemini_request)
        .send()
        .await
        .map_err(|_| {
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !response.status().is_success() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let gemini_response: GeminiResponse = response.json().await.map_err(|_| {
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response_text = gemini_response
        .candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.clone())
        .unwrap_or_else(|| "No response".to_string());

    // Clean up the response - remove markdown code blocks
    let cleaned_response = {
        let mut text = response_text.trim().to_string();

        // Remove code block markers
        if text.starts_with("```json") {
            text = text.strip_prefix("```json").unwrap_or(&text).to_string();
        } else if text.starts_with("```") {
            text = text.strip_prefix("```").unwrap_or(&text).to_string();
        }

        if text.ends_with("```") {
            text = text.strip_suffix("```").unwrap_or(&text).to_string();
        }

        // Trim any remaining whitespace
        text.trim().to_string()
    };

    // Parse the JSON response
    let mut parsed_json: Value = serde_json::from_str(&cleaned_response).map_err(|_| {
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Concise pretty print to terminal
    if let Some(dishes) = parsed_json.get("dishes").and_then(|d| d.as_array()) {
        for _dish in dishes.iter() {
        }
    }

    // Attempt to fetch images for each dish and add to response
    if let Some(dishes) = parsed_json.get_mut("dishes").and_then(|d| d.as_array_mut()) {
        for dish in dishes.iter_mut() {
            // 1️⃣ Extract owned String first (end immutable borrow)
            let simplified = dish
                .get("simplified")
                .and_then(|s| s.as_str())
                .map(|s| s.to_string());

            // 2️⃣ Now safe to mutate dish
            if let Some(simplified) = simplified {
                match get_one_image(&simplified).await {
                    Ok(Some(image_url)) => {
                        dish["image_url"] = Value::String(image_url);
                    }
                    Ok(None) => {}
                    Err(_) => {}
                }
            }
        }
    }

    // Return the parsed JSON directly
    Ok(parsed_json)

    // ... rest of logic ...
}

// 3. Better response validation
#[allow(dead_code)]
fn validate_recipe_response(json: &Value) -> bool {
    if let Some(dishes) = json.get("dishes").and_then(|d| d.as_array()) {
        if dishes.len() != 3 {
            return false;
        }

        for dish in dishes {
            // Check required fields
            if dish.get("dishName").and_then(|n| n.as_str()).is_none() {
                return false;
            }
            if dish.get("portions").and_then(|p| p.as_object()).is_none() {
                return false;
            }
            if dish
                .get("preparationSteps")
                .and_then(|s| s.as_array())
                .is_none()
            {
                return false;
            }
        }

        return true;
    }

    false
}

/// Handles image classification via Gemini Vision API
/// Accepts multipart form with "image" field (PNG or JPG)
/// Optional query param: ?information=CuisineType for additional context
/// Returns dish identification with ingredients in "ingredient": "amount unit" format
pub async fn classify_image_handler(
    mut multipart: Multipart,
) -> Result<ResponseJson<ClassifyResponse>, StatusCode> {
    // Extract and validate image from multipart form
    let mut image_bytes = Vec::new();
    let mut mime_type = String::new();

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let field_name = field.name().unwrap_or("").to_string();
        
        if field_name == "image" {
            if let Some(content_type_str) = field.content_type() {
                mime_type = content_type_str.to_string();
            }

            if mime_type.is_empty() {
                mime_type = "image/jpeg".to_string();
            }
            
            // Validate MIME type (PNG or JPG)
            if !mime_type.contains("image/png") && !mime_type.contains("image/jpeg") {
                eprintln!("Invalid image MIME type: {}", mime_type);
                return Err(StatusCode::BAD_REQUEST);
            }

            image_bytes = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?.to_vec();
            break;
        }
    }

    if image_bytes.is_empty() {
        eprintln!("No image provided in multipart form");
        return Err(StatusCode::BAD_REQUEST);
    }

    // Preprocess image before base64 to reduce payload and token pressure.
    let (prepared_image_bytes, prepared_mime_type) = preprocess_image_for_gemini(&image_bytes, &mime_type);
    if prepared_image_bytes.len() < image_bytes.len() {
        eprintln!(
            "classify: compressed image {} -> {} bytes",
            image_bytes.len(),
            prepared_image_bytes.len()
        );
    }

    // Convert preprocessed image to base64
    let base64_image = base64_encode(&prepared_image_bytes);

    // Build context from query parameter
    let context_line = String::new();

    // Build Gemini Vision request
    let system_prompt = format!(r#"You are a food identification expert. Your task is to respond with JSON ONLY, no matter what.

{}If there is a dish in the image, identify it, list ingredients with amounts, and estimate nutrition info.
If there is NO food/dish in the image, return:
{{
  "dishName": "No Food Detected",
  "ingredients": {{}}
}}

ALWAYS respond with valid JSON (no markdown, no extra text, no explanations):
{{
  "dishName": "Name of the dish or 'No Food Detected'",
    "genericName": "short searchable base name such as 'fried rice', 'noodle soup', 'grilled chicken'",
  "ingredients": {{
    "ingredient_name": "amount unit",
    "rice": "2 cups",
    "salt": "1 teaspoon"
  }},
  "calories": "250 kcal",
  "nutrients": {{
    "protein": "15g",
    "carbs": "30g",
    "fat": "8g",
    "fiber": "2g"
  }}
}}

Be concise. Include calories per serving and nutrient breakdown. Respond ONLY with JSON."#, context_line);

    let content_text = "Please identify this dish and provide its ingredients.";

    let gemini_request = GeminiRequest {
        system_instruction: Some(SystemInstruction {
            parts: vec![GeminiPart {
                text: Some(system_prompt.to_string()),
                inline_data: None,
            }],
        }),
        contents: vec![GeminiContent {
            parts: vec![
                GeminiPart {
                    text: Some(content_text.to_string()),
                    inline_data: None,
                },
                GeminiPart {
                    text: None,
                    inline_data: Some(InlineImageData {
                        mime_type: prepared_mime_type,
                        data: base64_image,
                    }),
                },
            ],
        }],
        generation_config: GenerationConfig {
            temperature: 0.7,
            max_output_tokens: 2500,
            top_p: 0.8,
            top_k: 40,
        },
    };

    // Call Gemini API
    let api_key = GeminiConfig::from_env().api_key;
    let url = GeminiConfig::from_env().base_url;
    let client = Client::new();

    let response = client
        .post(url)
        .header("x-goog-api-key", &api_key)
        .header("Content-Type", "application/json")
        .json(&gemini_request)
        .send()
        .await
        .map_err(|e| {
            eprintln!("Gemini API request failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if !response.status().is_success() {
        let err_text = response.text().await.unwrap_or_default();
        eprintln!("Gemini API error response: {}", err_text);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let gemini_response: GeminiResponse = response.json().await.map_err(|e| {
        eprintln!("Failed to parse Gemini response: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response_text = gemini_response
        .candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.clone())
        .unwrap_or_else(|| "No response".to_string());

    // Clean up response - remove markdown code blocks
    let cleaned_response = {
        let mut text = response_text.trim().to_string();

        if text.starts_with("```json") {
            text = text.strip_prefix("```json").unwrap_or(&text).to_string();
        } else if text.starts_with("```") {
            text = text.strip_prefix("```").unwrap_or(&text).to_string();
        }

        if text.ends_with("```") {
            text = text.strip_suffix("```").unwrap_or(&text).to_string();
        }

        text.trim().to_string()
    };

    // Parse JSON response into ClassifyResponse
    let parsed_json: Value = serde_json::from_str(&cleaned_response).map_err(|e| {
        eprintln!("Failed to parse classification JSON: {}", e);
        eprintln!("Response was: {}", cleaned_response);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    fn value_to_string(v: &serde_json::Value) -> String {
        match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Null => String::new(),
            _ => v.to_string(),
        }
    }

    let classify_response = ClassifyResponse {
        dish_name: parsed_json
            .get("dishName")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Dish")
            .to_string(),
        ingredients: parsed_json
            .get("ingredients")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), value_to_string(v)))
                    .collect()
            })
            .unwrap_or_default(),
        calories: parsed_json
            .get("calories")
            .map(value_to_string)
            .and_then(|s| if s.is_empty() { None } else { Some(s) }),
        nutrients: parsed_json
            .get("nutrients")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), value_to_string(v)))
                    .collect()
            }),
    };

    let no_food = classify_response.dish_name.eq_ignore_ascii_case("No Food Detected");
    if !no_food && !classify_response.ingredients.is_empty() {
        let firebase_config = FirebaseConfig::from_env();
        let generic_name = parsed_json
            .get("genericName")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| classify_response.dish_name.clone());

        let mut nutrients = classify_response.nutrients.clone().unwrap_or_default();
        if let Some(calories) = &classify_response.calories {
            if !calories.trim().is_empty() {
                nutrients.insert("calories".to_string(), calories.clone());
            }
        }

        let aliases = if generic_name != classify_response.dish_name {
            vec![
                classify_response.dish_name.to_lowercase().replace(" ", ""),
                generic_name.to_lowercase().replace(" ", ""),
            ]
        } else {
            vec![generic_name.to_lowercase().replace(" ", "")]
        };

        let recipe_id = match insert_recipe(
            &firebase_config,
            generic_name.clone(),
            classify_response.ingredients.clone(),
            classify_response.calories.clone(),
            Some(nutrients),
            aliases.clone(),
        )
        .await
        {
            Ok(id) => id,
            Err(e) => {
                eprintln!("⚠️ classify save: failed to insert recipe '{}': {}", generic_name, e);
                return Ok(ResponseJson(classify_response));
            }
        };

        let normalized = generic_name.to_lowercase().replace(" ", "");
        if let Err(e) = insert_dishname(
            &firebase_config,
            recipe_id.clone(),
            generic_name.clone(),
            normalized,
            aliases,
        )
        .await
        {
            eprintln!("⚠️ classify save: failed to insert dishname for '{}': {}", generic_name, e);
        } else {
            eprintln!("✅ classify save: recipe persisted '{}', recipe_id='{}'", generic_name, recipe_id);
        }
    }

    Ok(ResponseJson(classify_response))
}

/// Helper: Encode bytes to base64
fn base64_encode(data: &[u8]) -> String {
    const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    
    for chunk in data.chunks(3) {
        let b1 = chunk[0];
        let b2 = chunk.get(1).copied().unwrap_or(0);
        let b3 = chunk.get(2).copied().unwrap_or(0);
        
        let n = ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32);
        
        result.push(BASE64_CHARS[((n >> 18) & 63) as usize] as char);
        result.push(BASE64_CHARS[((n >> 12) & 63) as usize] as char);
        
        if chunk.len() > 1 {
            result.push(BASE64_CHARS[((n >> 6) & 63) as usize] as char);
        } else {
            result.push('=');
        }
        
        if chunk.len() > 2 {
            result.push(BASE64_CHARS[(n & 63) as usize] as char);
        } else {
            result.push('=');
        }
    }
    
    result
}

/// Resize and compress image for Gemini vision calls.
fn preprocess_image_for_gemini(image_bytes: &[u8], original_mime_type: &str) -> (Vec<u8>, String) {
    const MAX_DIMENSION: u32 = 1024;
    const JPEG_QUALITY: u8 = 78;

    let normalized_mime = if original_mime_type.contains("image/png") {
        "image/png"
    } else if original_mime_type.contains("image/jpeg") {
        "image/jpeg"
    } else {
        "image/jpeg"
    };

    let decoded = match image::load_from_memory(image_bytes) {
        Ok(img) => img,
        Err(err) => {
            eprintln!("classify: image decode failed, using original bytes: {}", err);
            return (image_bytes.to_vec(), normalized_mime.to_string());
        }
    };

    let resized = if decoded.width() > MAX_DIMENSION || decoded.height() > MAX_DIMENSION {
        decoded.resize(MAX_DIMENSION, MAX_DIMENSION, FilterType::Triangle)
    } else {
        decoded
    };

    let mut compressed = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut compressed, JPEG_QUALITY);
    if let Err(err) = encoder.encode_image(&resized) {
        eprintln!("classify: image encode failed, using original bytes: {}", err);
        return (image_bytes.to_vec(), normalized_mime.to_string());
    }

    (compressed, "image/jpeg".to_string())
}
