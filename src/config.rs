//! A module for validating and managing configurations for a structural analysis application.

use serde::Deserialize;
use std::fmt;
use std::fs;
use std::path::Path;
use serde_yaml;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use serde_json::from_str;
use std::io::{self, BufRead};
use std::fs::File;

/// Represents an error that can occur during validation of configuration data.
#[derive(Debug)]
pub struct ValidationError{
    message: String,
}

impl ValidationError {
    /// Creates a new `ValidationError` with a given message.
    ///
    /// # Arguments
    ///
    /// * `message` - A description of the error.    
    fn new(message: &str) -> ValidationError {
        ValidationError {
            message: message.to_owned(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Represents the configuration for a structural analysis application.
#[derive(Debug, Deserialize)]
pub struct Config {
    pub solution: Solution,
    pub material: Material,
    pub safety_factor: SafetyFactor,
    pub timeseries: TimeSeries,
}

impl Config {
    /// Validates the entire configuration.
    ///
    /// This method checks the validity of each component of the configuration
    /// and ensures all required conditions are met.
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.solution.validate()?;
        self.material.validate()?;
        self.safety_factor.validate()?;
        self.timeseries.validate()?;           
        self.validate_sensor_against_sensorfile()?;
        Ok(())
    }

    /// Validates that all sensors specified in the `TimeSeries` configuration
    /// exist within the sensor file.
    fn validate_sensor_against_sensorfile(&self) -> Result<(), ValidationError> {
        // Attempt to read the sensorfile and handle potential errors gracefully
        let sen = self.timeseries.read_sensorfile()
            .map_err(|e| ValidationError::new(&format!("Failed to read sensor file: {}", e)))?;
        
        for interp in self.timeseries.interpolations.iter() {
            for sensor in interp.sensor.iter() {
                // Direct comparison without converting to String
                if !sen.iter().any(|s| s.name == *sensor) {
                    return Err(ValidationError::new(&format!("Sensor '{}' not found in sensorfile", sensor)));
                }
            }
        }
        Ok(())
    }
}


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

#[derive(Debug, Deserialize)]
pub struct SensorFile {
    pub no: usize,
    pub correction: f64,
    pub unit: String,
    pub name: String,
    pub description: String,
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

/// Represents the solution configuration for a structural analysis session.
///
/// This struct encapsulates the settings and criteria defining how a structural analysis
/// is to be run, including the type of analysis, the mode of operation, desired output format,
/// and various criteria such as stress evaluation, mean stress correction, node specification,
/// and damage metrics.
#[derive(Debug, Deserialize)]
pub struct Solution {
    /// Specifies the type of run. Valid values are "FAT" for fatigue analysis and "NONE" for no analysis.
    pub run_type: String,
    /// Defines the mode of operation. Valid modes are "STRESS" for stress analysis and "NONE" for no specific mode.
    pub mode: String,
    /// The desired output format. Currently, "JSON" is supported as a valid output.
    pub output: String,
    /// Criteria for evaluating stress within the analysis.
    pub stress_criteria: StressCriteria,
    /// Parameters for mean stress correction.
    pub mean: Mean,
    /// Node range specification for the analysis.
    pub node: Node,
    /// Damage metrics for the analysis.
    pub damage: Damage,
}

impl Solution {
    /// Validates the `Solution` configuration to ensure all specified settings are valid and
    /// consistent with the application's requirements.
    ///
    /// This method checks each field to verify that:
    /// - `run_type` is either "FAT" or "NONE".
    /// - `mode` is either "STRESS" or "NONE".
    /// - `output` is "JSON", indicating the output format.
    /// Additionally, it invokes validation on nested structs (`stress_criteria`, `mean`, `node`, `damage`)
    /// to ensure their configurations are also valid.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the solution configuration and all related criteria are valid.
    /// If any configuration is invalid, it returns a `ValidationError` with a detailed explanation.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate_name::{Solution, StressCriteria, Mean, Node, Damage};
    ///
    /// let solution = Solution {
    ///     run_type: String::from("FAT"),
    ///     mode: String::from("STRESS"),
    ///     output: String::from("JSON"),
    ///     stress_criteria: StressCriteria::default(), // Assume default implementations
    ///     mean: Mean::default(),
    ///     node: Node::default(),
    ///     damage: Damage::default(),
    /// };
    ///
    /// assert!(solution.validate().is_ok());
    /// ```    
    pub fn validate(&self) -> Result<(), ValidationError> {
        match self.run_type.as_str() {
            "FAT" | "NONE" => Ok(()),
            _ => Err(ValidationError::new(&format!("run_type must be FAT or NONE, got {}", self.run_type))),
        }?;

        match self.mode.as_str() {
            "STRESS" | "NONE" => Ok(()),
            _ => Err(ValidationError::new(&format!("mode must be STRESS, STRAIN, or NONE, got {}", self.mode))),
        }?;
        match self.output.as_str() {
            "JSON" => Ok(()),
            _ => Err(ValidationError::new(&format!("output must be ANSYS or ASCII, got {}", self.output))),
        }?;

        self.stress_criteria.validate()?;
        self.mean.validate()?;
        self.node.validate()?;
        self.damage.validate()?;
        Ok(())
    }
}

/// Represents the criteria for evaluating stress in a structural analysis application.
///
/// This struct is used to define how stress should be assessed, including the choice of method and,
/// optionally, a numerical parameter that may be required by certain methods.
#[derive(Debug, Deserialize)]
pub struct StressCriteria {
    /// An optional numerical parameter required by some stress evaluation methods.
    /// For example, when using the `SXXCRIT` method, this number must be specified and greater than 0.
    pub number: Option<i32>,
    /// The method used to evaluate stress. Valid methods include "VONMISES", "MAXIMUM", "SXXCRIT", and "NONE".
    /// Some methods, like "SXXCRIT", may require an additional numerical parameter specified in `number`.
    pub method: String,
}

impl StressCriteria {
    /// Validates the `StressCriteria` to ensure the method and its associated parameters are correctly defined.
    ///
    /// Validation checks include ensuring that the method is one of the accepted values and,
    /// for methods like "SXXCRIT", that the `number` is provided and is greater than 0.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the stress criteria are valid. If not, returns a `ValidationError`
    /// detailing the nature of the validation failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate_name::StressCriteria;
    ///
    /// let criteria_vonmises = StressCriteria {
    ///     number: None,
    ///     method: String::from("VONMISES"),
    /// };
    /// assert!(criteria_vonmises.validate().is_ok());
    ///
    /// let criteria_sxxcrit_invalid = StressCriteria {
    ///     number: None, // Missing required number for SXXCRIT
    ///     method: String::from("SXXCRIT"),
    /// };
    /// assert!(criteria_sxxcrit_invalid.validate().is_err());
    ///
    /// let criteria_sxxcrit_valid = StressCriteria {
    ///     number: Some(10),
    ///     method: String::from("SXXCRIT"),
    /// };
    /// assert!(criteria_sxxcrit_valid.validate().is_ok());
    /// ```    
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.method == "SXXCRIT" {
            match self.number {
                Some(number) if number > 0 => (),
                _ => return Err(ValidationError::new("number must be greater than 0 for method SXXCRIT".into())),
            }
        };
        match self.method.as_str() {
            "VONMISES" | "MAXIMUM" | "SXXCRIT" | "NONE" => Ok(()),
            _ => Err(ValidationError::new(&format!("method must be VONMISES, MAXIMUM, SXXCRIT, or NONE, got {}", self.method))),
        }?;
        Ok(())
    }
}

/// Represents the mean stress correction factors in a structural analysis context.
///
/// This struct holds information about the mean stress correction approach used,
/// including the method, any postfix applied to the calculations, and a numerical factor.
#[derive(Debug, Deserialize)]
pub struct Mean {
    /// The mean stress correction method. Valid values are "GOODMAN", "LINEAR", "BI-LINEAR", or "NONE".
    pub mean: String,
    /// An additional postfix applied to the mean stress correction. Valid values are "FIXEDMEAN" or "NONE".
    pub postfix: String,
    /// A numerical factor associated with the mean stress correction, expected to be between 0.0 and 1.0.
    pub number: String,
}

impl Mean {
    /// Validates the `Mean` struct's fields to ensure they conform to expected values and ranges.
    ///
    /// This method checks that `mean` and `postfix` are within their respective valid sets of values,
    /// and that `number` is a valid floating-point number between 0.0 and 1.0.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all fields are valid. Otherwise, returns a `ValidationError`
    /// with a detailed message about the specific validation failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate_name::Mean;
    ///
    /// let mean_correction = Mean {
    ///     mean: String::from("GOODMAN"),
    ///     postfix: String::from("FIXEDMEAN"),
    ///     number: String::from("0.5")
    /// };
    /// assert!(mean_correction.validate().is_ok());
    ///
    /// let invalid_mean_correction = Mean {
    ///     mean: String::from("INVALID"),
    ///     postfix: String::from("FIXEDMEAN"),
    ///     number: String::from("1.5")
    /// };
    /// assert!(invalid_mean_correction.validate().is_err());
    /// ```    
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Validate 'mean' field
        match self.mean.as_str() {
            "GOODMAN" | "LINEAR" | "BI-LINEAR" | "NONE" => Ok(()),
            _ => Err(ValidationError::new(&format!("mean must be GOODMAN, LINEAR, BI-LINEAR, or NONE, got {}", self.mean))),
        }?;

        // Validate 'postfix' field
        match self.postfix.as_str() {
            "FIXEDMEAN" | "NONE" => Ok(()),
            _ => Err(ValidationError::new(&format!("postfix must be FIXEDMEAN or NONE, got {}", self.postfix))),
        }?;

        if !(0.0..=1.0).contains(&self.number.parse::<f64>().unwrap()) {
            return Err(ValidationError::new(&format!("number must be between 0.0 and 1.0, got {}", self.number)));
        };
        Ok(())
    }
}

/// Represents a range of nodes within a structural analysis model.
///
/// This struct is used to define a sequence of nodes, typically for specifying a region
/// or segment of a structure where analysis or operations are to be focused. The range
/// is inclusive, represented by `from` and `to` fields.
#[derive(Debug, Deserialize)]
pub struct Node {
    /// The starting node of the range. Must be greater than 0.
    pub from: i32,
    /// The ending node of the range. Must be greater than 0 and should be greater than or equal to `from`.
    pub to: i32,
    /// The file path associated with the nodes. This could be a path to a file containing
    /// additional data or configuration related to the specified node range.    
    pub path: String,
}

impl Node {
    /// Validates the `Node` struct's fields to ensure they are within acceptable ranges and conditions.
    ///
    /// Specifically, it checks that both `from` and `to` fields are greater than 0 to ensure
    /// valid node indices. Additionally, although not explicitly checked here, `to` should ideally
    /// be greater than or equal to `from` to represent a valid range.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if both `from` and `to` fields are valid. Otherwise, returns a `ValidationError`
    /// with a detailed message about which field is invalid and why.
    ///
    /// # Examples
    ///
    /// ```
    /// use your_crate_name::Node;
    ///
    /// let node_range = Node { from: 1, to: 10, path: String::from("path/to/data") };
    /// assert!(node_range.validate().is_ok());
    ///
    /// let invalid_node_range = Node { from: 0, to: 5, path: String::from("path/to/data") };
    /// assert!(invalid_node_range.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Validate the 'from' field to ensure it's greater than 0
        if self.from <= 0 {
            return Err(ValidationError::new(&format!("'from' must be greater than 0, got {}", self.from)));
        };
        // Assuming similar validation needed for the 'to' field
        if self.to <= 0 {
            return Err(ValidationError::new(&format!("'to' must be greater than 0, got {}", self.to)));
        };
        Ok(())
    }
}

/// Represents damage metrics associated with a material under analysis.
///
/// Contains error and damage accumulation (dadm) factors, both of which should be
/// within the range [0.0, 1.0] to represent a percentage of the total possible damage.
#[derive(Debug, Deserialize)]
pub struct Damage {
    /// Error factor in damage calculation. Must be between 0.0 and 1.0.
    pub error: f64,
    /// Damage accumulation factor. Must be between 0.0 and 1.0.
    pub dadm: f64,
}

impl Damage {
    /// Validates the `Damage` struct to ensure `error` and `dadm` are within acceptable ranges.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if both `error` and `dadm` are within the range [0.0, 1.0]. Otherwise,
    /// it returns a `ValidationError` detailing which field is out of the expected range.    
    pub fn validate(&self) -> Result<(), ValidationError>{
        if !(0.0..=1.0).contains(&self.error) {
            return Err(ValidationError::new(&format!("error must be between 0.0 and 1.0, got {}", self.error)));
        }
        if !(0.0..=1.0).contains(&self.dadm) {
            return Err(ValidationError::new(&format!("dadm must be between 0.0 and 1.0, got {}", self.dadm)));
        }
        Ok(())
    }
}

/// Represents material properties used in structural analysis.
///
/// Includes material's mechanical properties such as Young's modulus, Poisson's ratio,
/// yield stress, and ultimate stress, along with a `Fatigue` struct representing the
/// material's fatigue characteristics.
#[derive(Debug, Deserialize)]
pub struct Material {
    /// Name of the material.
    pub name: String,
    /// Young's modulus of the material in appropriate units (e.g., MPa, psi).
    pub youngs_modulus: f64,
    /// Poisson's ratio of the material.
    pub poissons_ratio: f64,
    /// Yield stress of the material in appropriate units.
    pub yield_stress: f64,
    /// Ultimate stress of the material in appropriate units.
    pub ultimate_stress: f64,
    /// Fatigue characteristics of the material.
    pub fatigue: Fatigue,
}

impl Material {
    /// Validates the `Material` struct to ensure all mechanical properties are defined correctly
    /// and the `Fatigue` properties are valid.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all properties are valid and within their expected ranges.
    /// Otherwise, it returns a `ValidationError` detailing the issue.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.name.trim().is_empty() {
            return Err(ValidationError::new(&format!("name must not be empty, got {}", self.name)));
        }
        if self.youngs_modulus < 0.0 {
            return Err(ValidationError::new(&format!("youngs_modulus must be greater than 0.0, got {}", self.youngs_modulus)));
        }
        if self.poissons_ratio < 0.0 {
            return Err(ValidationError::new(&format!("poissons_ratio must be greater than 0.0, got {}", self.poissons_ratio)));
        }
        if self.yield_stress < 0.0 {
            return Err(ValidationError::new(&format!("yield_stress must be greater than 0.0, got {}", self.yield_stress)));
        }
        if self.ultimate_stress < 0.0 {
            return Err(ValidationError::new(&format!("ultimate_stress must be greater than 0.0, got {}", self.ultimate_stress)));
        }
        self.fatigue.validate()?;
        Ok(())
    }
}

