use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn simulated_classify_delay_ms(file_name: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    file_name.hash(&mut hasher);
    3_000 + (hasher.finish() % 5_000)
}

pub fn local_classify_payload(file_name: &str) -> Option<Value> {
    let normalized = file_name.trim().to_ascii_lowercase();

    let payload = match normalized.as_str() {
        "burger__image.jpg" => serde_json::json!({
            "dishName": "Classic Cheeseburger",
            "genericName": "cheeseburger",
            "ingredients": {
                "American cheese slice": "1",
                "lettuce": "1-2 leaves",
                "ground beef patty": "1/4 lb",
                "red onion": "2-3 slices",
                "tomato": "1 slice",
                "dill pickles": "3-4 slices",
                "sesame seed bun": "1"
            },
            "calories": "580 kcal",
            "nutrients": {
                "protein": "32g",
                "carbs": "40g",
                "fat": "35g",
                "fiber": "3g"
            }
        }),
        "kawmankai__image.png" => serde_json::json!({
            "dishName": "Hainanese Chicken Rice (Khao Man Gai)",
            "genericName": "hainanese chicken rice",
            "ingredients": {
                "rice (cooked in chicken broth)": "1.5 cups",
                "chicken blood (congealed)": "50g",
                "cucumber": "0.5 cup sliced",
                "chicken broth": "1 cup",
                "cilantro": "1 sprig",
                "ginger-garlic chili sauce": "2 tablespoons",
                "poached chicken breast": "150g",
                "soy sauce based dipping sauce": "1 tablespoon"
            },
            "calories": "550 kcal",
            "nutrients": {
                "protein": "45g",
                "carbs": "60g",
                "fat": "15g",
                "fiber": "3g"
            }
        }),
        "kaitomkamin__image.jpg" => serde_json::json!({
            "dishName": "Chicken Turmeric Soup",
            "genericName": "chicken turmeric soup",
            "ingredients": {
                "carrot": "1/2 cup, sliced",
                "chicken broth": "2 cups",
                "chicken breast": "150g",
                "garlic": "2 cloves, minced",
                "green beans": "1/4 cup, chopped",
                "black pepper": "1/4 teaspoon",
                "cilantro": "2 tablespoons, chopped",
                "salt": "1/2 teaspoon",
                "turmeric powder": "1 teaspoon",
                "ginger": "1 inch, grated",
                "red chili": "1, sliced"
            },
            "calories": "280 kcal",
            "nutrients": {
                "protein": "35g",
                "carbs": "15g",
                "fat": "8g",
                "fiber": "3g"
            }
        }),
        "ramen__image.jpg" => serde_json::json!({
            "dishName": "Ramen",
            "genericName": "ramen noodle soup",
            "ingredients": {
                "soft-boiled egg": "1 large",
                "soy sauce": "1 tablespoon",
                "sake": "1 teaspoon",
                "chashu pork": "1 slice (approx. 70g)",
                "pork broth": "2 cups",
                "green onions": "2 tablespoons, chopped",
                "shiitake mushrooms": "3-4 slices",
                "ramen noodles": "150g",
                "mirin": "1 teaspoon"
            },
            "calories": "550 kcal",
            "nutrients": {
                "protein": "35g",
                "carbs": "60g",
                "fat": "20g",
                "fiber": "3g"
            }
        }),
        _ => return None,
    };

    Some(payload)
}