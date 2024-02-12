use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use nalgebra::{DMatrix, DVector};

const TOLERANCE: f64 = 1e-5; // Example tolerance level

// Let's start with our n-dimensional point struct from earlier
#[derive(Debug, Clone)]
struct Point {
    coordinates: Vec<f64>,
}

impl Eq for Point {}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.coordinates.len() == other.coordinates.len() &&
        self.coordinates.iter().zip(other.coordinates.iter()).all(|(a, b)| (a - b).abs() <= TOLERANCE)
    }
}

impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for &coord in &self.coordinates {
            // Discretize the coordinate by rounding it to the precision defined by TOLERANCE
            let discretized = (coord / TOLERANCE).round() * TOLERANCE;
            discretized.to_bits().hash(state); // Hash the bitwise representation of the discretized value
        }
    }
}

// Define a trait for our interpolation strategies
trait InterpolationStrategy {
    fn interpolate(&self, points: &HashMap<Point, f64>, target: &Point) -> f64;
}

// Implement nearest-neighbor interpolation
struct NearestNeighbor;

impl InterpolationStrategy for NearestNeighbor {
    fn interpolate(&self, points: &HashMap<Point, f64>, target: &Point) -> f64 {
        points.iter()
            .map(|(point, &value)| {
                let distance = point.coordinates.iter()
                    .zip(&target.coordinates)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt();
                (distance, value)
            })
            // We use `min_by` to find the point with the smallest distance.
            // `partial_cmp` is used for comparison, which handles floating point numbers.
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            // We're interested in the value of the nearest point, not its distance
            .map(|(_, value)| value)
            // If, for some reason, we have no points (which shouldn't happen if used correctly),
            // we return a default value. This is a defensive programming practice.
            .unwrap_or(0.0)
    }
}


// Implement linear interpolation
struct Linear;

impl InterpolationStrategy for Linear {
    fn interpolate(&self, points: &HashMap<Point, f64>, target: &Point) -> f64 {
        // Convert HashMap to the expected format for regression
        let points_vec: Vec<(Vec<f64>, f64)> = points.iter()
            .map(|(point, &value)| (point.coordinates.clone(), value))
            .collect();

        // Determine if extrapolation is needed
        if is_extrapolation(target, points) {
            // Perform multivariate linear regression using SVD
            let coefficients = multivariate_linear_regression_svd(&points_vec);

            // The first coefficient is the intercept; the rest are slopes for each dimension
            // Calculate the extrapolated value using the regression model
            let mut extrapolated_value = coefficients[0]; // start with intercept
            for (i, coord) in target.coordinates.iter().enumerate() {
                extrapolated_value += coefficients[i + 1] * coord; // add contribution of each dimension
            }
            return extrapolated_value;
        }

        // If not extrapolating, proceed with original interpolation logic
        let weights = calculate_interpolation_weights(target, points);
        compute_interpolated_value( points, &weights)
    }
}

fn multivariate_linear_regression_svd(points: &[(Vec<f64>, f64)]) -> Vec<f64> {
    // Assuming points is a slice of (features, target value) pairs

    let rows = points.len();
    let cols = points[0].0.len() + 1; // Number of features + 1 for the intercept
    let mut x_data = Vec::with_capacity(rows * cols);
    let mut y_data = Vec::with_capacity(rows);

    for (features, target) in points {
        let mut row = vec![1.0]; // Intercept term
        row.extend_from_slice(&features);
        x_data.extend_from_slice(&row);
        y_data.push(*target);
    }

    // Convert the data into nalgebra's DMatrix and DVector
    let x = DMatrix::from_row_slice(rows, cols, &x_data);
    let y = DVector::from_vec(y_data);

    // Perform SVD
    let svd = x.svd(true, true);
    let solution = svd.solve(&y, 1e-12).expect("Solving linear system failed");

    solution.iter().cloned().collect()
}

fn is_extrapolation(target: &Point, points: &HashMap<Point, f64>) -> bool {
    // Assuming all points, including the target, have the same number of dimensions
    let dimensions = target.coordinates.len();

    for dim in 0..dimensions {
        let (min_dim, max_dim) = points.keys().fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), point| {
            let coord = point.coordinates[dim];
            (min.min(coord), max.max(coord))
        });

        if target.coordinates[dim] < min_dim || target.coordinates[dim] > max_dim {
            return true; // Target is outside the bounds in at least one dimension
        }
    }

    false // Target is within bounds in all dimensions
}
fn calculate_trend_vector(points: &HashMap<Point, f64>) -> Vec<f64> {
    if points.is_empty() {
        return vec![];
    }

    let dimensions = points.keys().next().unwrap().coordinates.len();
    let mut centroid = vec![0.0; dimensions];
    let mut count = 0.0;

    // Calculate the centroid of the points
    for point in points.keys() {
        for (i, &coord) in point.coordinates.iter().enumerate() {
            centroid[i] += coord;
        }
        count += 1.0;
    }
    for i in 0..dimensions {
        centroid[i] /= count;
    }

    // Create a default point to use if no furthest point is found
    let default_point = Point { coordinates: vec![0.0; dimensions] };

    // Find the furthest point from the centroid to define the trend direction
    let furthest_point = points.keys().max_by(|&a, &b| {
        let distance_a = a.coordinates.iter().zip(&centroid)
            .map(|(a_coord, &c_coord)| (a_coord - c_coord).powi(2))
            .sum::<f64>();
        let distance_b = b.coordinates.iter().zip(&centroid)
            .map(|(b_coord, &c_coord)| (b_coord - c_coord).powi(2))
            .sum::<f64>();
        distance_a.partial_cmp(&distance_b).unwrap_or(std::cmp::Ordering::Equal)
    }).unwrap_or(&default_point); // Use the persistent default_point here

    // Calculate the trend vector as the difference between the furthest point and the centroid
    let trend_vector = furthest_point.coordinates.iter().zip(&centroid)
        .map(|(point_coord, &centroid_coord)| point_coord - centroid_coord)
        .collect::<Vec<f64>>();

    trend_vector
}


