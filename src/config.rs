use serde::Deserialize;
use std::fmt;
use std::fs;
use std::path::Path;
use serde_yaml;
use regex::Regex;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ValidationError(String);

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub solution: Solution,
    pub material: Material,
    pub safety_factor: SafetyFactor,
    pub timeseries: TimeSeries,
    pub parameters: HashMap<String, f64>,
    pub variables: HashMap<String, String>,
    pub expressions: Expressions,
}

impl Config {
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.solution.validate()?;
        self.material.validate()?;
        self.safety_factor.validate()?;
        self.expressions.validate()?;
        self.timeseries.validate()?;
        let re = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();
        
        for (key, value) in &self.variables {
            if !re.is_match(key) {
                return Err(ValidationError(format!("Invalid variable name: {}", key)));
            }
            if value.trim().is_empty() {
                return Err(ValidationError(format!("Variable expression is empty for: {}", key)));
            }
        }

        for (key, value) in &self.parameters {
            if key.trim().is_empty() {
                return Err(ValidationError("parameter key must not be empty".into()));
            }
            if value.is_nan() {
                return Err(ValidationError(format!("parameter value must be a number, got {}", value)));
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct TimeSeries {
    pub sensors: Sensors,
    pub interpolation: Interpolation,
}

#[derive(Debug, Deserialize)]
pub struct Sensors {
    pub path: Option<String>,
    pub parse_function: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ParseConfig {
    pub header: usize,
    pub delimiter: String,
}

impl ParseConfig {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.delimiter.is_empty() {
            return Err(ValidationError("delimiter must not be empty".into()));
        }
        Ok(())
    }
}

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

impl Sensors {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.path.is_none() && self.parse_function.is_none() {
            return Err(ValidationError("path and parse_function cannot both be None".into()));
        }
        Ok(())
    }
}

impl Interpolation {
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.parse_config.validate()?;
        match self.method.as_str() {
            "LINEAR" | "CUBIC" | "NONE" => Ok(()),
            _ => Err(ValidationError(format!("method must be LINEAR, CUBIC, or NONE, got {}", self.method))),
        }?;
        if self.name.trim().is_empty() {
            return Err(ValidationError("name must not be empty".into()));
        }

        if self.path.trim().is_empty() {
            return Err(ValidationError("path must not be empty".into()));
        }
        if self.scale < 0.0 {
            return Err(ValidationError(format!("scale must be greater than 0.0, got {}", self.scale)));
        }
        // Validate the dimension and sensor vector length condition
        if self.sensor.len() != self.dimension {
            return Err(ValidationError(format!("When dimension is {}, the sensor vector must also have a length of 3. Found length: {}", self.dimension, self.sensor.len())));
        }
        if self.sensor.is_empty() {
            return Err(ValidationError("sensor must not be empty".into()));
        }

        if self.points.is_empty() {
            return Err(ValidationError("points must not be empty".into()));
        }
        for point in &self.points {
            if point.file.trim().is_empty() {
                return Err(ValidationError("file must not be empty".into()));
            }
            if point.value.len() != self.dimension {
                return Err(ValidationError(format!("When dimension is {}, the values per point must also have a length of 3. Found length: {}", self.dimension, point.value.len())));
            }
            if point.value.is_empty() {
                return Err(ValidationError("value must not be empty".into()));
            }
        }
        Ok(())
    }
}

impl TimeSeries {
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.sensors.validate()?;
        self.interpolation.validate()?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Expressions {
    pub order: Vec<String>,
}

impl Expressions {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.order.is_empty() {
            return Err(ValidationError("order must not be empty".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Solution {
    pub run_type: String,
    pub format: String,
    pub mode: String,
    pub units: String,
    pub output: String,
    pub stress_criteria: StressCriteria,
    pub mean: Mean,
    pub node: Node,
    pub damage: Damage,
}

impl Solution {
    pub fn validate(&self) -> Result<(), ValidationError> {
        match self.run_type.as_str() {
            "FAT" | "NONE" => Ok(()),
            _ => Err(ValidationError(format!("run_type must be FAT or NONE, got {}", self.run_type))),
        }?;

        match self.format.as_str() {
            "TimeSeries" | "NONE" => Ok(()),
            _ => Err(ValidationError(format!("format must be TimeSeries or NONE, got {}", self.format))),
        }?;

        match self.mode.as_str() {
            "STRESS" | "STRAIN" | "NONE" => Ok(()),
            _ => Err(ValidationError(format!("mode must be STRESS, STRAIN, or NONE, got {}", self.mode))),
        }?;

        match self.units.as_str() {
            "MPA" | "PA" => Ok(()),
            _ => Err(ValidationError(format!("units must be MPA, PA got {}", self.units))),
        }?;

        match self.output.as_str() {
            "ANSYS" | "ASCII" | "JSON" => Ok(()),
            _ => Err(ValidationError(format!("output must be ANSYS or ASCII, got {}", self.output))),
        }?;

        self.stress_criteria.validate()?;
        self.mean.validate()?;
        self.node.validate()?;
        self.damage.validate()?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct StressCriteria {
    pub number: Option<i32>,
    pub method: String,
    pub extreme: String,
}

impl StressCriteria {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.method == "SXXCRIT" {
            match self.number {
                Some(number) if number > 0 => (),
                _ => return Err(ValidationError("number must be greater than 0 for method SXXCRIT".into())),
            }
        }

        match self.method.as_str() {
            "VONMISES" | "MAXIMUM" | "SXXCRIT" | "NONE" => Ok(()),
            _ => Err(ValidationError(format!("method must be VONMISES, MAXIMUM, SXXCRIT, or NONE, got {}", self.method))),
        }?;

        match self.extreme.as_str() {
            "YIELD" | "NONE" => Ok(()),
            _ => Err(ValidationError(format!("extreme must be YIELD or NONE, got {}", self.extreme))),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Mean {
    pub mean: String,
    pub postfix: String,
    pub number: String,
}

impl Mean {
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Validate 'mean' field
        match self.mean.as_str() {
            "GOODMAN" | "LINEAR" | "BI-LINEAR" | "NONE" => Ok(()),
            _ => Err(ValidationError(format!("mean must be GOODMAN, LINEAR, BI-LINEAR, or NONE, got {}", self.mean))),
        }?;

        // Validate 'postfix' field
        match self.postfix.as_str() {
            "FIXEDMEAN" | "NONE" => Ok(()),
            _ => Err(ValidationError(format!("postfix must be FIXEDMEAN or NONE, got {}", self.postfix))),
        }?;

        if !(0.0..=1.0).contains(&self.number.parse::<f64>().unwrap()) {
            return Err(ValidationError(format!("number must be between 0.0 and 1.0, got {}", self.number)));
        };
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Node {
    pub from: i32,
    pub to: i32,
    pub software: String,
    pub path: String,
}

impl Node {
    // Validates the Node struct's software field
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Validate the software field
        match self.software.as_str() {
            "ANSYS" | "LIST" | "NONE" => Ok(()),
            _ => Err(ValidationError(format!("software must be ANSYS or NONE, got {}", self.software))),
        }?;
        // Validate the 'from' field to ensure it's greater than 0
        if self.from <= 0 {
            return Err(ValidationError(format!("'from' must be greater than 0, got {}", self.from)));
        };
        // Assuming similar validation needed for the 'to' field
        if self.to <= 0 {
            return Err(ValidationError(format!("'to' must be greater than 0, got {}", self.to)));
        };
        // Conditionally validate the 'path' field if software is "ANSYS" or "LIST"
        if self.software == "ANSYS" || self.software == "LIST" {
            if self.path.trim().is_empty() {
                return Err(ValidationError(format!("path must not be empty when software is ANSYS or LIST")));
            }
            if !std::path::Path::new(&self.path).exists() {
                return Err(ValidationError(format!("path does not exist: {}", self.path)));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Damage {
    pub error: f64,
    pub dadm: f64,
}

impl Damage {
    pub fn validate(&self) -> Result<(), ValidationError>{
        if !(0.0..=1.0).contains(&self.error) {
            return Err(ValidationError(format!("error must be between 0.0 and 1.0, got {}", self.error)));
        }
        if !(0.0..=1.0).contains(&self.dadm) {
            return Err(ValidationError(format!("dadm must be between 0.0 and 1.0, got {}", self.dadm)));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Material {
    pub name: String,
    pub youngs_modulus: f64,
    pub poissons_ratio: f64,
    pub yield_stress: f64,
    pub ultimate_stress: f64,
    pub hardening_modulus: f64,
    pub hardening_exponent: f64,
    pub fatigue: Fatigue,
}

impl Material {
    // Validates the Material struct's fields based on custom logic
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.name.trim().is_empty() {
            return Err(ValidationError(format!("name must not be empty, got {}", self.name)));
        }
        if self.youngs_modulus < 0.0 {
            return Err(ValidationError(format!("youngs_modulus must be greater than 0.0, got {}", self.youngs_modulus)));
        }
        if self.poissons_ratio < 0.0 {
            return Err(ValidationError(format!("poissons_ratio must be greater than 0.0, got {}", self.poissons_ratio)));
        }
        if self.yield_stress < 0.0 {
            return Err(ValidationError(format!("yield_stress must be greater than 0.0, got {}", self.yield_stress)));
        }
        if self.ultimate_stress < 0.0 {
            return Err(ValidationError(format!("ultimate_stress must be greater than 0.0, got {}", self.ultimate_stress)));
        }
        if self.hardening_modulus < 0.0 {
            return Err(ValidationError(format!("hardening_modulus must be greater than 0.0, got {}", self.hardening_modulus)));
        }
        if self.hardening_exponent < 0.0 {
            return Err(ValidationError(format!("hardening_exponent must be greater than 0.0, got {}", self.hardening_exponent)));
        }
        self.fatigue.validate()?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Fatigue {
    pub slope: Slope,
    pub knee: Knee,
    pub cutoff: Cutoff,
}

impl Fatigue {
    // Validates the Fatigue struct's fields based on custom logic
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.slope.validate()?;
        self.knee.validate()?;
        self.cutoff.validate()?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Slope {
    pub m1: i32,
    pub m2: i32,
}

impl Slope {
    // Validates the Slope struct's fields based on custom logic
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.m1 < 0 {
            return Err(ValidationError(format!("m1 must be greater than 0, got {}", self.m1)));
        }
        if self.m2 < 0 {
            return Err(ValidationError(format!("m2 must be greater than 0, got {}", self.m2)));
        }
        Ok(())
    }    
}

#[derive(Debug, Deserialize)]
pub struct Knee {
    pub cycle: i64,
    pub stress: f64,
}

impl Knee {
    // Validates the Knee struct's fields based on custom logic
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.cycle < 0 {
            return Err(ValidationError(format!("cycle must be greater than 0, got {}", self.cycle)));
        }
        if self.stress < 0.0 {
            return Err(ValidationError(format!("stress must be greater than 0.0, got {}", self.stress)));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Cutoff {
    pub max: f64,
    pub min: f64,
}

impl Cutoff {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.max < 0.0 {
            return Err(ValidationError(format!("max must be greater than 0.0, got {}", self.max)));
        }
        if self.min < 0.0 {
            return Err(ValidationError(format!("min must be greater than 0.0, got {}", self.min)));
        }
        Ok(())
    }
}


#[derive(Debug, Deserialize)]
pub struct SafetyFactor {
    pub gmre: f64,
    pub gmrm: f64,
    pub gmfat: f64,
}

impl SafetyFactor {
    // Validates the SafetyFactor struct's fields based on custom logic
    pub fn validate(&self) -> Result<(), ValidationError> {
        if !(1.0..=2.0).contains(&self.gmre) {
            return Err(ValidationError(format!("gmre must be between 1.0 and 2.0, got {}", self.gmre)));
        }
        if !(1.0..=2.0).contains(&self.gmrm) {
            return Err(ValidationError(format!("gmrm must be between 1.0 and 2.0, got {}", self.gmrm)));
        }
        if !(1.0..=2.0).contains(&self.gmfat) {
            return Err(ValidationError(format!("gmfat must be between 1.0 and 2.0, got {}", self.gmfat)));
        }
        Ok(())
    }
}

// Add the load_config function to read from YAML
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
