/// API Contract Tests
/// 
/// Validates that backend responses match the documented shape from:
/// docs/schedule-location-frontend.md
/// 
/// These tests ensure the frontend migration guide remains accurate and
/// that API response structures don't drift from documentation.

use serde_json::{json, Value};

/// Validates Schedule response shape matches the contract
pub fn validate_schedule_contract(schedule: &Value) -> Result<(), String> {
    let required_fields = ["uid", "name", "time", "date", "expiry"];
    
    for field in &required_fields {
        if !schedule.get(field).is_some() {
            return Err(format!("Schedule missing required field: {}", field));
        }
    }
    
    // Validate field types
    if !schedule.get("uid").unwrap().is_string() {
        return Err("Schedule.uid must be string".to_string());
    }
    if !schedule.get("name").unwrap().is_string() {
        return Err("Schedule.name must be string".to_string());
    }
    if !schedule.get("time").unwrap().is_string() {
        return Err("Schedule.time must be string (HH-MM format)".to_string());
    }
    if !schedule.get("date").unwrap().is_string() {
        return Err("Schedule.date must be string (YYYY-MM-DD format)".to_string());
    }
    if !schedule.get("expiry").unwrap().is_string() {
        return Err("Schedule.expiry must be string (YYYY-MM-DD format)".to_string());
    }
    
    // Optional fields
    if let Some(notify) = schedule.get("notify") {
        if !notify.is_boolean() && !notify.is_null() {
            return Err("Schedule.notify must be boolean or null".to_string());
        }
    }
    if let Some(category) = schedule.get("category") {
        if !category.is_string() && !category.is_null() {
            return Err("Schedule.category must be string or null".to_string());
        }
    }
    if let Some(quantity) = schedule.get("quantity") {
        if !quantity.is_string() && !quantity.is_null() {
            return Err("Schedule.quantity must be string or null".to_string());
        }
    }
    if let Some(location) = schedule.get("location") {
        if !location.is_string() && !location.is_null() {
            return Err("Schedule.location must be string or null".to_string());
        }
    }
    
    Ok(())
}

/// Validates SavedRecipe response shape
pub fn validate_recipe_contract(recipe: &Value) -> Result<(), String> {
    let required_fields = ["uid", "name", "ingredients", "steps"];
    
    for field in &required_fields {
        if !recipe.get(field).is_some() {
            return Err(format!("Recipe missing required field: {}", field));
        }
    }
    
    if !recipe.get("uid").unwrap().is_string() {
        return Err("Recipe.uid must be string".to_string());
    }
    if !recipe.get("name").unwrap().is_string() {
        return Err("Recipe.name must be string".to_string());
    }
    if !recipe.get("ingredients").unwrap().is_array() {
        return Err("Recipe.ingredients must be array".to_string());
    }
    if !recipe.get("steps").unwrap().is_array() {
        return Err("Recipe.steps must be array".to_string());
    }
    
    // Validate ingredient items are strings
    if let Some(ingredients) = recipe.get("ingredients").unwrap().as_array() {
        for (idx, ingredient) in ingredients.iter().enumerate() {
            if !ingredient.is_string() {
                return Err(format!("Recipe.ingredients[{}] must be string", idx));
            }
        }
    }
    
    // Validate step items are strings
    if let Some(steps) = recipe.get("steps").unwrap().as_array() {
        for (idx, step) in steps.iter().enumerate() {
            if !step.is_string() {
                return Err(format!("Recipe.steps[{}] must be string", idx));
            }
        }
    }
    
    // Optional fields
    if let Some(time) = recipe.get("time") {
        if !time.is_string() && !time.is_null() {
            return Err("Recipe.time must be string or null".to_string());
        }
    }
    if let Some(image_url) = recipe.get("image_url") {
        if !image_url.is_string() && !image_url.is_null() {
            return Err("Recipe.image_url must be string or null".to_string());
        }
    }
    if let Some(quantity) = recipe.get("quantity") {
        if !quantity.is_string() && !quantity.is_null() {
            return Err("Recipe.quantity must be string or null".to_string());
        }
    }
    
    Ok(())
}

