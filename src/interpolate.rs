use std::collections::HashMap;
use nalgebra::{DMatrix, DVector};
use crate::timeseries::Point;

// Define a trait for our interpolation strategies
pub trait InterpolationStrategy {
    // Corrected to include only two parameters: points and target
    fn interpolate(&self, points: &HashMap<Point, f64>, target: &Point) -> Result<f64, String>;
}


// Implement nearest-neighbor interpolation
pub struct NearestNeighbor;

impl InterpolationStrategy for NearestNeighbor {
    fn interpolate(&self, points: &HashMap<Point, f64>, target: &Point) -> Result<f64, String> {
        points.iter()
            .map(|(point, &value)| {
                let distance = point.coordinates.iter()
                    .zip(&target.coordinates)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt();
                (distance, value)
            })
            // Handle floating point comparison carefully
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            // We're interested in the value of the nearest point, not its distance
            .map(|(_, value)| value)
            // Convert to Result; provide an error message if no points are found
            .ok_or_else(|| "No points available for interpolation.".to_string())
    }
}


// Implement linear interpolation
pub struct Linear;

impl InterpolationStrategy for Linear {
    fn interpolate(&self, points: &HashMap<Point, f64>, target: &Point) -> Result<f64, String> {
        // Ensure there are enough points for SVD-based interpolation/extrapolation
        if points.len() < 2 {
            return Err("Not enough points for interpolation".to_string());
        }

        // Convert points to a format suitable for SVD and regression
        let points_vec: Vec<(Vec<f64>, f64)> = points.iter()
            .map(|(point, &value)| (point.coordinates.clone(), value))
            .collect();

        // Perform multivariate linear regression using SVD
        let coefficients = multivariate_linear_regression_svd(&points_vec)
            .map_err(|e| format!("Failed to perform linear regression: {}", e))?;

        // Calculate the interpolated or extrapolated value using the regression model
        // Start with the intercept (the first coefficient)
        let mut interpolated_value = coefficients[0];
        // Add the contribution of each dimension
        for (i, coord) in target.coordinates.iter().enumerate() {
            if i + 1 < coefficients.len() {
                interpolated_value += coefficients[i + 1] * coord;
            }
        }
        Ok(interpolated_value)
    }
}

fn multivariate_linear_regression_svd(points: &[(Vec<f64>, f64)]) -> Result<Vec<f64>, String> {
    if points.is_empty() {
        return Err("No points provided for linear regression.".to_string());
    }

    let rows = points.len();
    let cols = points[0].0.len() + 1; // Number of features + 1 for the intercept
    let mut x_data = Vec::with_capacity(rows * cols);
    let mut y_data = Vec::with_capacity(rows);

    for (features, target) in points {
        let mut row = vec![1.0]; // Intercept term
        row.extend(features.iter());
        x_data.extend(row);
        y_data.push(*target);
    }

    // Convert the data into nalgebra's DMatrix and DVector
    let x = DMatrix::from_row_slice(rows, cols, &x_data);
    let y = DVector::from_vec(y_data);

    // Perform SVD
    let svd = x.svd(true, true);
    match svd.solve(&y, 1e-12) {
        Ok(solution) => Ok(solution.iter().cloned().collect()),
        Err(_) => Err("Failed to solve the linear system using SVD.".to_string()),
    }
}

// NDInterpolation now includes a lifetime parameter `'a`
pub struct NDInterpolation<'a> {
    points: HashMap<Point, f64>,
    strategy: &'a Box<dyn InterpolationStrategy>,
}

impl<'a> NDInterpolation<'a> {
    // Accepts a reference to a Box<dyn InterpolationStrategy> with the same lifetime `'a`
    pub fn new(strategy: &'a Box<dyn InterpolationStrategy>) -> Self {
        NDInterpolation {
            points: HashMap::new(),
            strategy,
        }
    }

    pub fn add_point(&mut self, point: Point, value: f64) {
        self.points.insert(point, value);
    }