/// Represents the fatigue parameters of a material in a structural analysis application.
///
/// Includes parameters for the slope of the S-N curve, the knee point of the curve, and the cutoff limits.
#[derive(Debug, Deserialize)]
pub struct Fatigue {
    /// The slope parameters of the S-N curve.
    pub slope: Slope,
    /// The knee point of the S-N curve, indicating a change in slope.
    pub knee: Knee,
    /// The cutoff limits for the S-N curve, defining the maximum and minimum stress values.
    pub cutoff: Cutoff,
}

impl Fatigue {
    /// Validates the `Fatigue` struct's fields to ensure they meet the application's requirements.
    ///
    /// Each component (slope, knee, cutoff) is validated individually.
    ///
    /// # Returns
    ///
    /// This method returns `Ok(())` if all components are valid. Otherwise, it returns a `ValidationError`
    /// with a detailed message about the validation failure.
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.slope.validate()?;
        self.knee.validate()?;
        self.cutoff.validate()?;
        Ok(())
    }
}

/// Represents the slope parameters of the S-N curve for fatigue analysis.
#[derive(Debug, Deserialize)]
pub struct Slope {
    /// The slope of the S-N curve before the knee point.
    pub m1: i32,
    /// The slope of the S-N curve after the knee point.
    pub m2: i32,
}

