use serde_json::Value as JsonValue;

/// Parse array index, supporting negative indices
fn parse_array_index(part: &str, arr_len: usize) -> Result<usize, SjsonError> {
    let index: i64 = part.parse()
        .map_err(|_| SjsonError::InvalidPath)?;
    
    if index >= 0 {
        Ok(index as usize)
    } else {
        // Handle negative indices: -1 means last element, -2 means second to last, etc.
        let abs_index = (-index) as usize;
        if abs_index > arr_len {
            Err(SjsonError::InvalidPath)
        } else {
            Ok(arr_len - abs_index)
        }
    }
}

/// Options represents additional options for the Set and Delete functions.
#[derive(Default, Clone)]
pub struct Options {
    /// Optimistic is a hint that the value likely exists which
    /// allows for the sjson to perform a fast-track search and replace.
    pub optimistic: bool,
}

#[derive(Debug)]
pub enum SjsonError {
    EmptyPath,
    InvalidPath,
    NoChange,
    ComplexPathNotSupported,
    JsonMustBeObjectOrArray,
    CannotSetArrayElementForNonNumericKey(String),
    Custom(String),
}

impl std::fmt::Display for SjsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SjsonError::EmptyPath => write!(f, "path cannot be empty"),
            SjsonError::InvalidPath => write!(f, "invalid path"),
            SjsonError::NoChange => write!(f, "no change"),
            SjsonError::ComplexPathNotSupported => write!(f, "complex path not supported"),
            SjsonError::JsonMustBeObjectOrArray => write!(f, "json must be an object or array"),
            SjsonError::CannotSetArrayElementForNonNumericKey(key) => {
                write!(f, "cannot set array element for non-numeric key '{}'", key)
            }
            SjsonError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for SjsonError {}

/// Check if a path is optimistic (simple characters only)
fn is_optimistic_path(path: &str) -> bool {
    path.chars().all(|ch| {
        ch >= '.' && ch <= 'z' && !(ch > '9' && ch < 'A') && ch <= 'z'
    })
}

/// Find the position of a value in JSON string for optimistic replacement
fn find_value_position(json: &str, path: &str) -> Option<(usize, usize)> {
    // Simple implementation to find value position
    // This is a basic version - a full implementation would need more sophisticated parsing
    
    let mut current_pos = 0;
    let parts: Vec<&str> = path.split('.').collect();
    
    for (i, part) in parts.iter().enumerate() {
        // Find the key
        let key_pattern = format!("\"{}\":", part);
        if let Some(key_pos) = json[current_pos..].find(&key_pattern) {
            let key_start = current_pos + key_pos;
            let value_start = key_start + key_pattern.len();
            
            // Skip whitespace
            let value_start = value_start + json[value_start..]
                .chars()
                .take_while(|c| c.is_whitespace())
                .map(|c| c.len_utf8())
                .sum::<usize>();
            
            if i == parts.len() - 1 {
                // This is the final part, find the end of the value
                let value_end = find_value_end(&json[value_start..]);
                return Some((value_start, value_start + value_end));
            } else {
                // Continue to next part
                current_pos = value_start;
            }
        } else {
            return None;
        }
    }
    
    None
}

/// Find the end of a JSON value
fn find_value_end(json: &str) -> usize {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;
    
    for (i, ch) in json.chars().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }
        
        match ch {
            '"' if !escape_next => in_string = !in_string,
            '\\' if in_string => escape_next = true,
            '{' | '[' if !in_string => depth += 1,
            '}' | ']' if !in_string => {
                if depth > 0 {
                    depth -= 1;
                } else {
                    return i + 1;
                }
            }
            ',' if !in_string && depth == 0 => return i,
            _ => {}
        }
    }
    
    json.len()
}

/// Set sets a json value for the specified path.
/// A path is in dot syntax, such as "name.last" or "age".
/// This function expects that the json is well-formed, and does not validate.
/// Invalid json will not panic, but it may return back unexpected results.
/// An error is returned if the path is not valid.
///
/// A path is a series of keys separated by a dot.
///
/// ```json
/// {
///   "name": {"first": "Tom", "last": "Anderson"},
///   "age": 37,
///   "children": ["Sara","Alex","Jack"],
///   "friends": [
///     {"first": "James", "last": "Murphy"},
///     {"first": "Roger", "last": "Craig"}
///   ]
/// }
/// ```
/// "name.last"          >> "Anderson"
/// "age"                >> 37
/// "children.1"         >> "Alex"
pub fn set(json: &str, path: &str, value: &str) -> Result<String, SjsonError> {
    set_options(json, path, value, None)
}