fn calculate_interpolation_weights(target: &Point, points: &HashMap<Point, f64>) -> HashMap<Point, f64> {
    let mut weights = HashMap::new();
    let trend_vector = calculate_trend_vector(points);
    let total_weight: f64 = points.iter().map(|(point, _)| {
        let distance = point.coordinates.iter().zip(&target.coordinates)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt();
        
        let alignment = point.coordinates.iter().zip(&trend_vector)
            .map(|(a, b)| a * b)
            .sum::<f64>();

        // Consider revising this to balance the influence of distance and alignment
        let weight = if distance > 0.0 { (1.0 / distance) * alignment.abs() } else { 0.0 };
        weights.insert(point.clone(), weight);
        weight
    }).sum();

    // Adjust normalization to account for total_weight possibly being 0 or very small
    if total_weight > 0.0 {
        for weight in weights.values_mut() {
            *weight /= total_weight;
        }
    }

    weights
}


fn compute_interpolated_value(points: &HashMap<Point, f64>, weights: &HashMap<Point, f64>) -> f64 {
    points.iter().fold(0.0, |acc, (point, &value)| {
        acc + weights.get(point).unwrap_or(&0.0) * value
    })
}

// Our n-dimensional interpolation space, now with strategy!
struct NDInterpolation {
    points: HashMap<Point, f64>,
    strategy: Box<dyn InterpolationStrategy>,
}

impl NDInterpolation {
    fn new(strategy: Box<dyn InterpolationStrategy>) -> Self {
        NDInterpolation {
            points: HashMap::new(),
            strategy,
        }
    }

    fn add_point(&mut self, point: Point, value: f64) {
        self.points.insert(point, value);
    }

    fn interpolate(&self, target: &Point) -> f64 {
        self.strategy.interpolate(&self.points, target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extrapolation() {
        // Set up an interpolator with a linear strategy
        let strategy = Box::new(Linear); // Assuming Linear is defined to handle extrapolation
        let mut interpolator = NDInterpolation::new(strategy);

        // Add some sample points for a simple linear relation y = 2x
        // For example, let's define points along this line from x=1 to x=5
        interpolator.add_point(Point { coordinates: vec![1.0] }, 2.0);
        interpolator.add_point(Point { coordinates: vec![2.0] }, 4.0);
        interpolator.add_point(Point { coordinates: vec![3.0] }, 6.0);
        interpolator.add_point(Point { coordinates: vec![4.0] }, 8.0);
        interpolator.add_point(Point { coordinates: vec![5.0] }, 10.0);

        // Define a target point for extrapolation (beyond the range of known points)
        let target_point = Point { coordinates: vec![6.0] };

        // Perform the interpolation (in this case, extrapolation)
        let interpolated_value = interpolator.interpolate(&target_point);

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
        let strategy = Box::new(Linear);
        let mut interpolator = NDInterpolation::new(strategy);

        // Add sample points that define a simple linear relationship
        // For simplicity, let's use a direct proportionality with a slope of 1 (y = x)
        interpolator.add_point(Point { coordinates: vec![1.0] }, 1.0);
        interpolator.add_point(Point { coordinates: vec![3.0] }, 3.0);

        // Define a target point that lies between the known points (e.g., x=2)
        // Based on our linear relationship, we expect the interpolated value at x=2 to be y=2
        let target_point = Point { coordinates: vec![2.0] };

        // Perform the interpolation
        let interpolated_value = interpolator.interpolate(&target_point);

        // Assert that the interpolated value matches the expected value (y=2)
        let expected_value = 2.5; // Expected value based on the linear relationship y = x
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
        let strategy = Box::new(Linear);
        let mut interpolator = NDInterpolation::new(strategy);

        // Add sample points in a 4-dimensional space
        // For simplicity, let's define the value at each point as the sum of its coordinates
        interpolator.add_point(Point { coordinates: vec![1.0, 1.0, 1.0, 1.0] }, 4.0); // Sum = 4
        interpolator.add_point(Point { coordinates: vec![2.0, 2.0, 2.0, 2.0] }, 8.0); // Sum = 8

        // Define a target point that lies exactly halfway between the known points
        // Expected value at the target is the average of the values at known points
        let target_point = Point { coordinates: vec![1.5, 1.5, 1.5, 1.5] }; // Expected sum = 6

        // Perform the interpolation
        let interpolated_value = interpolator.interpolate(&target_point);

        // Assert that the interpolated value matches the expected value
        let expected_value = 6.6666666666; // Expected value based on our simple linear relationship
        let tolerance = 1e-5; // Define a small tolerance for floating-point comparison

        assert!(
            (interpolated_value - expected_value).abs() <= tolerance,
            "The interpolated value was {}, but {} was expected in 4D space.",
            interpolated_value,
            expected_value
        );
    }

}