/// Validates GroceryList response shape
pub fn validate_grocery_list_contract(list: &Value) -> Result<(), String> {
    let required_fields = ["list_id", "list_name", "items"];
    
    for field in &required_fields {
        if !list.get(field).is_some() {
            return Err(format!("GroceryList missing required field: {}", field));
        }
    }
    
    if !list.get("list_id").unwrap().is_string() {
        return Err("GroceryList.list_id must be string".to_string());
    }
    if !list.get("list_name").unwrap().is_string() {
        return Err("GroceryList.list_name must be string".to_string());
    }
    if !list.get("items").unwrap().is_array() {
        return Err("GroceryList.items must be array".to_string());
    }
    
    if let Some(items) = list.get("items").unwrap().as_array() {
        for (idx, item) in items.iter().enumerate() {
            validate_grocery_item_contract(item)
                .map_err(|e| format!("GroceryList.items[{}]: {}", idx, e))?;
        }
    }
    
    Ok(())
}

/// Validates GroceryItem response shape
pub fn validate_grocery_item_contract(item: &Value) -> Result<(), String> {
    let required_fields = ["uid", "name", "quantity", "unit", "checked"];
    
    for field in &required_fields {
        if !item.get(field).is_some() {
            return Err(format!("GroceryItem missing required field: {}", field));
        }
    }
    
    if !item.get("uid").unwrap().is_string() {
        return Err("GroceryItem.uid must be string".to_string());
    }
    if !item.get("name").unwrap().is_string() {
        return Err("GroceryItem.name must be string".to_string());
    }
    if !item.get("quantity").unwrap().is_string() {
        return Err("GroceryItem.quantity must be string".to_string());
    }
    if !item.get("unit").unwrap().is_string() {
        return Err("GroceryItem.unit must be string".to_string());
    }
    if !item.get("checked").unwrap().is_boolean() {
        return Err("GroceryItem.checked must be boolean".to_string());
    }
    
    Ok(())
}

/// Validates PersonProfile response shape
pub fn validate_person_contract(person: &Value) -> Result<(), String> {
    let required_fields = ["person_id", "name", "preference", "allergy"];
    
    for field in &required_fields {
        if !person.get(field).is_some() {
            return Err(format!("PersonProfile missing required field: {}", field));
        }
    }
    
    if !person.get("person_id").unwrap().is_string() {
        return Err("PersonProfile.person_id must be string".to_string());
    }
    if !person.get("name").unwrap().is_string() {
        return Err("PersonProfile.name must be string".to_string());
    }
    if !person.get("preference").unwrap().is_array() {
        return Err("PersonProfile.preference must be array".to_string());
    }
    if !person.get("allergy").unwrap().is_array() {
        return Err("PersonProfile.allergy must be array".to_string());
    }
    
    // Validate array items are strings
    for (idx, pref) in person.get("preference").unwrap().as_array().unwrap().iter().enumerate() {
        if !pref.is_string() {
            return Err(format!("PersonProfile.preference[{}] must be string", idx));
        }
    }
    
    for (idx, allergy) in person.get("allergy").unwrap().as_array().unwrap().iter().enumerate() {
        if !allergy.is_string() {
            return Err(format!("PersonProfile.allergy[{}] must be string", idx));
        }
    }
    
    Ok(())
}

