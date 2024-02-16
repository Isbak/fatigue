//! A module for validating and managing configurations for a structural analysis application.

use serde::Deserialize;
use std::fmt;
use std::fs;
use std::path::Path;
use serde_yaml;

use crate::material::Material; 
use crate::timeseries::TimeSeries;

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
    pub fn new(message: &str) -> ValidationError {
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
    /// use fatigue::config::StressCriteria;
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
    /// use fatigue::config::Mean;
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
    /// 
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
    /// use fatigue::config::Node;
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
    /// use fatigue::config::SafetyFactor;
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
