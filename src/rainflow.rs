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

