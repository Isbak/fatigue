//! Contains the `TimeSeries` struct and related functionality for time series analysis.
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use regex::Regex;
use serde_json::from_str;
use std::path::Path;
use std::fs::{File, read_to_string};
use std::io::BufReader;
use evalexpr::{eval_with_context, ContextWithMutableVariables, HashMapContext, Value};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use crate::config::ValidationError;
use crate::interpolate::{NDInterpolation, InterpolationStrategyEnum, Linear, NearestNeighbor};
use crate::stress::read_stress_tensors_from_file;

const TOLERANCE: f64 = 1e-5; // Example tolerance level

#[derive(Debug, Deserialize)]
pub struct TimeSeries {
    pub path: String,
    pub sensorfile: String,
    pub interpolations: Vec<Interpolation>,
    pub loadcases: Vec<LoadCase>,
    pub parameters: HashMap<String, f64>,
    pub variables: HashMap<String, String>,
    pub expressions: Expressions,
}

#[derive(Debug, Deserialize)]
pub struct LoadCase {
    pub fam: usize,
    pub file: String,
    pub frequency: f64,
    pub gf_ext: f64,
    pub gf_fat: f64,
}

impl LoadCase {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.file.trim().is_empty() {
            return Err(ValidationError::new("file must not be empty".into()));
        }
        if self.frequency < 0.0 {
            return Err(ValidationError::new(&format!("frequency must be greater than 0.0, got {}", self.frequency)));
        }
        if self.gf_ext < 0.0 {
            return Err(ValidationError::new(&format!("gf_ext must be greater than 0.0, got {}", self.gf_ext)));
        }
        if self.gf_fat < 0.0 {
            return Err(ValidationError::new(&format!("gf_fat must be greater than 0.0, got {}", self.gf_fat)));
        }
        Ok(())
    }
}


#[derive(Debug, Deserialize)]
pub struct ParseConfig {
    pub header: usize,
    pub delimiter: String,
}

impl ParseConfig {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.delimiter.is_empty() {
            return Err(ValidationError::new("delimiter must not be empty".into()));
        }
        Ok(())
    }
}

/// Represents the interpolation properties for a structural analysis application.
#[derive(Debug, Deserialize)]
pub struct Interpolation {
    pub method: String,
    pub name: String,
    pub path: String,
    pub parse_config: ParseConfig,
    pub scale: f64,
    pub dimension: usize,
    pub sensor: Vec<String>,
    pub points: Vec<Point>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Point {
    pub file: Option<String>,
    pub coordinates: Vec<f64>,
}

impl Point {
    pub fn new(file: Option<String>, coordinates: Vec<f64>) -> Self {
        Point { file, coordinates }
    }
}

impl Eq for Point {}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.coordinates.len() == other.coordinates.len() &&
        self.coordinates.iter().zip(other.coordinates.iter()).all(|(a, b)| (a - b).abs() <= TOLERANCE)
    }
}

impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for &coord in &self.coordinates {
            // Discretize the coordinate by rounding it to the precision defined by TOLERANCE
            let discretized = (coord / TOLERANCE).round() * TOLERANCE;
            discretized.to_bits().hash(state); // Hash the bitwise representation of the discretized value
        }
    }
}


/// Interpolation configuration for a structural analysis application.
impl Interpolation {
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.parse_config.validate()?;
        match self.method.as_str() {
            "LINEAR" | "NEAREST" | "NONE" => Ok(()),
            _ => Err(ValidationError::new(&format!("method must be LINEAR, NEAREST, or NONE, got {}", self.method))),
        }?;
        if self.name.trim().is_empty() {
            return Err(ValidationError::new("name must not be empty".into()));
        }

        if self.path.trim().is_empty() {
            return Err(ValidationError::new("path must not be empty".into()));
        }
        if self.scale < 0.0 {
            return Err(ValidationError::new(&format!("scale must be greater than 0.0, got {}", self.scale)));
        }
        // Validate the dimension and sensor vector length condition
        if self.sensor.len() != self.dimension {
            return Err(ValidationError::new(&format!("When dimension is {}, the sensor vector must also have a length of 3. Found length: {}", self.dimension, self.sensor.len())));
        }
        if self.sensor.is_empty() {
            return Err(ValidationError::new("sensor must not be empty".into()));
        }