impl Slope {
    /// Validates the `Slope` struct's fields based on custom logic.
    ///
    /// Ensures that both `m1` and `m2` are greater than 0.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if both `m1` and `m2` are valid. Otherwise, returns a `ValidationError`
    /// with a detailed message about the validation failure.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.m1 < 0 {
            return Err(ValidationError::new(&format!("m1 must be greater than 0, got {}", self.m1)));
        }
        if self.m2 < 0 {
            return Err(ValidationError::new(&format!("m2 must be greater than 0, got {}", self.m2)));
        }
        Ok(())
    }    
}

/// Represents the knee point of the S-N curve for fatigue analysis.
#[derive(Debug, Deserialize)]
pub struct Knee {
    /// The cycle count at the knee point of the S-N curve.
    pub cycle: i64,
    /// The stress value at the knee point of the S-N curve.
    pub stress: f64,
}

impl Knee {
    /// Validates the `Knee` struct's fields based on custom logic.
    ///
    /// Ensures that `cycle` is greater than 0 and `stress` is greater than 0.0.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if both `cycle` and `stress` are valid. Otherwise, returns a `ValidationError`
    /// with a detailed message about the validation failure.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.cycle < 0 {
            return Err(ValidationError::new(&format!("cycle must be greater than 0, got {}", self.cycle)));
        }
        if self.stress < 0.0 {
            return Err(ValidationError::new(&format!("stress must be greater than 0.0, got {}", self.stress)));
        }
        Ok(())
    }
}

