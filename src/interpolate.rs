use std::collections::HashMap;
use nalgebra::{DMatrix, DVector};
use crate::timeseries::Point;
use rayon::prelude::*;


// Define a trait for our interpolation strategies
pub trait InterpolationStrategy {
    // Corrected to include only two parameters: points and target
    fn interpolate(&self, points: &HashMap<Point, f64>, target: &Vec<Vec<f64>>) -> Result<Vec<f64>, String>;
}

// Implement nearest-neighbor interpolation
pub struct NearestNeighbor;

impl InterpolationStrategy for NearestNeighbor {
    fn interpolate(&self, points: &HashMap<Point, f64>, target: &Vec<Vec<f64>>) -> Result<Vec<f64>, String> {
        if points.is_empty() {
            return Err("No points available for interpolation.".to_string());
        }

        // Convert HashMap into a Vec once to avoid repetitive hashing operations
        let points_vec: Vec<(&Point, &f64)> = points.iter().collect();

        let results: Result<Vec<_>, _> = target.par_iter()
            .map(|target_vec| {
                points_vec.iter()
                    .map(|(point, &value)| {
                        let distance = point.coordinates.iter()
                            .zip(target_vec)
                            .map(|(a, b)| (a - b).powi(2))
                            .sum::<f64>()
                            .sqrt();
                        (distance, value)
                    })
                    .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(_, value)| value)  // Dereference value to return the f64 directly
                    .ok_or_else(|| "Error finding nearest neighbor.".to_string())
            })
            .collect();

        results
    }
}


// Implement linear interpolation
pub struct Linear;

impl InterpolationStrategy for Linear {
    fn interpolate(&self, points: &HashMap<Point, f64>, target: &Vec<Vec<f64>>) -> Result<Vec<f64>, String> {
        if points.len() < 2 {
            return Err("Not enough points for interpolation".to_string());
        }

        // Convert points to a format suitable for SVD and regression
        let points_vec: Vec<(Vec<f64>, f64)> = points.iter()
            .map(|(point, &value)| (point.coordinates.clone(), value))
            .collect();

        let coefficients = multivariate_linear_regression_svd(&points_vec)
            .map_err(|e| format!("Failed to perform linear regression: {}", e))?;

        // Use parallel iterator on targets for prediction
        let predictions: Result<Vec<f64>, _> = target.par_iter()
            .map(|t| {
                let predicted_value = coefficients[0] + t.iter().enumerate().map(|(i, coord)| {
                    if i + 1 < coefficients.len() {
                        coefficients[i + 1] * coord
                    } else {
                        0.0
                    }
                }).sum::<f64>();
                Ok(predicted_value)
            })
            .collect();

        predictions
    }
}

fn multivariate_linear_regression_svd(points: &[(Vec<f64>, f64)]) -> Result<Vec<f64>, String> {
    if points.is_empty() {
        return Err("No points provided for linear regression.".to_string());
    }

    // Parallel processing to prepare x_data and y_data
    let (x_data, y_data): (Vec<_>, Vec<f64>) = points.par_iter()
        .map(|(features, target)| {
            let mut row = vec![1.0]; // Intercept term
            row.extend(features.iter().cloned());
            (row, *target)
        })
        .unzip();

    // Flatten x_data for DMatrix
    let x_data: Vec<f64> = x_data.into_iter().flatten().collect();
    let rows = points.len();
    let cols = points[0].0.len() + 1; // Number of features + 1 for the intercept

    // Convert the data into nalgebra's DMatrix and DVector
    let x = DMatrix::from_row_slice(rows, cols, &x_data);
    let y = DVector::from_vec(y_data);

    // Perform SVD
    let svd = x.svd(true, true);
    match svd.solve(&y, 1e-12) {
        Ok(solution) => Ok(solution.iter().cloned().collect()),
        Err(e) => Err(format!("Failed to solve the linear system using SVD: {}", e)),
    }
}


// Enum to encapsulate different strategies
pub enum InterpolationStrategyEnum {
    Linear(Linear),
    NearestNeighbor(NearestNeighbor),
}

impl InterpolationStrategyEnum {
    pub fn interpolate(&self, points: &HashMap<Point, f64>, target: &Vec<Vec<f64>>) -> Result<Vec<f64>, String> {
        match self {
            InterpolationStrategyEnum::Linear(strategy) => strategy.interpolate(&points, &target),
            InterpolationStrategyEnum::NearestNeighbor(strategy) => strategy.interpolate(&points,&target),
        }
    }
}

// NDInterpolation struct utilizing the enum for static dispatch
pub struct NDInterpolation<'a> {
    points: HashMap<Point, f64>,
    strategy: &'a InterpolationStrategyEnum,
}

impl<'a> NDInterpolation<'a> {
    pub fn new(strategy: &'a InterpolationStrategyEnum) -> Self {
        NDInterpolation { 
            points: HashMap::new(), 
            strategy 
        }
    }

    // Method to add a point and its associated value to the interpolation dataset
    pub fn add_point(&mut self, point: Point, value: f64) {
        self.points.insert(point, value);
    }