        if self.points.is_empty() {
            return Err(ValidationError::new("points must not be empty".into()));
        }
        for point in &self.points {
            if point.file.as_ref().unwrap().trim().is_empty() {
                return Err(ValidationError::new("file must not be empty".into()));
            }
            if point.coordinates.len() != self.dimension {
                return Err(ValidationError::new(&format!("When dimension is {}, the values per point must also have a length of 3. Found length: {}", self.dimension, point.coordinates.len())));
            }
            if point.coordinates.is_empty() {
                return Err(ValidationError::new("value must not be empty".into()));
            }
        }
        Ok(())
    }
}

/// Represents a series of time-dependent data used for structural analysis.
///
/// This struct holds the configuration and data necessary for conducting time series
/// analysis, including paths to sensor files and load cases, as well as definitions
/// for interpolation and variable validation.
impl TimeSeries {
    /// Validates the configuration and data of the `TimeSeries`.
    ///
    /// This method ensures that:
    /// - The specified sensor file exists and is not empty.
    /// - The specified path for time series data is valid.
    /// - Interpolations and load cases are specified and valid.
    ///
    /// It also checks the validity of variable names and values against a set of predefined rules.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all configurations and data are valid. Otherwise,
    /// returns a `ValidationError` detailing the specific issue encountered.

    pub fn validate(&self) -> Result<(), ValidationError> {
        // Validates the existence of the sensor file and the correctness of specified paths.
        // Also ensures that interpolations and load cases are properly defined.
        // Detailed validation of each component is performed to ensure data integrity.

        self.expressions.validate()?;
        self.validate_variables_and_values()?;
        if !Path::new(&self.sensorfile).exists() {
            return Err(ValidationError::new("sensorfile does not exist".into()));
        }        
        if self.path.trim().is_empty() {
            return Err(ValidationError::new("path must not be empty".into()));
        }
        if self.sensorfile.trim().is_empty() {
            return Err(ValidationError::new("sensorfile must not be empty".into()));
        }
        if self.interpolations.is_empty() {
            return Err(ValidationError::new("interpolations must not be empty".into()));
        }
        for interp in &self.interpolations {
            interp.validate()?;
        }
        if self.loadcases.is_empty() {
            return Err(ValidationError::new("loadcases must not be empty".into()));
        }
        for lc in &self.loadcases {
            // Construct the full path for the loadcase file
            lc.validate()?;
            let full_path = format!("{}/{}", self.path.trim(), lc.file.trim());
            
            if !Path::new(&full_path).exists() {
                return Err(ValidationError::new(&format!("loadcase file does not exist: {}", full_path)));
            }            
        }
        Ok(())
    }
    /// Reads and deserializes the sensor file specified in the `TimeSeries` configuration.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `SensorFile` structs if successful.
    /// Otherwise, returns an error detailing the issue encountered during file reading or deserialization.

    pub fn read_sensorfile(&self) -> Result<Vec<SensorFile>, Box<dyn std::error::Error>> {
        // Reads the sensor file, deserializes its content into `SensorFile` structs, 
        // and returns them for further processing.        
        let content = read_to_string(&self.sensorfile)?;
        let sensors: Vec<SensorFile> = match from_str(&content) {
            Ok(sensors) => sensors,
            Err(e) => {
                println!("Error parsing JSON: {:?}", e);
                Vec::new()
            }
        };
        Ok(sensors)
    }

    /// Validates the expressions defined within the `TimeSeries` configuration.
    ///
    /// This method checks if any of the defined expressions contain valid variable names
    /// as per the predefined rules and available data.
    ///
    /// # Arguments
    ///
    /// * `expression` - The expression string to validate.
    /// * `valid_names` - A set of valid variable names to check against.
    ///
    /// # Returns
    ///
    /// Returns `true` if the expression is valid, otherwise returns `false`.
    fn expression_valid(&self, expression: &str, valid_names: &HashSet<String>) -> bool {
        // Simplified logic to check if the expression contains any valid variable names.
        valid_names.iter().any(|name| expression.contains(name))
    }
    /// Validates variables and values against predefined rules and configurations.
    ///
    /// This method checks the validity of variable names and values, ensuring they
    /// adhere to predefined naming conventions and value ranges.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all variables and values are valid. Otherwise,
    /// returns a `ValidationError` with details on the specific validation failure.

