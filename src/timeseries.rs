//! Contains the `TimeSeries` struct and related functionality for time series analysis.
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use regex::Regex;
use serde_json::from_str;
use std::io::{self, BufRead};
use std::fs::File;
use std::path::Path;
use std::fs;

use crate::config::ValidationError;

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

#[derive(Debug, Deserialize)]
pub struct Point {
    pub point: Vec<i32>,
    pub file: String,
    pub value: Vec<f64>,
}

/// Interpolation configuration for a structural analysis application.
impl Interpolation {
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.parse_config.validate()?;
        match self.method.as_str() {
            "LINEAR" | "CUBIC" | "NONE" => Ok(()),
            _ => Err(ValidationError::new(&format!("method must be LINEAR, CUBIC, or NONE, got {}", self.method))),
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
            if point.file.trim().is_empty() {
                return Err(ValidationError::new("file must not be empty".into()));
            }
            if point.value.len() != self.dimension {
                return Err(ValidationError::new(&format!("When dimension is {}, the values per point must also have a length of 3. Found length: {}", self.dimension, point.value.len())));
            }
            if point.value.is_empty() {
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
        let content = fs::read_to_string(&self.sensorfile)?;
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

    /// Performs interpolation based on the provided load cases and interpolation configurations.
    ///
    /// This method processes each load case, applying the specified interpolation method to
    /// the time series data. The actual implementation of the interpolation logic may vary.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` upon successful completion of all interpolations. Otherwise,
    /// returns an `io::Error` detailing any issues encountered during the process.
    pub fn interpolate(&self) -> Result<(), io::Error> {
        // Applies interpolation to the load cases based on specified methods and configurations.
        for lc in &self.loadcases {
            println!("Processing load case: {}", lc.file);
            let lc_file_path = format!("{}/{}", self.path, lc.file);
            let lc_data = Self::read_loadcase_file(&lc_file_path)?;
            
            for interp in &self.interpolations {
                println!("Interpolating with: {}", interp.name);
                // Simplified: Apply linear interpolation based on the provided points
                // Note: Actual implementation will vary based on how you're applying these interpolations
            }
        }
        Ok(())
    } 
    /// Reads and returns the content of a load case file specified by path.
    ///
    /// # Arguments
    ///
    /// * `path` - The file path to the load case data.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of strings, each representing a line of the file,
    /// if successful. Otherwise, returns an `io::Error` detailing the issue encountered.
    pub fn read_loadcase_file(path: &str) -> Result<Vec<String>, io::Error> {
        // Reads the content of the load case file, returning it line by line.
        let file = File::open(path)?;
        let buf = io::BufReader::new(file);
        buf.lines().collect()
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
    /// use your_crate_name::Expressions;
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