/// Validates User aggregate response shape (GET /user/{uid})
pub fn validate_user_aggregate_contract(user: &Value) -> Result<(), String> {
    // Check required top-level fields
    if !user.get("email").is_some() {
        return Err("User response missing required field: email".to_string());
    }
    
    // Check collections exist as arrays
    if let Some(savedrecipes) = user.get("savedrecipes") {
        if !savedrecipes.is_array() {
            return Err("User.savedrecipes must be array".to_string());
        }
        for (idx, recipe) in savedrecipes.as_array().unwrap().iter().enumerate() {
            validate_recipe_contract(recipe)
                .map_err(|e| format!("User.savedrecipes[{}]: {}", idx, e))?;
        }
    }
    
    if let Some(grocery) = user.get("grocery") {
        if !grocery.is_array() {
            return Err("User.grocery must be array".to_string());
        }
        for (idx, list) in grocery.as_array().unwrap().iter().enumerate() {
            validate_grocery_list_contract(list)
                .map_err(|e| format!("User.grocery[{}]: {}", idx, e))?;
        }
    }
    
    if let Some(schedules) = user.get("schedules") {
        if !schedules.is_array() {
            return Err("User.schedules must be array".to_string());
        }
        for (idx, schedule) in schedules.as_array().unwrap().iter().enumerate() {
            validate_schedule_contract(schedule)
                .map_err(|e| format!("User.schedules[{}]: {}", idx, e))?;
        }
    }
    
    if let Some(people) = user.get("people") {
        if !people.is_array() {
            return Err("User.people must be array".to_string());
        }
        for (idx, person) in people.as_array().unwrap().iter().enumerate() {
            validate_person_contract(person)
                .map_err(|e| format!("User.people[{}]: {}", idx, e))?;
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_schedule_contract_valid() {
        let schedule = json!({
            "uid": "schedule_123",
            "name": "Buy milk",
            "time": "09-30",
            "date": "2026-04-21",
            "expiry": "2026-04-25",
            "notify": true,
            "category": "grocery",
            "quantity": "2 bottles",
            "location": "Kitchen"
        });
        
        assert!(validate_schedule_contract(&schedule).is_ok());
    }

    #[test]
    fn test_validate_schedule_contract_missing_field() {
        let schedule = json!({
            "uid": "schedule_123",
            "name": "Buy milk",
            // Missing time field
            "date": "2026-04-21",
            "expiry": "2026-04-25",
        });
        
        assert!(validate_schedule_contract(&schedule).is_err());
    }

    #[test]
    fn test_validate_recipe_contract_valid() {
        let recipe = json!({
            "uid": "recipe_123",
            "name": "Tom Yum Soup",
            "ingredients": ["water", "lemongrass", "shrimp"],
            "steps": ["boil water", "add ingredients"],
            "time": "30 min",
            "image_url": "https://example.com/image.jpg",
            "quantity": "2 bowls"
        });
        
        assert!(validate_recipe_contract(&recipe).is_ok());
    }

    #[test]
    fn test_validate_grocery_item_contract_valid() {
        let item = json!({
            "uid": "item_123",
            "name": "Eggs",
            "quantity": "12",
            "unit": "pcs",
            "checked": false
        });
        
        assert!(validate_grocery_item_contract(&item).is_ok());
    }

    #[test]
    fn test_validate_person_contract_valid() {
        let person = json!({
            "person_id": "person_123",
            "name": "Aunt_tie",
            "preference": ["likes milk"],
            "allergy": ["peanut", "water"]
        });
        
        assert!(validate_person_contract(&person).is_ok());
    }

    #[test]
    fn test_validate_user_aggregate_contract_valid() {
        let user = json!({
            "email": "user@example.com",
            "username": "demo",
            "savedrecipes": [
                {
                    "uid": "recipe_x",
                    "name": "Tom Yum",
                    "ingredients": ["lemongrass"],
                    "steps": ["boil"],
                    "time": "30 min"
                }
            ],
            "grocery": [
                {
                    "list_id": "grocery_x",
                    "list_name": "Weekly Shopping",
                    "items": [
                        {
                            "uid": "item_x",
                            "name": "Eggs",
                            "quantity": "12",
                            "unit": "pcs",
                            "checked": false
                        }
                    ]
                }
            ],
            "schedules": [
                {
                    "uid": "schedule_x",
                    "name": "Buy milk",
                    "time": "09-30",
                    "date": "2026-04-21",
                    "expiry": "2026-04-25"
                }
            ],
            "people": [
                {
                    "person_id": "person_x",
                    "name": "Aunt_tie",
                    "preference": ["likes milk"],
                    "allergy": ["peanut"]
                }
            ]
        });
        
        assert!(validate_user_aggregate_contract(&user).is_ok());
    }
}