    fn validate_variables_and_values(&self) -> Result<(), ValidationError> {
        // Validates variable names and values, ensuring compliance with rules.
        let mut valid_names = HashSet::<String>::new();
        let re = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();
        // Add parameters and variables to valid names
        self.parameters.iter().for_each(|(key, _)| { valid_names.insert(key.clone()); });
        self.variables.iter().for_each(|(key, _)| { valid_names.insert(key.clone()); });
        for interp in self.interpolations.iter() {
            valid_names.insert(interp.name.clone());
            interp.sensor.iter().for_each(|sensor| { valid_names.insert(sensor.clone()); });
        }
        for (key, value) in &self.variables {
            if !re.is_match(key) {
                return Err(ValidationError::new(&format!("Invalid variable name: {}", key)));
            }
            if value.trim().is_empty() {
                return Err(ValidationError::new(&format!("Variable expression is empty for: {}", key)));
            }
        }
        for (key, value) in &self.parameters {
            if key.trim().is_empty() {
                return Err(ValidationError::new("parameter key must not be empty".into()));
            }
            if value.is_nan() {
                return Err(ValidationError::new(&format!("parameter value must be a number, got {}", value)));
            }
        }
        // Validate variables expressions
        for (name, expression) in &self.variables {
            if !self.expression_valid(expression, &valid_names) {
                return Err(ValidationError::new(&format!("Invalid expression for variable '{}': {}", name, expression)));
            }
        }
        Ok(())
    }

