use serde::Deserialize;
use std::fmt;

#[derive(Debug)]
pub struct ValidationError(String);

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    solution: Solution,
    material: Material,
    safety_factor: SafetyFactor,
}

impl Config {
    pub fn validate(&self) -> Result<(), ValidationError> {
        self.solution.validate()?;
        self.material.validate()?;
        self.safety_factor.validate()?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct Solution {
    run_type: String,
    format: String,
    mode: String,
    units: String,
    output: String,
    stress_criteria: StressCriteria,
    mean: Mean,
    node: Node,
    damage: Damage,
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
            "ANSYS" | "ASCII" => Ok(()),
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
    number: Option<i32>,
    method: String,
    extreme: String,
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
    mean: String,
    postfix: String,
    number: String,
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
    from: i32,
    to: i32,
    software: String,
    path: String,
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
    error: f64,
    dadm: f64,
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
    name: String,
    youngs_modulus: f64,
    poissons_ratio: f64,
    yield_stress: f64,
    ultimate_stress: f64,
    hardening_modulus: f64,
    hardening_exponent: f64,
    fatigue: Fatigue,
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
    slope: Slope,
    knee: Knee,
    cutoff: Cutoff,
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
    m1: i32,
    m2: i32,
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
    cycle: i64,
    stress: f64,
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
    max: f64,
    min: f64,
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
    gmre: f64,
    gmrm: f64,
    gmfat: f64,
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

pub fn load_config(config_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let config_str = std::fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_str)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        let config_path = "tests/config_test.toml"; // Adjust the path as needed
        let config = load_config(config_path).unwrap();
        assert_eq!(config.solution.run_type, "FAT");
        assert_eq!(config.solution.format, "TimeSeries");
        // Continue asserting for other fields as needed
    }
}
