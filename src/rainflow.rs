//! A module for rainflow counting algorithm
use std::collections::VecDeque;

/// Rainflow counting algorithm
#[cfg(any(feature = "cli", feature = "wasm"))]
pub fn rainflow(stress: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut reversals = VecDeque::new();
    let mut outmean = Vec::new();
    let mut outrange = Vec::new();

    // Identify reversals in the stress history
    for i in 1..stress.len() {
        if stress[i] != stress[i - 1] {
            reversals.push_back(stress[i]);
        }
    }

    // Rainflow counting algorithm
    while reversals.len() >= 3 {
        let z = reversals[0];
        let y = reversals[1];
        let x = reversals[2];

        let r_x = (x - z).abs();
        let r_y = (y - z).abs();

        if r_x < r_y {
            // Count Y as 1 cycle
            let mean = (y + z) / 2.0;
            let range = r_y;
            outmean.push(mean);
            outrange.push(range);

            // Discard both points of Y
            reversals.pop_front();
            reversals.pop_front();
        } else {
            // Check if Y includes Z
            if (z < y && y < x) || (z > y && y > x) {
                // Count Y as 1/2 cycle
                let mean = (y + z) / 2.0;
                let range = r_y;
                outmean.push(mean);
                outrange.push(range / 2.0);

                // Discard the first reversal of Y
                reversals.pop_front();

                // Set Z to the second reversal of Y
                reversals[0] = x;
            } else {
                // Not enough reversals to form a cycle, read more reversals
                break;
            }
        }
    }

    // Handle the remaining reversals as half cycles
    while let Some(rev) = reversals.pop_front() {
        if let Some(next_rev) = reversals.front() {
            let mean = (rev + next_rev) / 2.0;
            let range = (next_rev - rev).abs();
            outmean.push(mean);
            outrange.push(range / 2.0);
        }
    }

    (outmean, outrange)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rainflow(){
        let stress_sequence = vec![-2.0, 1.0, -3.0, 5.0, -1.0, 3.0, -4.0, 4.0, -3.0, 1.0, -2.0, 3.0, 6.0];
        let (means, ranges) = rainflow(&stress_sequence);
    
        // Output the results
        for (mean, range) in means.iter().zip(ranges.iter()) {
            println!("{:.4}, {:.4}", mean, range);
        }
    }
}