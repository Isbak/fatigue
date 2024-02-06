use evalexpr::{eval_with_context, ContextWithMutableVariables, HashMapContext, Value};
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
    for key in &config.expressions.order {
        let expression = config.variables
        .get(key)
        .ok_or_else(|| format!("Variable '{}' not found in config", key))?;
        match eval_with_context(expression, &context) {
            Ok(result) => {
                // Insert the result of the evaluation into the context
                if context.set_value(key.to_string(), result.clone()).is_err() {
                    return Err(format!("Failed to insert result for variable '{}' into context", key));
                }
            },
            Err(e) => return Err(format!("Failed to evaluate expression for variable '{}': {}", key, e)),
        }
    }

    let mut results = HashMap::new();
    // Evaluate expressions based on the specified order
    for key in &config.expressions.order {
        if let Some(expression) = config.variables.get(key).map(|vars| vars) {
            match eval_with_context(expression, &context) {
                Ok(result) => {   
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

        let final_expression = results.get("final_expression").and_then(|v| v.as_float().ok());
        assert!((final_expression.unwrap() - 22.051083228736417).abs() < 1e-6, "Should match 22.0510832");
    }
}