     pub fn parse_input(&self) -> Result<HashMap<String, Value>, String> {
        let mut context = HashMapContext::new();
    
        // Insert parameters into context
        for (key, value) in &self.parameters {
            println!("key: {:#?}", key);
            println!("value: {:#?}", value);
            if context.set_value(key.clone(), (*value).into()).is_err() {
                return Err(format!("Failed to insert parameter '{}' into context", key));
            }
        }
    
        // Insert variables into context with actual values
        for key in &self.expressions.order {
            let expression = self.variables
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
        for key in &self.expressions.order {
            if let Some(expression) = self.variables.get(key).map(|vars| vars) {
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

    fn interpolate(&self, /* interpolation parameters */) -> Result<(), String> {
        for interp in self.interpolations.iter() {
            // Revised strategy instantiation using the enum
            let strategy = match interp.method.as_str() {
                "LINEAR" => InterpolationStrategyEnum::Linear(Linear{}),
                "NEAREST" => InterpolationStrategyEnum::NearestNeighbor(NearestNeighbor{}),
                _ => return Err("Unsupported interpolation method".to_string()),
            };

            // Initialize NDInterpolation with the chosen strategy
            let mut interpolator_map: HashMap<usize, HashMap<String, NDInterpolation>> = HashMap::new();
            if let Some(ref file_name) = interp.points[0].file {
                let path = PathBuf::from(&interp.path).join(file_name);
                let tensors = read_stress_tensors_from_file(&path).unwrap(); // Handle the Result using `?`
                for tensor in tensors.iter() {
                    // Retrieve or create the inner HashMap for the current tensor (node)
                    let node_map = interpolator_map.entry(tensor.0).or_insert_with(HashMap::new);
                    // Insert NDInterpolation instances for SXX, SYY, and SZZ
                    node_map.insert("SXX".to_string(), NDInterpolation::new(&strategy));
                    node_map.insert("SYY".to_string(), NDInterpolation::new(&strategy));
                    node_map.insert("SZZ".to_string(), NDInterpolation::new(&strategy));
                    node_map.insert("SXY".to_string(), NDInterpolation::new(&strategy));         
                    node_map.insert("SYZ".to_string(), NDInterpolation::new(&strategy));         
                    node_map.insert("SZX".to_string(), NDInterpolation::new(&strategy));                                    
                }
            }
            // Assuming interp.path does not change, move the PathBuf construction outside the first loop.
            let base_path = PathBuf::from(&interp.path);

            for point in &interp.points {
                if let Some(ref file_name) = point.file {
                    let path = base_path.join(file_name);
                    // Use `?` for error propagation instead of `unwrap()`
                    let tensors = read_stress_tensors_from_file(&path).unwrap();

                    for tensor in &tensors {
                        // Static mapping of components to methods; consider defining this outside of your loop if applicable.
                        let components_and_methods = [
                            ("SXX", tensor.1.sxx()),
                            ("SYY", tensor.1.syy()),
                            ("SZZ", tensor.1.szz()),
                            ("SXY", tensor.1.sxy()),
                            ("SYZ", tensor.1.syz()),
                            ("SZX", tensor.1.szx()),
                        ];
                        if let Some(inner_map) = interpolator_map.get_mut(&tensor.0) {
                            for (component, value) in components_and_methods.iter() {
                                if let Some(nd_interpolation) = inner_map.get_mut(*component) {
                                    nd_interpolation.add_point(point.clone(), *value);
                                } else {
                                    // Handle missing stress component in map for current node, if necessary.
                                }
                            }
                        } else {
                            // Handle missing node in `interpolator_map` more gracefully or log error as needed.
                            return Err(format!("Node {} not found in interpolator_map", tensor.0).into());
                        }
                    }
                }
            }

            for lc in self.loadcases.iter(){
                let _sensor = self.read_sensorfile().unwrap();
                let path = PathBuf::from(&self.path).join(&lc.file);
                let file = File::open(path).unwrap();
                let _reader = BufReader::new(file);
            }
        }
        Ok(())
    }

}

/// Represents the order in which expressions should be evaluated in a structural analysis context.
///
/// This struct holds an ordered list of expression names, defining the sequence in which
/// calculations or operations should be executed. The order is critical for ensuring that
/// dependencies between expressions are correctly managed, and results are accurate.
#[derive(Debug, Deserialize)]
pub struct Expressions {
    /// A list of expression names indicating the sequence of evaluation.
    /// The list should not be empty to ensure a valid sequence of operations.
    pub order: Vec<String>,
}

impl Expressions {
    /// Validates the `Expressions` configuration to ensure that the order of expressions is specified.
    ///
    /// Validation checks include verifying that the `order` vector is not empty,
    /// indicating that there is at least one expression to evaluate.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the order of expressions is properly specified (i.e., the list is not empty).
    /// Otherwise, returns a `ValidationError` with a message indicating that the order must not be empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use fatigue::timeseries::Expressions;
    ///
    /// let expressions = Expressions {
    ///     order: vec![String::from("expression1"), String::from("expression2")],
    /// };
    /// assert!(expressions.validate().is_ok());
    ///
    /// let empty_expressions = Expressions { order: vec![] };
    /// assert!(empty_expressions.validate().is_err());
    /// ```
    ///
    /// This method ensures that the application has a clear, non-empty sequence of expressions to evaluate,
    /// maintaining the integrity of the computational process.    
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.order.is_empty() {
            return Err(ValidationError::new("order must not be empty".into()));
        }
        Ok(())
    }
}


#[derive(Debug, Deserialize)]
pub struct SensorFile {
    pub no: usize,
    pub correction: f64,
    pub unit: String,
    pub name: String,
    pub description: String,
}


#[cfg(test)]
mod tests {
    use crate::config::load_config; // Ensure this is correctly imported

    #[test]
    fn test_interpolate_timeseries() {
        let config_path = "tests/config.yaml";
        let config = load_config(config_path).expect("Failed to load config");
        config.timeseries.interpolate().expect("Failed to interpolate timeseries");
    }

    #[test]
    fn test_parse_input() {
        let config_path = "tests/config.yaml";
        let config = load_config(config_path).expect("Failed to load config");

        println!("config: {:#?}", config);

        let results = config.timeseries.parse_input().expect("Failed to parse input");
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
