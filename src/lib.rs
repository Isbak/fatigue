#[cfg(any(feature = "cli", feature = "wasm"))]
pub mod rainflow;
#[cfg(any(feature = "cli", feature = "wasm"))]
pub mod interpolate;
pub use interpolate::{InterpolationStrategy, Linear, NDInterpolation};
#[cfg(feature = "cli")]
mod app_logic;
#[cfg(feature = "cli")]
pub mod config;
#[cfg(feature = "cli")]
pub mod stress;
#[cfg(feature = "cli")]
pub mod material;
#[cfg(feature = "cli")]
pub mod timeseries;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

// When the "wasm" feature is enabled, use wasm_bindgen to expose functions to the host environment.
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn run_rainflow(stress: &[f64]) -> Vec<f64> {
    let (means, ranges) = rainflow::rainflow(stress);
    // Combine the means and ranges into a single Vec to return.
    // This is just one way to handle the return; you might choose a different method
    // depending on how you want to process the data on the JavaScript side.
    means.into_iter().chain(ranges.into_iter()).collect()
}