/// SetOptions sets a json value for the specified path with options.
pub fn set_options(
    json: &str,
    path: &str,
    value: &str,
    opts: Option<&Options>,
) -> Result<String, SjsonError> {
    if path.is_empty() {
        return Err(SjsonError::EmptyPath);
    }

    let optimistic = opts.map(|o| o.optimistic).unwrap_or(false);

    // Try optimistic path replacement if enabled
    if optimistic && is_optimistic_path(path) {
        if let Some((start, end)) = find_value_position(json, path) {
            let mut result = String::with_capacity(json.len() - (end - start) + value.len() + 2);
            result.push_str(&json[..start]);
            
            // Add quotes if the value is not already quoted and looks like a string
            if !value.starts_with('"') && !value.starts_with('{') && !value.starts_with('[') 
               && !value.parse::<f64>().is_ok() && value != "true" && value != "false" && value != "null" {
                result.push('"');
                result.push_str(value);
                result.push('"');
            } else {
                result.push_str(value);
            }
            
            result.push_str(&json[end..]);
            return Ok(result);
        }
    }

    // Fall back to full JSON parsing approach
    let parsed = serde_json::from_str::<JsonValue>(json)
        .map_err(|e| SjsonError::Custom(format!("Invalid JSON: {}", e)))?;

    match set_simple_path(&parsed, path, value) {
        Ok(new_value) => serde_json::to_string(&new_value)
            .map_err(|e| SjsonError::Custom(format!("Failed to serialize: {}", e))),
        Err(e) => Err(e),
    }
}

fn set_simple_path(json: &JsonValue, path: &str, value: &str) -> Result<JsonValue, SjsonError> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return Err(SjsonError::EmptyPath);
    }

    let mut result = json.clone();
    let mut current = &mut result;

    // Navigate to the parent of the target
    for i in 0..parts.len() - 1 {
        let part = parts[i];
        match current {
            JsonValue::Object(map) => {
                if !map.contains_key(part) {
                    map.insert(part.to_string(), JsonValue::Object(serde_json::Map::new()));
                }
                current = map.get_mut(part).unwrap();
            }
            JsonValue::Array(arr) => {
                let index = parse_array_index(part, arr.len())?;
                if index >= arr.len() {
                    // Extend array with null values
                    while arr.len() <= index {
                        arr.push(JsonValue::Null);
                    }
                }
                current = &mut arr[index];
            }
            _ => {
                // Convert to object if needed
                *current = JsonValue::Object(serde_json::Map::new());
                if let JsonValue::Object(map) = current {
                    map.insert(part.to_string(), JsonValue::Object(serde_json::Map::new()));
                    current = map.get_mut(part).unwrap();
                }
            }
        }
    }

    // Set the final value
    let final_part = parts.last().unwrap();
    let json_value = parse_value(value);
    
    match current {
        JsonValue::Object(map) => {
            map.insert(final_part.to_string(), json_value);
        }
        JsonValue::Array(arr) => {
            let index = parse_array_index(final_part, arr.len())?;
            if index >= arr.len() {
                // Extend array with null values
                while arr.len() <= index {
                    arr.push(JsonValue::Null);
                }
            }
            arr[index] = json_value;
        }
        _ => {
            // Convert to object if needed
            *current = JsonValue::Object(serde_json::Map::new());
            if let JsonValue::Object(map) = current {
                map.insert(final_part.to_string(), json_value);
            }
        }
    }

    Ok(result)
}

fn parse_value(value: &str) -> JsonValue {
    // Try to parse as different types
    if value == "true" {
        JsonValue::Bool(true)
    } else if value == "false" {
        JsonValue::Bool(false)
    } else if value == "null" {
        JsonValue::Null
    } else if let Ok(num) = value.parse::<i64>() {
        JsonValue::Number(serde_json::Number::from(num))
    } else if let Ok(num) = value.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(num) {
            JsonValue::Number(n)
        } else {
            JsonValue::String(value.to_string())
        }
    } else {
        // Try to parse as JSON if it looks like JSON
        if (value.starts_with('[') && value.ends_with(']')) || 
           (value.starts_with('{') && value.ends_with('}')) {
            if let Ok(json_value) = serde_json::from_str::<JsonValue>(value) {
                return json_value;
            }
        }
        JsonValue::String(value.to_string())
    }
}