/// Represents the cutoff limits of the S-N curve for fatigue analysis.
#[derive(Debug, Deserialize)]
pub struct Cutoff {
    /// The maximum stress value considered in the fatigue analysis.
    pub max: f64,
    /// The minimum stress value considered in the fatigue analysis.
    pub min: f64,
}

impl Cutoff {
    /// Validates the `Cutoff` struct's fields based on custom logic.
    ///
    /// Ensures that both `max` and `min` are greater than 0.0.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if both `max` and `min` are valid. Otherwise, returns a `ValidationError`
    /// with a detailed message about the validation failure.
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.max < 0.0 {
            return Err(ValidationError::new(&format!("max must be greater than 0.0, got {}", self.max)));
        }
        if self.min < 0.0 {
            return Err(ValidationError::new(&format!("min must be greater than 0.0, got {}", self.min)));
        }
        Ok(())
    }
}

/// Represents the safety factors used in a structural analysis application.
///
/// This struct holds the safety factors for material resistance (`gmre`), material resistance margin (`gmrm`),
/// and fatigue (`gmfat`). These factors are used to ensure that the design complies with the required safety standards.
#[derive(Debug, Deserialize)]
pub struct SafetyFactor {
    /// Safety factor for material resistance.
    /// This value should be between 1.0 and 2.0, inclusive.
    pub gmre: f64,
    /// Safety factor for material resistance margin.
    /// This value should be between 1.0 and 2.0, inclusive.
    pub gmrm: f64,
    /// Safety factor for fatigue.
    /// This value should be between 1.0 and 2.0, inclusive.
    pub gmfat: f64,
}

