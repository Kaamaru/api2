use crate::firebase::GEMINI_API_KEY;
use crate::structures::models::{
    ChatRequest, ChatResponse, GeminiContent, GeminiPart, GeminiRequest, GeminiResponse,
    GenerationConfig, SystemInstruction,
};
use axum::{extract::Json, http::StatusCode, response::Json as ResponseJson};
use reqwest::Client;
use serde_json::Value;

pub async fn call_gemini_api(gemini_request: &GeminiRequest) -> Result<String, reqwest::Error> {
    let api_key = std::env::var("GEMINI_API_KEY")
        .unwrap_or_else(|_| panic!("GEMINI_API_KEY not set in environment"));

    let url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent";

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
    let api_key = GEMINI_API_KEY;

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

    let system_instruction_text = format!(
        "You are a highly creative, reliable, and friendly cooking assistant named NekoChef.\n\nYour task is to help users discover delicious meals they can cook using the ingredients they provide.\n\nGiven the following ingredients: {}\n\nSuggest exactly three distinct dishes. Each dish must:\n- Have a clear and appealing \"dishName\".\n- Include a \"portions\" object with ingredient amounts using appropriate units (grams, milliliters, pieces, tablespoons, teaspoons, cups, etc.).\n- Include a \"preparationSteps\" array, where each item is a single clear step of the recipe.\n- Use common cooking knowledge to make the best use of the given ingredients, even if not all are used in each dish.\n- Be imaginative, yet realistic to prepare in a regular kitchen.\n- Provide realistic portion sizes for 2-3 servings.\n\nRespond strictly as a JSON object with the following structure:\n\n{{\n  \"dishes\": [\n    {{\n      \"dishName\": \"Dish Title\",\n      \"portions\": {{\n        \"ingredient1\": \"amount with unit\",\n        \"ingredient2\": \"amount with unit\",\n        \"oil\": \"2 tablespoons\",\n        \"salt\": \"1 teaspoon\"\n      }},\n      \"preparationSteps\": [\n        \"Step 1...\",\n        \"Step 2...\",\n        \"...etc\"\n      ]\n    }},\n    ... 2 more dishes\n  ]\n}}\n\nPlease provide the entire response in the {} language.\nDo **not** include any extra text, commentary, or formatting outside of the JSON object.",
        prompt_text, payload.prompt_language
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

    let token_limit = std::cmp::max(3000, ingredient_count * 300);

    let gemini_request = GeminiRequest {
        system_instruction: Some(SystemInstruction {
            parts: vec![GeminiPart {
                text: system_instruction_text,
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
    let url = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent";

    println!("Sending request to: {}", url);
    println!("With API key: {}", api_key);
    println!(
        "Request body: {}",
        serde_json::to_string_pretty(&gemini_request).unwrap()
    );

    let response = client
        .post(url)
        .header("x-goog-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&gemini_request)
        .send()
        .await
        .map_err(|e| {
            println!("Request error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    println!("Response status: {}", response.status());

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        println!("API error response: {}", error_text);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    let gemini_response: GeminiResponse = response.json().await.map_err(|e| {
        println!("JSON parse error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response_text = gemini_response
        .candidates
        .first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.clone())
        .unwrap_or_else(|| "No response".to_string());

    println!("Raw Gemini response: {}", response_text);
    println!(
        "Response starts with ```json: {}",
        response_text.starts_with("```json")
    );
    println!(
        "Response starts with ```: {}",
        response_text.starts_with("```")
    );

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

    println!("Cleaned response: {}", cleaned_response);
    println!("Attempting to parse JSON...");

    // Parse the JSON response
    let parsed_json: Value = serde_json::from_str(&cleaned_response).map_err(|e| {
        println!("Failed to parse Gemini response as JSON: {}", e);
        println!("Raw response: {}", response_text);
        println!("Cleaned response: {}", cleaned_response);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Pretty print to terminal (optional - you can remove this if not needed)
    if let Some(dishes) = parsed_json.get("dishes").and_then(|d| d.as_array()) {
        println!("\n🍳 NekoChef's Recipe Suggestions:");
        println!("================================");

        for (i, dish) in dishes.iter().enumerate() {
            if let Some(name) = dish.get("dishName").and_then(|n| n.as_str()) {
                println!("\n{}. 🍽️  {}", i + 1, name);

                // Print portions if available
                if let Some(portions) = dish.get("portions").and_then(|p| p.as_object()) {
                    println!("   📏 Ingredients:");
                    for (ingredient, amount) in portions {
                        if let Some(amount_str) = amount.as_str() {
                            println!("   - {}: {}", ingredient, amount_str);
                        }
                    }
                }

                // Print steps
                if let Some(steps) = dish.get("preparationSteps").and_then(|s| s.as_array()) {
                    println!("   👨‍🍳 Steps:");
                    for (step_num, step) in steps.iter().enumerate() {
                        if let Some(step_text) = step.as_str() {
                            println!("   {}. {}", step_num + 1, step_text);
                        }
                    }
                }
            }
        }
        println!("\n================================");
    }

    // Return the parsed JSON directly
    Ok(parsed_json)

    // ... rest of logic ...
}

// 3. Better response validation
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