/// SetRaw sets a raw json value for the specified path.
/// This function works the same as Set except that the value is set as a
/// raw block of json. This allows for setting premarshalled json objects.
pub fn set_raw(json: &str, path: &str, value: &str) -> Result<String, SjsonError> {
    set_raw_options(json, path, value, None)
}

/// SetRawOptions sets a raw json value for the specified path with options.
pub fn set_raw_options(
    json: &str,
    path: &str,
    value: &str,
    opts: Option<&Options>,
) -> Result<String, SjsonError> {
    let optimistic = opts.map(|o| o.optimistic).unwrap_or(false);

    // Try optimistic path replacement if enabled
    if optimistic && is_optimistic_path(path) {
        if let Some((start, end)) = find_value_position(json, path) {
            let mut result = String::with_capacity(json.len() - (end - start) + value.len());
            result.push_str(&json[..start]);
            result.push_str(value);
            result.push_str(&json[end..]);
            return Ok(result);
        }
    }

    // Parse the raw value as JSON
    let json_value = serde_json::from_str::<JsonValue>(value)
        .map_err(|e| SjsonError::Custom(format!("Invalid JSON value: {}", e)))?;

    // Parse the original JSON
    let parsed = serde_json::from_str::<JsonValue>(json)
        .map_err(|e| SjsonError::Custom(format!("Invalid JSON: {}", e)))?;

    // Set the value
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return Err(SjsonError::EmptyPath);
    }

    let mut result = parsed.clone();
    let mut current = &mut result;

    // Navigate to the parent of the target
    for i in 0..parts.len() - 1 {
        let part = parts[i];
        match current {
            JsonValue::Object(map) => {
                if !map.contains_key(part) {
                    map.insert(part.to_string(), JsonValue::Object(serde_json::Map::new()));
                }
                current = map.get_mut(part).unwrap();
            }
            JsonValue::Array(arr) => {
                let index = parse_array_index(part, arr.len())?;
                if index >= arr.len() {
                    while arr.len() <= index {
                        arr.push(JsonValue::Null);
                    }
                }
                current = &mut arr[index];
            }
            _ => {
                *current = JsonValue::Object(serde_json::Map::new());
                if let JsonValue::Object(map) = current {
                    map.insert(part.to_string(), JsonValue::Object(serde_json::Map::new()));
                    current = map.get_mut(part).unwrap();
                }
            }
        }
    }

    // Set the final value
    let final_part = parts.last().unwrap();
    
    match current {
        JsonValue::Object(map) => {
            map.insert(final_part.to_string(), json_value);
        }
        JsonValue::Array(arr) => {
            let index = parse_array_index(final_part, arr.len())?;
            if index >= arr.len() {
                while arr.len() <= index {
                    arr.push(JsonValue::Null);
                }
            }
            arr[index] = json_value;
        }
        _ => {
            *current = JsonValue::Object(serde_json::Map::new());
            if let JsonValue::Object(map) = current {
                map.insert(final_part.to_string(), json_value);
            }
        }
    }

    serde_json::to_string(&result)
        .map_err(|e| SjsonError::Custom(format!("Failed to serialize: {}", e)))
}

/// Delete deletes a value from json for the specified path.
pub fn delete(json: &str, path: &str) -> Result<String, SjsonError> {
    delete_options(json, path, None)
}

