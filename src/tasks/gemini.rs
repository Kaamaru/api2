use crate::firebase::GeminiConfig;
use crate::structures::models::{
    ChatRequest, GeminiContent, GeminiPart, GeminiRequest, GeminiResponse,
    GenerationConfig, SystemInstruction,
};
use crate::tasks::getimage::get_one_image;
use axum::{extract::Json, http::StatusCode, response::Json as ResponseJson};
use reqwest::Client;
use serde_json::Value;

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
"#,
        ingredients = prompt_text,
        language = payload.prompt_language
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
                text: system_prompt,
            }],
        }),
        contents: vec![GeminiContent {
            parts: vec![GeminiPart { text: content_text }],
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
