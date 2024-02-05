use std::collections::VecDeque;

pub fn rainflow(stress: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut cycles = VecDeque::new();
    let mut outmean = Vec::new();
    let mut outrange = Vec::new();
    let mut data = Vec::new();

    for i in 1..stress.len() {
        let dy = stress[i] - stress[i - 1];
        data.push(dy);
    }

    for i in 0..data.len() {
        let start = i;
        let mut end = i;
        let mut peak = data[i];
        let mut valley = data[i];

        for j in i + 1..data.len() {
            if (peak - valley).abs() >= data[j] - valley {
                break;
            }
            end = j;
            if data[j] > peak {
                peak = data[j];
            } else if data[j] < valley {
                valley = data[j];
            }
        }

        if start != end {
            let mean = (peak + valley) / 2.0;
            let range_ = (peak - valley).abs();

            let mut k = end;
            for l in end + 1..data.len() {
                if (data[l] - valley).abs() >= range_ {
                    break;
                }
                k = l;
                if data[l] > peak {
                    peak = data[l];
                } else if data[l] < valley {
                    valley = data[l];
                }
            }

            end = k;
            if end > start {
                outmean.push(mean);
                outrange.push((peak - valley).abs());
                cycles.push_back((start, end));
            }
        }
    }

    (outmean, outrange)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_rainflow_case_1() {
        // Define the time series as in TEST_CASE_1
        let series = vec![-2.0, 1.0, -3.0, 5.0, -1.0, 3.0, -4.0, 4.0, -2.0];
        let (mean, range) = rainflow(&series);

        // Expected results based on TEST_CASE_1, adjusted for the output format of your Rust function
        let expected_mean = vec![-0.5, -1.0, 1.0, 1.0, 0.5, 0.0, 1.0]; 
        let expected_range = vec![3.0, 4.0, 4.0, 8.0, 9.0, 8.0, 6.0]; 

        // Asserting that the lengths match
        assert_eq!(mean.len(), expected_mean.len());
        assert_eq!(range.len(), expected_range.len());

        // Asserting each value with an approximation due to floating-point arithmetic
        for (m, &expected_m) in mean.iter().zip(expected_mean.iter()) {
            assert_relative_eq!(m, &expected_m, epsilon = 1e-6);
        }
        for (r, &expected_r) in range.iter().zip(expected_range.iter()) {
            assert_relative_eq!(r, &expected_r, epsilon = 1e-6);
        }
    }
}