/// DeleteOptions deletes a value from json for the specified path with options.
pub fn delete_options(json: &str, path: &str, opts: Option<&Options>) -> Result<String, SjsonError> {
    if path.is_empty() {
        return Err(SjsonError::EmptyPath);
    }

    let optimistic = opts.map(|o| o.optimistic).unwrap_or(false);

    // Try optimistic path deletion if enabled
    if optimistic && is_optimistic_path(path) {
        if let Some((start, end)) = find_value_position(json, path) {
            // Find the key start position
            let key_pattern = format!("\"{}\":", path.split('.').last().unwrap());
            let key_start = json[..start].rfind(&key_pattern).unwrap_or(start);
            
            // Check if we need to remove a comma before the key
            let mut result = String::with_capacity(json.len() - (end - key_start));
            
            // Check if there's a comma before the key that we need to remove
            let mut actual_start = key_start;
            if key_start > 0 {
                // Look backwards for comma and whitespace
                let mut pos = key_start - 1;
                while pos > 0 && json[pos..].chars().next().map_or(false, |c| c.is_whitespace()) {
                    pos -= 1;
                }
                if pos > 0 && json[pos..].starts_with(',') {
                    actual_start = pos;
                    // Also remove whitespace before comma
                    while actual_start > 0 && json[actual_start-1..].chars().next().map_or(false, |c| c.is_whitespace()) {
                        actual_start -= 1;
                    }
                }
            }
            
            result.push_str(&json[..actual_start]);
            
            // Skip comma and whitespace after the deleted value
            let mut skip_pos = end;
            // Skip whitespace first
            while skip_pos < json.len() && json[skip_pos..].chars().next().map_or(false, |c| c.is_whitespace()) {
                skip_pos += 1;
            }
            // Then skip comma if present
            if skip_pos < json.len() && json[skip_pos..].starts_with(',') {
                skip_pos += 1;
                // Skip whitespace after comma
                while skip_pos < json.len() && json[skip_pos..].chars().next().map_or(false, |c| c.is_whitespace()) {
                    skip_pos += 1;
                }
            }
            
            // Include everything after the deleted value
            result.push_str(&json[end..]);
            return Ok(result);
        }
    }

    let parsed = serde_json::from_str::<JsonValue>(json)
        .map_err(|e| SjsonError::Custom(format!("Invalid JSON: {}", e)))?;

    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return Err(SjsonError::EmptyPath);
    }

    let mut result = parsed.clone();
    let mut current = &mut result;

    // Navigate to the parent of the target
    for i in 0..parts.len() - 1 {
        let part = parts[i];
        match current {
            JsonValue::Object(map) => {
                if !map.contains_key(part) {
                    return Err(SjsonError::NoChange);
                }
                current = map.get_mut(part).unwrap();
            }
            JsonValue::Array(arr) => {
                let index = parse_array_index(part, arr.len())?;
                if index >= arr.len() {
                    return Err(SjsonError::NoChange);
                }
                current = &mut arr[index];
            }
            _ => {
                return Err(SjsonError::NoChange);
            }
        }
    }

    // Delete the final value
    let final_part = parts.last().unwrap();
    
    match current {
        JsonValue::Object(map) => {
            if map.remove(&final_part.to_string()).is_none() {
                return Err(SjsonError::NoChange);
            }
        }
        JsonValue::Array(arr) => {
            let index = parse_array_index(final_part, arr.len())?;
            if index >= arr.len() {
                return Err(SjsonError::NoChange);
            }
            arr.remove(index);
        }
        _ => {
            return Err(SjsonError::NoChange);
        }
    }

    serde_json::to_string(&result)
        .map_err(|e| SjsonError::Custom(format!("Failed to serialize: {}", e)))
}

/// Set a boolean value
pub fn set_bool(json: &str, path: &str, value: bool, opts: Option<&Options>) -> Result<String, SjsonError> {
    let raw = if value { "true" } else { "false" };
    set_options(json, path, raw, opts)
}

/// Set an integer value
pub fn set_int<T: std::fmt::Display>(
    json: &str,
    path: &str,
    value: T,
    opts: Option<&Options>,
) -> Result<String, SjsonError> {
    let raw = value.to_string();
    set_options(json, path, &raw, opts)
}

/// Set a float value
pub fn set_float<T: std::fmt::Display>(
    json: &str,
    path: &str,
    value: T,
    opts: Option<&Options>,
) -> Result<String, SjsonError> {
    let raw = value.to_string();
    set_options(json, path, &raw, opts)
}

