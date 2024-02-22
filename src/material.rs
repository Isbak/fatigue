//! A module for material properties for a structural fatigue analysis application.

use serde::Deserialize;
use anyhow::{Result, anyhow};
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
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(anyhow!("name must not be empty, got {}", self.name));
        }
        if self.youngs_modulus < 0.0 {
            return Err(anyhow!("youngs_modulus must be greater than 0.0, got {}", self.youngs_modulus));
        }
        if self.poissons_ratio < 0.0 {
            return Err(anyhow!("poissons_ratio must be greater than 0.0, got {}", self.poissons_ratio));
        }
        if self.yield_stress < 0.0 {
            return Err(anyhow!("yield_stress must be greater than 0.0, got {}", self.yield_stress));
        }
        if self.ultimate_stress < 0.0 {
            return Err(anyhow!("ultimate_stress must be greater than 0.0, got {}", self.ultimate_stress));
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
    pub fn validate(&self) -> Result<()> {
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
    pub fn validate(&self) -> Result<()> {
        if self.m1 < 0 {
            return Err(anyhow!("m1 must be greater than 0, got {}", self.m1));
        }
        if self.m2 < 0 {
            return Err(anyhow!("m2 must be greater than 0, got {}", self.m2));
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
    pub fn validate(&self) -> Result<()> {
        if self.cycle < 0 {
            return Err(anyhow!("cycle must be greater than 0, got {}", self.cycle));
        }
        if self.stress < 0.0 {
            return Err(anyhow!("stress must be greater than 0.0, got {}", self.stress));
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
    pub fn validate(&self) -> Result<()> {
        if self.max < 0.0 {
            return Err(anyhow!("max must be greater than 0.0, got {}", self.max));
        }
        if self.min < 0.0 {
            return Err(anyhow!("min must be greater than 0.0, got {}", self.min));
        }
        Ok(())
    }
}