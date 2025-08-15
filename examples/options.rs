use sjson::{set_options, set_raw_options, delete_options, Options};

fn main() {
    println!("=== sjson Options Feature Examples ===\n");

    // 1. Basic Options usage
    println!("1. Basic Options usage:");
    let json = r#"{"name":"Tom","age":37}"#;
    let mut opts = Options::default();
    opts.optimistic = true;
    
    let result = set_options(json, "name", "Jerry", Some(&opts)).unwrap();
    println!("Original: {}", json);
    println!("Using optimistic=true: {}", result);
    println!();

    // 2. Without Options (default behavior)
    println!("2. Without Options (default behavior):");
    let result = set_options(json, "name", "Jerry", None).unwrap();
    println!("Original: {}", json);
    println!("Without Options: {}", result);
    println!();

    // 3. Set complex object
    println!("3. Set complex object:");
    let json = r#"{"user":{"name":"Tom"}}"#;
    let mut opts = Options::default();
    opts.optimistic = true;
    
    let complex_value = r#"{"city":"Beijing","country":"China","population":21540000}"#;
    let result = set_raw_options(json, "user.address", complex_value, Some(&opts)).unwrap();
    println!("Original: {}", json);
    println!("Set complex address: {}", result);
    println!();

    // 4. Delete operation
    println!("4. Delete operation:");
    let json = r#"{"name":"Tom","age":37,"city":"Beijing"}"#;
    let mut opts = Options::default();
    opts.optimistic = true;
    
    let result = delete_options(json, "age", Some(&opts)).unwrap();
    println!("Original: {}", json);
    println!("Delete age: {}", result);
    println!();

    // 5. Performance comparison
    println!("5. Performance comparison:");
    let json = r#"{"name":"Tom","age":37,"city":"Beijing","country":"China"}"#;
    
    // Using optimistic
    let mut opts = Options::default();
    opts.optimistic = true;
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = set_options(json, "name", "Jerry", Some(&opts)).unwrap();
    }
    let optimistic_time = start.elapsed();
    
    // Without optimistic
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = set_options(json, "name", "Jerry", None).unwrap();
    }
    let normal_time = start.elapsed();
    
    println!("Optimistic mode 1000 operations time: {:?}", optimistic_time);
    println!("Normal mode 1000 operations time: {:?}", normal_time);
    println!("Performance improvement: {:.2}x", normal_time.as_nanos() as f64 / optimistic_time.as_nanos() as f64);
    println!();

    // 6. Error handling
    println!("6. Error handling:");
    let json = r#"{"name":"Tom"}"#;
    let mut opts = Options::default();
    opts.optimistic = true;
    
    match set_options(json, "", "value", Some(&opts)) {
        Ok(result) => println!("Success: {}", result),
        Err(e) => println!("Error: {}", e),
    }
    
    match set_options("invalid json", "name", "value", Some(&opts)) {
        Ok(result) => println!("Success: {}", result),
        Err(e) => println!("Error: {}", e),
    }
}