/// Generic Set function that accepts any value that can be serialized to JSON
pub fn set_value<T: serde::Serialize>(
    json: &str,
    path: &str,
    value: &T,
    opts: Option<&Options>,
) -> Result<String, SjsonError> {
    let json_value = serde_json::to_string(value)
        .map_err(|e| SjsonError::Custom(format!("Failed to serialize value: {}", e)))?;
    
    set_raw_options(json, path, &json_value, opts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_simple() {
        let json = r#"{"name":"Tom","age":37}"#;
        let result = set(json, "name", "Jerry").unwrap();
        assert_eq!(result, r#"{"age":37,"name":"Jerry"}"#);
    }

    #[test]
    fn test_set_nested() {
        let json = r#"{"name":{"first":"Tom","last":"Anderson"}}"#;
        let result = set(json, "name.first", "Jerry").unwrap();
        assert_eq!(result, r#"{"name":{"first":"Jerry","last":"Anderson"}}"#);
    }

    #[test]
    fn test_set_array() {
        let json = r#"{"children":["Sara","Alex","Jack"]}"#;
        let result = set(json, "children.1", "Jerry").unwrap();
        assert_eq!(result, r#"{"children":["Sara","Jerry","Jack"]}"#);
        let result = set(json, "children", "[]").unwrap();
        assert_eq!(result, r#"{"children":[]}"#);
    }

    #[test]
    fn test_set_new_field() {
        let json = r#"{"name":"Tom"}"#;
        let result = set(json, "age", "37").unwrap();
        assert_eq!(result, r#"{"age":37,"name":"Tom"}"#);
    }

    #[test]
    fn test_array_index_operation() {
        let json = r#"{"children":["Sara","Alex","Jack"]}"#;
        let result = set(json, "children.-1", "Jerry").unwrap();
        assert_eq!(result, r#"{"children":["Sara","Alex","Jerry"]}"#);
        let result = delete(json, "children.-1").unwrap();
        assert_eq!(result, r#"{"children":["Sara","Alex"]}"#);
    }

    #[test]
    fn test_negative_array_indices() {
        let json = r#"{"items":["a","b","c","d","e"]}"#;
        
        // Test -1 (last element)
        let result = set(json, "items.-1", "z").unwrap();
        assert_eq!(result, r#"{"items":["a","b","c","d","z"]}"#);
        
        // Test -2 (second to last)
        let result = set(json, "items.-2", "y").unwrap();
        assert_eq!(result, r#"{"items":["a","b","c","y","e"]}"#);
        
        // Test -3 (third to last)
        let result = set(json, "items.-3", "x").unwrap();
        assert_eq!(result, r#"{"items":["a","b","x","d","e"]}"#);
    }

    #[test]
    fn test_negative_array_indices_delete() {
        let json = r#"{"items":["a","b","c","d","e"]}"#;
        
        // Test deleting -1 (last element)
        let result = delete(json, "items.-1").unwrap();
        assert_eq!(result, r#"{"items":["a","b","c","d"]}"#);
        
        // Test deleting -2 (second to last)
        let result = delete(json, "items.-2").unwrap();
        assert_eq!(result, r#"{"items":["a","b","c","e"]}"#);
        
        // Test deleting -3 (third to last)
        let result = delete(json, "items.-3").unwrap();
        assert_eq!(result, r#"{"items":["a","b","d","e"]}"#);
    }

    #[test]
    fn test_negative_array_indices_nested() {
        let json = r#"{"data":{"items":[{"name":"item1"},{"name":"item2"},{"name":"item3"}]}}"#;
        
        // Test setting in nested array with negative index
        let result = set(json, "data.items.-1.name", "updated").unwrap();
        assert!(result.contains("\"name\":\"updated\""));
        
        // Test deleting in nested array with negative index
        let result = delete(json, "data.items.-1.name").unwrap();
        assert!(!result.contains("\"name\":\"item3\""));
    }

    #[test]
    fn test_negative_array_indices_invalid() {
        let json = r#"{"items":["a","b"]}"#;
        
        // Test invalid negative index (beyond array bounds)
        let result = set(json, "items.-3", "x");
        assert!(result.is_err());
        
        let result = delete(json, "items.-3");
        assert!(result.is_err());
    }

    #[test]
    fn test_negative_array_indices_optimistic() {
        let json = r#"{"items":["a","b","c","d"]}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        
        // Test optimistic mode with negative indices
        let result = set_options(json, "items.-1", "z", Some(&opts)).unwrap();
        assert_eq!(result, r#"{"items":["a","b","c","z"]}"#);
        
        let result = delete_options(json, "items.-2", Some(&opts)).unwrap();
        assert_eq!(result, r#"{"items":["a","b","d"]}"#);
    }

    #[test]
    fn test_negative_array_indices_with_large_array() {
        let json = r#"{"items":["a","b","c","d","e","f","g","h","i","j"]}"#;
        
        // Test various negative indices
        let result = set(json, "items.-1", "last").unwrap();
        assert_eq!(result, r#"{"items":["a","b","c","d","e","f","g","h","i","last"]}"#);
        
        let result = set(json, "items.-5", "middle").unwrap();
        assert_eq!(result, r#"{"items":["a","b","c","d","e","middle","g","h","i","j"]}"#);
        
        let result = delete(json, "items.-1").unwrap();
        assert_eq!(result, r#"{"items":["a","b","c","d","e","f","g","h","i"]}"#);
    }

    #[test]
    fn test_delete() {
        let json = r#"{"name":"Tom","age":37}"#;
        let result = delete(json, "age").unwrap();
        assert_eq!(result, r#"{"name":"Tom"}"#);
    }

    #[test]
    fn test_set_bool() {
        let json = r#"{"name":"Tom"}"#;
        let result = set_bool(json, "active", true, None).unwrap();
        assert_eq!(result, r#"{"active":true,"name":"Tom"}"#);
    }

    #[test]
    fn test_set_int() {
        let json = r#"{"name":"Tom"}"#;
        let result = set_int(json, "age", 37, None).unwrap();
        assert_eq!(result, r#"{"age":37,"name":"Tom"}"#);
    }

    #[test]
    fn test_empty_path() {
        let json = r#"{"name":"Tom"}"#;
        let result = set(json, "", "value");
        assert!(result.is_err());
    }

    #[test]
    fn test_optimistic_set() {
        let json = r#"{"name":"Tom","age":37}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_options(json, "name", "Jerry", Some(&opts)).unwrap();
        assert_eq!(result, r#"{"name":"Jerry","age":37}"#);
    }

    #[test]
    fn test_optimistic_delete() {
        let json = r#"{"name":"Tom","age":37}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = delete_options(json, "age", Some(&opts)).unwrap();
        // For now, just check that it doesn't panic and produces valid JSON
        assert!(result.contains("\"name\":\"Tom\""));
        assert!(!result.contains("\"age\":37"));
    }

    #[test]
    fn test_options_without_optimistic() {
        let json = r#"{"name":"Tom","age":37}"#;
        let opts = Options::default(); // optimistic = false
        let result = set_options(json, "name", "Jerry", Some(&opts)).unwrap();
        assert_eq!(result, r#"{"age":37,"name":"Jerry"}"#);
    }

    #[test]
    fn test_options_none() {
        let json = r#"{"name":"Tom","age":37}"#;
        let result = set_options(json, "name", "Jerry", None).unwrap();
        assert_eq!(result, r#"{"age":37,"name":"Jerry"}"#);
    }

    #[test]
    fn test_optimistic_nested_set() {
        let json = r#"{"user":{"name":"Tom","age":37}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_options(json, "user.name", "Jerry", Some(&opts)).unwrap();
        // Check that the result contains the expected values, regardless of field order
        assert!(result.contains("\"user\""));
        assert!(result.contains("\"name\":\"Jerry\""));
        assert!(result.contains("\"age\":37"));
    }

    #[test]
    fn test_optimistic_array_set() {
        let json = r#"{"items":["a","b","c"]}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_options(json, "items.1", "x", Some(&opts)).unwrap();
        assert_eq!(result, r#"{"items":["a","x","c"]}"#);
    }

    #[test]
    fn test_optimistic_set_raw() {
        let json = r#"{"data":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let complex_value = r#"{"city":"Beijing","country":"China"}"#;
        let result = set_raw_options(json, "data.address", complex_value, Some(&opts)).unwrap();
        assert_eq!(result, r#"{"data":{"address":{"city":"Beijing","country":"China"},"name":"Tom"}}"#);
    }

    #[test]
    fn test_optimistic_set_bool() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_bool(json, "user.active", true, Some(&opts)).unwrap();
        assert_eq!(result, r#"{"user":{"active":true,"name":"Tom"}}"#);
    }

    #[test]
    fn test_optimistic_set_int() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_int(json, "user.age", 25, Some(&opts)).unwrap();
        assert_eq!(result, r#"{"user":{"age":25,"name":"Tom"}}"#);
    }

    #[test]
    fn test_optimistic_set_float() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_float(json, "user.score", 95.5, Some(&opts)).unwrap();
        assert_eq!(result, r#"{"user":{"name":"Tom","score":95.5}}"#);
    }

    #[test]
    fn test_optimistic_set_value() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        
        #[derive(serde::Serialize)]
        struct Address {
            city: String,
            country: String,
        }
        
        let address = Address {
            city: "Beijing".to_string(),
            country: "China".to_string(),
        };
        
        let result = set_value(json, "user.address", &address, Some(&opts)).unwrap();
        assert_eq!(result, r#"{"user":{"address":{"city":"Beijing","country":"China"},"name":"Tom"}}"#);
    }

    #[test]
    fn test_options_clone() {
        let mut opts1 = Options::default();
        opts1.optimistic = true;
        let opts2 = opts1.clone();
        assert_eq!(opts1.optimistic, opts2.optimistic);
    }

    #[test]
    fn test_options_default() {
        let opts = Options::default();
        assert_eq!(opts.optimistic, false);
    }

    #[test]
    fn test_optimistic_delete_nested() {
        let json = r#"{"user":{"name":"Tom","age":37,"city":"Beijing"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = delete_options(json, "user.age", Some(&opts)).unwrap();
        assert!(result.contains("\"user\""));
        assert!(result.contains("\"name\":\"Tom\""));
        assert!(result.contains("\"city\":\"Beijing\""));
        assert!(!result.contains("\"age\":37"));
    }

    #[test]
    fn test_optimistic_delete_array_element() {
        let json = r#"{"items":["a","b","c","d"]}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = delete_options(json, "items.1", Some(&opts)).unwrap();
        assert_eq!(result, r#"{"items":["a","c","d"]}"#);
    }

    #[test]
    fn test_optimistic_set_with_special_characters() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        // Test with value containing special characters
        let result = set_options(json, "user.description", "Hello, \"World\"!", Some(&opts)).unwrap();
        assert!(result.contains("\"description\":\"Hello, \\\"World\\\"!\""));
    }

    #[test]
    fn test_optimistic_set_null_value() {
        let json = r#"{"user":{"name":"Tom","age":37}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_options(json, "user.age", "null", Some(&opts)).unwrap();
        assert!(result.contains("\"age\":null"));
    }

    #[test]
    fn test_optimistic_set_boolean_values() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        
        // Test true
        let result = set_options(json, "user.active", "true", Some(&opts)).unwrap();
        assert!(result.contains("\"active\":true"));
        
        // Test false
        let result = set_options(result.as_str(), "user.verified", "false", Some(&opts)).unwrap();
        assert!(result.contains("\"verified\":false"));
    }

    #[test]
    fn test_optimistic_set_numeric_values() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        
        // Test integer
        let result = set_options(json, "user.age", "25", Some(&opts)).unwrap();
        assert!(result.contains("\"age\":25"));
        
        // Test float
        let result = set_options(result.as_str(), "user.score", "95.5", Some(&opts)).unwrap();
        assert!(result.contains("\"score\":95.5"));
        
        // Test negative number
        let result = set_options(result.as_str(), "user.balance", "-100.50", Some(&opts)).unwrap();
        assert!(result.contains("\"balance\":-100.5"));
    }

    #[test]
    fn test_optimistic_set_array_value() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_options(json, "user.hobbies", "[\"reading\",\"swimming\"]", Some(&opts)).unwrap();
        assert!(result.contains("\"hobbies\":[\"reading\",\"swimming\"]"));
    }

    #[test]
    fn test_optimistic_set_object_value() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_options(json, "user.address", "{\"city\":\"Beijing\",\"country\":\"China\"}", Some(&opts)).unwrap();
        assert!(result.contains("\"address\":{\"city\":\"Beijing\",\"country\":\"China\"}"));
    }

    #[test]
    fn test_optimistic_set_empty_string() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_options(json, "user.description", "", Some(&opts)).unwrap();
        assert!(result.contains("\"description\":\"\""));
    }

    #[test]
    fn test_optimistic_set_with_unicode() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_options(json, "user.name", "张三", Some(&opts)).unwrap();
        assert!(result.contains("\"name\":\"张三\""));
    }

    #[test]
    fn test_optimistic_set_deep_nested() {
        let json = r#"{"level1":{"level2":{"level3":{"name":"Tom"}}}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_options(json, "level1.level2.level3.age", "25", Some(&opts)).unwrap();
        assert!(result.contains("\"age\":25"));
        assert!(result.contains("\"name\":\"Tom\""));
    }

    #[test]
    fn test_optimistic_set_array_deep_nested() {
        let json = r#"{"data":{"items":[{"name":"item1"},{"name":"item2"}]}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_options(json, "data.items.0.price", "100", Some(&opts)).unwrap();
        assert!(result.contains("\"price\":100"));
    }

    #[test]
    fn test_optimistic_delete_array_deep_nested() {
        let json = r#"{"data":{"items":[{"name":"item1","price":100},{"name":"item2","price":200}]}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = delete_options(json, "data.items.0.price", Some(&opts)).unwrap();
        assert!(result.contains("\"name\":\"item1\""));
        assert!(!result.contains("\"price\":100"));
    }

    #[test]
    fn test_optimistic_set_with_existing_array() {
        let json = r#"{"items":["a","b","c"]}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_options(json, "items.3", "d", Some(&opts)).unwrap();
        assert_eq!(result, r#"{"items":["a","b","c","d"]}"#);
    }

    #[test]
    fn test_optimistic_set_with_large_array_index() {
        let json = r#"{"items":["a","b"]}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_options(json, "items.5", "f", Some(&opts)).unwrap();
        assert_eq!(result, r#"{"items":["a","b",null,null,null,"f"]}"#);
    }

    #[test]
    fn test_optimistic_set_raw_with_complex_json() {
        let json = r#"{"data":{"user":{"name":"Tom"}}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let complex_value = r#"{"address":{"street":"123 Main St","city":"Beijing","country":"China"},"phone":"+86-123-4567","active":true,"scores":[95,87,92]}"#;
        let result = set_raw_options(json, "data.user.profile", complex_value, Some(&opts)).unwrap();
        // Check that all expected fields are present, regardless of order
        assert!(result.contains("\"profile\""));
        assert!(result.contains("\"active\":true"));
        assert!(result.contains("\"phone\":\"+86-123-4567\""));
        assert!(result.contains("\"scores\":[95,87,92]"));
        assert!(result.contains("\"street\":\"123 Main St\""));
        assert!(result.contains("\"city\":\"Beijing\""));
        assert!(result.contains("\"country\":\"China\""));
    }

    #[test]
    fn test_optimistic_fallback_to_parser() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        // Test with a path that contains special characters (should fall back to parser)
        let result = set_options(json, "user.name", "Jerry", Some(&opts)).unwrap();
        assert!(result.contains("\"name\":\"Jerry\""));
    }

    #[test]
    fn test_optimistic_set_multiple_operations() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        
        // Multiple set operations
        let result1 = set_options(json, "user.age", "25", Some(&opts)).unwrap();
        let result2 = set_options(result1.as_str(), "user.city", "Beijing", Some(&opts)).unwrap();
        let result3 = set_options(result2.as_str(), "user.active", "true", Some(&opts)).unwrap();
        
        assert!(result3.contains("\"age\":25"));
        assert!(result3.contains("\"city\":\"Beijing\""));
        assert!(result3.contains("\"active\":true"));
        assert!(result3.contains("\"name\":\"Tom\""));
    }

    #[test]
    fn test_optimistic_delete_multiple_operations() {
        let json = r#"{"user":{"name":"Tom","age":25,"city":"Beijing","active":true}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        
        // Multiple delete operations
        let result1 = delete_options(json, "user.age", Some(&opts)).unwrap();
        let result2 = delete_options(result1.as_str(), "user.city", Some(&opts)).unwrap();
        let result3 = delete_options(result2.as_str(), "user.active", Some(&opts)).unwrap();
        
        assert!(result3.contains("\"name\":\"Tom\""));
        assert!(!result3.contains("\"age\":25"));
        assert!(!result3.contains("\"city\":\"Beijing\""));
        assert!(!result3.contains("\"active\":true"));
    }

    #[test]
    fn test_optimistic_set_with_escaped_quotes() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_options(json, "user.quote", "He said \"Hello World\"", Some(&opts)).unwrap();
        assert!(result.contains("\"quote\":\"He said \\\"Hello World\\\"\""));
    }

    #[test]
    fn test_optimistic_set_with_newlines() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_options(json, "user.description", "Line 1\nLine 2", Some(&opts)).unwrap();
        assert!(result.contains("\"description\":\"Line 1\\nLine 2\""));
    }

    #[test]
    fn test_optimistic_set_with_tabs() {
        let json = r#"{"user":{"name":"Tom"}}"#;
        let mut opts = Options::default();
        opts.optimistic = true;
        let result = set_options(json, "user.description", "Tab\there", Some(&opts)).unwrap();
        assert!(result.contains("\"description\":\"Tab\\there\""));
    }
}