impl SafetyFactor {
    /// Validates the `SafetyFactor`'s fields to ensure they fall within the acceptable range.
    ///
    /// Each safety factor (`gmre`, `gmrm`, `gmfat`) must be between 1.0 and 2.0, inclusive.
    /// This method checks each field and returns an error if any value does not meet the requirement.
    ///
    /// # Returns
    ///
    /// This method returns `Ok(())` if all safety factors are within the acceptable range.
    /// Otherwise, it returns a `ValidationError` with a message indicating which safety factor
    /// is out of range and what its value was.
    ///
    /// # Examples
    ///
    /// ```
    /// use config::SafetyFactor;
    ///
    /// let sf = SafetyFactor { gmre: 1.5, gmrm: 1.2, gmfat: 1.3 };
    /// assert!(sf.validate().is_ok());
    ///
    /// let sf_invalid = SafetyFactor { gmre: 2.1, gmrm: 1.0, gmfat: 0.9 };
    /// assert!(sf_invalid.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), ValidationError> {
        if !(1.0..=2.0).contains(&self.gmre) {
            return Err(ValidationError::new(&format!("gmre must be between 1.0 and 2.0, got {}", self.gmre)));
        }
        if !(1.0..=2.0).contains(&self.gmrm) {
            return Err(ValidationError::new(&format!("gmrm must be between 1.0 and 2.0, got {}", self.gmrm)));
        }
        if !(1.0..=2.0).contains(&self.gmfat) {
            return Err(ValidationError::new(&format!("gmfat must be between 1.0 and 2.0, got {}", self.gmfat)));
        }
        Ok(())
    }
}

/// Additional struct and impl blocks would follow the same pattern:
/// - Briefly describe the purpose of the struct.
/// - Document each public field if necessary.
/// - For each method, describe what it does, its parameters, and its return value.
/// - Use examples in the documentation where appropriate.

/// Loads the configuration from a YAML file.
///
/// # Arguments
///
/// * `config_path` - A path reference to the configuration file.
///
/// # Returns
///
/// This function returns a `Result` containing either the loaded `Config` or an error.
///
/// # Errors
///
/// This function will return an error if reading or parsing the configuration file fails.

pub fn load_config<P: AsRef<Path>>(config_path: P) -> Result<Config, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(config_path)?;
    let config: Config = serde_yaml::from_str(&content)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        let config_path = "tests/config.yaml"; // Adjust the path as needed
        let config = load_config(config_path).expect("Failed to load config");
        assert!(config.validate().is_ok(), "Expected Ok(()) but got Err with {:?}", config.validate());
        // Additional tests as needed
    }
}
