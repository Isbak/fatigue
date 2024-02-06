use evalexpr::{eval_with_context, Context, ContextWithMutableVariables, HashMapContext, Value};
use rayon::result;
use std::collections::HashMap;
use crate::config::Config; // Adjust this import based on your actual module structure

pub fn parse_input(config: &Config) -> Result<HashMap<String, Value>, String> {
    let mut context = HashMapContext::new();
    
    // Insert parameters into context
    for (key, value) in &config.parameters {
        println!("key: {:#?}", key);
        println!("value: {:#?}", value);
        if context.set_value(key.clone(), (*value).into()).is_err() {
            return Err(format!("Failed to insert parameter '{}' into context", key));
        }
    }

    // Insert variables into context with actual values
    for (key, expression) in config.variables.iter() {
        match eval_with_context(&expression, &context) {
            Ok(value) => {
                if context.set_value(key.clone(), value.clone()).is_err() {
                    return Err(format!("Failed to insert variable '{}' with value '{:?}' into context", key, value));
                }
            },
            Err(e) => return Err(format!("Failed to evaluate variable '{}' with expression '{}': {}", key, expression, e)),
        }
    }

    let mut results = HashMap::new();

    // Evaluate expressions based on the specified order
    for key in &config.expressions.order {
        if let Some(expression) = config.variables.get(key).map(|vars| vars) {
            match eval_with_context(expression, &context) {
                Ok(result) => {
                    // Assuming result is of type Value and you want to insert/update it in the context
                    context.set_value(key.clone(), result.clone()).map_err(|e| format!("Failed to update context for key '{}': {}", key, e))?;
    
                    // Also insert the result into the results hashmap
                    results.insert(key.clone(), result);
                },
                Err(e) => {
                    return Err(format!("Failed to evaluate expression '{}' for key '{}': {}", expression, key, e));
                }
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::load_config; // Ensure this is correctly imported

    #[test]
    fn test_parse_input() {
        let config_path = "tests/config.yaml";
        let config = load_config(config_path).expect("Failed to load config");

        println!("config: {:#?}", config);

        let results = parse_input(&config).expect("Failed to parse input");
        println!("Results: {:#?}", results);
        // Example of improved error handling in test assertions
        let max_value_result = results.get("max_value").and_then(|v| v.as_float().ok());
        assert!(max_value_result.is_some(), "max_value not found or not a float");
        assert_eq!(max_value_result.unwrap(), 5.0, "max_value should be 5.0");

        let sin_of_a_result = results.get("sin_of_a").and_then(|v| v.as_float().ok());
        assert!(sin_of_a_result.is_some(), "sin_of_a not found or not a float");
        // Compare floating point numbers within a small range to account for float precision issues
        assert!((sin_of_a_result.unwrap() - f64::sin(5.0)).abs() < 1e-6, "sin_of_a should match the sine of 5.0");
    }
}
