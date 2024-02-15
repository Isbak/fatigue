// src/lib.rs

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;
#[cfg(any(feature = "cli", feature = "wasm"))]
mod rainflow;

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