    // Delegates to the strategy's interpolate method
    pub fn interpolate(&self, target: &Point) -> Result<f64, String> {
        self.strategy.interpolate(&self.points, target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extrapolation() {
        // Set up an interpolator with a linear strategy
        let strategy: Box<dyn InterpolationStrategy> = Box::new(Linear);
        let mut interpolator = NDInterpolation::new(&strategy);

        // Add some sample points for a simple linear relation y = 2x
        // For example, let's define points along this line from x=1 to x=5
        interpolator.add_point(Point { coordinates: vec![1.0] , file:None}, 2.0);
        interpolator.add_point(Point { coordinates: vec![2.0] , file:None}, 4.0);
        interpolator.add_point(Point { coordinates: vec![3.0] , file:None}, 6.0);
        interpolator.add_point(Point { coordinates: vec![4.0] , file:None}, 8.0);
        interpolator.add_point(Point { coordinates: vec![5.0] , file:None}, 10.0);

        // Define a target point for extrapolation (beyond the range of known points)
        let target_point = Point { coordinates: vec![6.0] , file:None};

        // Perform the interpolation (in this case, extrapolation)
        let interpolated_value = interpolator.interpolate(&target_point).unwrap();

        // Assert the expected value based on the linear relation y = 2x
        // Since we're extrapolating for x=6, we expect y=12 according to our linear relation
        let expected_value = 12.0;
        let tolerance = 1e-5; // Define a small tolerance for floating-point comparison

        assert!(
            (interpolated_value - expected_value).abs() <= tolerance,
            "The extrapolated value was {}, but {} was expected",
            interpolated_value,
            expected_value
        );
    }

    #[test]
    fn test_linear_interpolation() {
        // Initialize the interpolator with the Linear strategy
        let strategy: Box<dyn InterpolationStrategy> = Box::new(Linear);
        let mut interpolator = NDInterpolation::new(&strategy);

        // Add sample points that define a simple linear relationship
        // For simplicity, let's use a direct proportionality with a slope of 1 (y = x)
        interpolator.add_point(Point { coordinates: vec![1.0] , file:None}, 1.0);
        interpolator.add_point(Point { coordinates: vec![3.0] , file:None}, 3.0);

        // Define a target point that lies between the known points (e.g., x=2)
        // Based on our linear relationship, we expect the interpolated value at x=2 to be y=2
        let target_point = Point { coordinates: vec![2.0] , file:None};

        // Perform the interpolation
        let interpolated_value = interpolator.interpolate(&target_point).unwrap();

        // Assert that the interpolated value matches the expected value (y=2)
        let expected_value = 2.0; // Expected value based on the linear relationship y = x
        let tolerance = 1e-5; // Define a small tolerance for floating-point comparison

        assert!(
            (interpolated_value - expected_value).abs() <= tolerance,
            "The interpolated value was {}, but {} was expected.",
            interpolated_value,
            expected_value
        );
    }

    #[test]
    fn test_4d_linear_interpolation() {
        // Initialize the interpolator with the Linear strategy
        let strategy: Box<dyn InterpolationStrategy> = Box::new(Linear);
        let mut interpolator = NDInterpolation::new(&strategy);

        // Add sample points in a 4-dimensional space
        // For simplicity, let's define the value at each point as the sum of its coordinates
        interpolator.add_point(Point { coordinates: vec![1.0, 1.0, 1.0, 1.0] , file:None}, 4.0); // Sum = 4
        interpolator.add_point(Point { coordinates: vec![2.0, 2.0, 2.0, 2.0] , file:None}, 8.0); // Sum = 8

        // Define a target point that lies exactly halfway between the known points
        // Expected value at the target is the average of the values at known points
        let target_point = Point { coordinates: vec![1.5, 1.5, 1.5, 1.5] , file:None}; // Expected sum = 6

        // Perform the interpolation
        let interpolated_value = interpolator.interpolate(&target_point).unwrap();

        // Assert that the interpolated value matches the expected value
        let expected_value = 6.0; // Expected value based on our simple linear relationship
        let tolerance = 1e-5; // Define a small tolerance for floating-point comparison

        assert!(
            (interpolated_value - expected_value).abs() <= tolerance,
            "The interpolated value was {}, but {} was expected in 4D space.",
            interpolated_value,
            expected_value
        );
    }
    #[test]
    fn test_non_linear_relationship() {
        let strategy: Box<dyn InterpolationStrategy> = Box::new(Linear);
        let mut interpolator = NDInterpolation::new(&strategy);
    
        // Define points that form a quadratic relationship, y = x^2
        interpolator.add_point(Point { coordinates: vec![1.0], file: None }, 1.0);
        interpolator.add_point(Point { coordinates: vec![2.0], file: None }, 4.0);
        interpolator.add_point(Point { coordinates: vec![3.0], file: None }, 9.0);
    
        // Attempt to interpolate at a point within the dataset range
        let target_point = Point { coordinates: vec![2.5], file: None };
        let interpolated_value = interpolator.interpolate(&target_point).unwrap();
    
        // Calculate the expected value based on the quadratic relationship (not linear!)
        let expected_value = 2.5 * 2.5;
    
        let tolerance = 1e-5;
        assert!(
            (interpolated_value - expected_value).abs() > tolerance,
            "Interpolation inaccurately predicts non-linear relationships."
        );        
    }
    #[test]
    fn test_insufficient_points() {
        let strategy: Box<dyn InterpolationStrategy> = Box::new(Linear);
        let mut interpolator = NDInterpolation::new(&strategy);
    
        // Add a single point, insufficient for linear interpolation
        interpolator.add_point(Point { coordinates: vec![1.0], file: None }, 1.0);
    
        // Attempt to interpolate
        let target_point = Point { coordinates: vec![2.0], file: None };
        let result = interpolator.interpolate(&target_point);
    
        assert!(result.is_err(), "Interpolation should fail with insufficient points.");
    }    
    
}