    // Delegates to the strategy's interpolate method
    pub fn interpolate(&self, target: &Vec<Vec<f64>>) -> Result<Vec<f64>, String> {
        self.strategy.interpolate(&self.points, &target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    const TOLERANCE: f64 = 1e-5;
    
    fn approx_eq(left: &[f64], right: &[f64], tolerance: f64) -> bool {
        left.len() == right.len() && 
        left.iter().zip(right.iter()).all(|(a, b)| (a - b).abs() <= tolerance)
    }

    fn setup_linear_interpolator() -> NDInterpolation<'static> {
        static LINEAR_STRATEGY: InterpolationStrategyEnum = InterpolationStrategyEnum::Linear(Linear);
        NDInterpolation::new(&LINEAR_STRATEGY)
    }

    #[test]
    fn test_extrapolation() {
        let strategy = InterpolationStrategyEnum::Linear(Linear);
        let mut interpolator = NDInterpolation::new(&strategy);

        // Add sample points
        interpolator.add_point(Point { coordinates: vec![1.0], file:None }, 2.0);
        interpolator.add_point(Point { coordinates: vec![2.0], file:None }, 4.0);
        interpolator.add_point(Point { coordinates: vec![3.0], file:None }, 6.0);
        interpolator.add_point(Point { coordinates: vec![4.0], file:None }, 8.0);
        interpolator.add_point(Point { coordinates: vec![5.0], file:None }, 10.0);

        let target = vec![vec![6.0]];
        let interpolated_values = interpolator.interpolate(&target).unwrap();

        let success = approx_eq(&interpolated_values, &vec![12.0], TOLERANCE);
        if !success {
            let message = format!(
                "The extrapolated value was {:?}, but {:?} was expected",
                interpolated_values, vec![12.0]
            );
            panic!("{}", message);
        }
    }

    #[test]
    fn test_basic_linear_interpolation() {
        let mut interpolator = setup_linear_interpolator();
        interpolator.add_point(Point { coordinates: vec![1.0], file: None }, 1.0);
        interpolator.add_point(Point { coordinates: vec![3.0], file: None }, 3.0);

        let target = vec![vec![2.0]];
        let interpolated_values = interpolator.interpolate(&target).unwrap();
        let expected = vec![2.0];
        assert!(approx_eq(&interpolated_values, &expected, TOLERANCE));
    }

    #[test]
    fn test_edge_case_interpolation() {
        let mut interpolator = setup_linear_interpolator();
        interpolator.add_point(Point { coordinates: vec![0.0], file: None }, 0.0);
        interpolator.add_point(Point { coordinates: vec![10.0], file: None }, 20.0);

        let targets = vec![vec![0.0], vec![10.0]];
        let interpolated_values = interpolator.interpolate(&targets).unwrap();
        let expected = vec![0.0, 20.0];

        assert!(approx_eq(&interpolated_values, &expected, TOLERANCE));
    }

    #[test]
    fn test_multiple_dimension_interpolation() {
        // Assuming NDInterpolation supports multi-dimensional points
        let mut interpolator = setup_linear_interpolator();
        interpolator.add_point(Point { coordinates: vec![0.0, 0.0], file: None }, 0.0);
        interpolator.add_point(Point { coordinates: vec![1.0, 1.0], file: None }, 2.0);

        let target = vec![vec![0.5, 0.5]];
        let interpolated_values = interpolator.interpolate(&target).unwrap();
        let expected = vec![1.0];

        assert!(approx_eq(&interpolated_values, &expected, TOLERANCE));
    }

    #[test]
    fn test_insufficient_points() {
        let mut interpolator = setup_linear_interpolator();
        interpolator.add_point(Point { coordinates: vec![1.0], file: None }, 1.0);

        let target = vec![vec![2.0]];
        let result = interpolator.interpolate(&target);

        assert!(result.is_err(), "Interpolation should fail with insufficient points.");
    }

    #[test]
    fn test_large_dataset_performance() {
        let mut interpolator = setup_linear_interpolator();

        // Generate a large dataset
        const DATASET_SIZE_IN: usize = 100; // Adjust the size based on your performance testing needs
        const DATASET_SIZE_TARGET: usize = 100000; // Adjust the size based on your performance testing needs
        for i in 0..DATASET_SIZE_IN {
            let x = i as f64;
            let y = 2.0 * x; // Simple linear relationship for the sake of example
            interpolator.add_point(Point { coordinates: vec![x], file: None }, y);
        }

        // Define a target vector for interpolation across a range of values
        let target: Vec<Vec<f64>> = (0..DATASET_SIZE_TARGET).map(|i| vec![i as f64 + 0.5]).collect();

        // Measure the time it takes to interpolate the entire dataset
        let start_time = Instant::now();
        let _ = interpolator.interpolate(&target).expect("Interpolation failed");
        let duration = start_time.elapsed();

        // Log the time taken to stdout (consider using logging frameworks for real applications)
        println!("Interpolated {} points in {:?}", DATASET_SIZE_IN, duration);

        // Example of asserting on performance (not recommended for CI/CD)
        // Assert that the operation completes within a specified duration (e.g., 2 seconds)
        // This is highly hardware and load dependent and should be used with caution
        assert!(duration < Duration::from_secs(2), "Interpolation took too long");
    }

}