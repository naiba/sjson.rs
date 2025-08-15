use sjson::{set, set_bool, set_int, set_float, set_raw, delete, set_value};

fn main() {
    println!("=== sjson Rust Examples ===\n");

    // Basic setting
    println!("1. Basic setting:");
    let json = r#"{"name":"Tom","age":37}"#;
    let result = set(json, "name", "Jerry").unwrap();
    println!("Original: {}", json);
    println!("Set name=Jerry: {}", result);
    println!();

    // Nested setting
    println!("2. Nested setting:");
    let json = r#"{"name":{"first":"Tom","last":"Anderson"}}"#;
    let result = set(json, "name.first", "Jerry").unwrap();
    println!("Original: {}", json);
    println!("Set name.first=Jerry: {}", result);
    println!();

    // Array setting
    println!("3. Array setting:");
    let json = r#"{"children":["Sara","Alex","Jack"]}"#;
    let result = set(json, "children.1", "Jerry").unwrap();
    println!("Original: {}", json);
    println!("Set children.1=Jerry: {}", result);
    println!();

    // Set new field
    println!("4. Set new field:");
    let json = r#"{"name":"Tom"}"#;
    let result = set(json, "age", "37").unwrap();
    println!("Original: {}", json);
    println!("Set age=37: {}", result);
    println!();

    // Set boolean value
    println!("5. Set boolean value:");
    let json = r#"{"name":"Tom"}"#;
    let result = set_bool(json, "active", true, None).unwrap();
    println!("Original: {}", json);
    println!("Set active=true: {}", result);
    println!();

    // Set integer value
    println!("6. Set integer value:");
    let json = r#"{"name":"Tom"}"#;
    let result = set_int(json, "age", 37, None).unwrap();
    println!("Original: {}", json);
    println!("Set age=37: {}", result);
    println!();

    // Set float value
    println!("7. Set float value:");
    let json = r#"{"name":"Tom"}"#;
    let result = set_float(json, "score", 95.5, None).unwrap();
    println!("Original: {}", json);
    println!("Set score=95.5: {}", result);
    println!();

    // Set complex object
    println!("8. Set complex object:");
    let json = r#"{"name":"Tom"}"#;
    let result = set_raw(json, "address", r#"{"city":"Beijing","country":"China"}"#).unwrap();
    println!("Original: {}", json);
    println!("Set address: {}", result);
    println!();

    // Set array
    println!("9. Set array:");
    let json = r#"{"name":"Tom"}"#;
    let result = set_raw(json, "hobbies", r#"["reading","swimming","coding"]"#).unwrap();
    println!("Original: {}", json);
    println!("Set hobbies: {}", result);
    println!();

    // Delete value
    println!("10. Delete value:");
    let json = r#"{"name":"Tom","age":37,"city":"Beijing"}"#;
    let result = delete(json, "age").unwrap();
    println!("Original: {}", json);
    println!("Delete age: {}", result);
    println!();

    // Use set_value to set complex types
    println!("11. Use set_value to set complex types:");
    let json = r#"{"name":"Tom"}"#;
    
    // Set struct
    #[derive(serde::Serialize)]
    struct Person {
        name: String,
        age: u32,
    }
    
    let person = Person {
        name: "Jerry".to_string(),
        age: 25,
    };
    
    let result = set_value(json, "friend", &person, None).unwrap();
    println!("Original: {}", json);
    println!("Set friend: {}", result);
    println!();

    // Error handling example
    println!("12. Error handling example:");
    let json = r#"{"name":"Tom"}"#;
    
    match set(json, "", "value") {
        Ok(result) => println!("Success: {}", result),
        Err(e) => println!("Error: {}", e),
    }
    
    match set("invalid json", "name", "value") {
        Ok(result) => println!("Success: {}", result),
        Err(e) => println!("Error: {}", e),
    }
}
