//! A module for stress tensor operations
extern crate nalgebra as na;
use na::{Matrix3, SymmetricEigen, Vector6, Const};
use std::fs::File;
use std::io::{BufReader, BufRead, Error};
use std::path::Path;


/// A struct representing a stress tensor where the stress components are stored in a 3x3 matrix and a 6x1 vector
#[derive(Debug)]
pub struct StressTensor {
    matrix: Matrix3<f64>,
    vector: Vector6<f64>,
}

/// StressTensor implementation
impl StressTensor {
    pub fn new(matrix: Matrix3<f64>) -> Self {
        let vector = Self::matrix_to_vector(&matrix);
        StressTensor { matrix, vector }
    }

    // Methods to access specific components
    pub fn sxx(&self) -> f64 {
        self.matrix[(0, 0)]
    }

    pub fn syy(&self) -> f64 {
        self.matrix[(1, 1)]
    }

    pub fn szz(&self) -> f64 {
        self.matrix[(2, 2)]
    }

    pub fn sxy(&self) -> f64 {
        self.matrix[(0, 1)]
    }

    pub fn syz(&self) -> f64 {
        self.matrix[(1, 2)]
    }

    pub fn szx(&self) -> f64 {
        self.matrix[(0, 2)]
    }

    /// Converts a Matrix3 to a Vector6 following Voigt notation
    fn matrix_to_vector(matrix: &Matrix3<f64>) -> Vector6<f64> {
        Vector6::new(
            matrix[(0, 0)], // σxx
            matrix[(1, 1)], // σyy
            matrix[(2, 2)], // σzz
            matrix[(0, 1)], // τxy
            matrix[(1, 2)], // τyz
            matrix[(0, 2)], // τzx
        )
    }

    /// Converts the internal Vector6 back to a Matrix3
    /* fn vector_to_matrix(vector: &Vector6<f64>) -> Matrix3<f64> {
        Matrix3::new(
            vector[0], vector[3], vector[5], // Row 1: σxx, τxy, τzx
            vector[3], vector[1], vector[4], // Row 2: τxy, σyy, τyz
            vector[5], vector[4], vector[2], // Row 3: τzx, τyz, σzz
        )
    }*/

    // method to update the stress tensor
    pub fn update_stress(&mut self, matrix: Matrix3<f64>) {
        self.matrix = matrix;
        self.vector = Self::matrix_to_vector(&matrix);
    }

    // Calculate principal stresses and their directions
    pub fn principal_stresses(&self) -> SymmetricEigen<f64, Const<3>> {
        self.matrix.symmetric_eigen()
    }

    // Calculate the principal direction of the maximum principal stress
    pub fn principal_direction(&self) ->  Matrix3<f64> {
        let eigen = self.matrix.symmetric_eigen();
        let eigenvectors = eigen.eigenvectors;
        let x = eigenvectors.column(0).normalize();
        let mut z = eigenvectors.column(2).into_owned();
        z = z/eigenvectors.column(2).norm();
        let y = x.cross(&z);
        let rot = Matrix3::from_columns(&[x, z, y]);
        rot.transpose()
    }

    // Example method to get the maximum principal stress
    pub fn max_principal_stress(&self) -> f64 {
        let eigen = self.principal_stresses();
        *eigen.eigenvalues.as_slice().iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap()
    }

    pub fn von_mises_stress(&self) -> f64 {
        let principal_stresses = self.principal_stresses();
        let s1 = principal_stresses.eigenvalues[0];
        let s2 = principal_stresses.eigenvalues[1];
        let s3 = principal_stresses.eigenvalues[2];
        (((s1 - s2).powi(2) + (s2 - s3).powi(2) + (s3 - s1).powi(2)) / 2.0).sqrt()
    }
}

// Function to read stress tensors from a file and return them as a vector of tuples
// Each tuple contains a node number and a `StressTensor` instance
pub fn read_stress_tensors_from_file<P: AsRef<Path>>(path: P) -> Result<Vec<(usize, StressTensor)>, Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut tensors = Vec::new();

    for line in reader.lines() {
        let line = line?;
        // Update this based on your file's format
        let delimiter = ' '; // Assuming space or another character as delimiter
        let values: Vec<f64> = line.split(delimiter)
                                   .filter_map(|s| s.trim().parse::<f64>().ok())
                                   .collect();

        // Ensure there are enough values for a node number and a matrix
        if values.len() == 7 {
            let node_number = values[0] as usize;
            let matrix = Matrix3::new(
                values[1], values[4], values[6],
                values[4], values[2], values[5],
                values[6], values[5], values[3],
            );

            let tensor = StressTensor::new(matrix); // Assuming this constructor exists
            tensors.push((node_number, tensor));
        }
    }
    Ok(tensors)
}


#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use na::Vector3;
    use std::io;
    
    #[test]
    fn test_update_stress() {
        let initial_matrix = Matrix3::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let mut stress_tensor = StressTensor::new(initial_matrix);
        let new_matrix = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
        stress_tensor.update_stress(new_matrix);
        assert_eq!(stress_tensor.matrix, new_matrix);
        let expected_vector = Vector6::new(1.0, 5.0, 9.0, 2.0, 6.0, 3.0);
        assert_eq!(stress_tensor.vector, expected_vector);
    }

    #[test]
    fn test_principal_stresses() {
        let matrix = Matrix3::new(1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0);
        let stress_tensor = StressTensor::new(matrix);
        let principal_stresses = stress_tensor.principal_stresses();
        let expected_eigenvalues = Vector3::new(1.0, 2.0, 3.0); // Assuming sorted
        assert_eq!(principal_stresses.eigenvalues, expected_eigenvalues);
    }

    #[test]
    fn test_max_principal_stress() {
        let matrix = Matrix3::new(3.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 2.0);
        let stress_tensor = StressTensor::new(matrix);
        let max_stress = stress_tensor.max_principal_stress();
        assert_eq!(max_stress, 3.0);
    }

    #[test]
    fn test_von_mises_stress() {
        let matrix = Matrix3::new(1.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 3.0);
        let stress_tensor = StressTensor::new(matrix);
        let von_mises = stress_tensor.von_mises_stress();
        // Ensure literals are typed as f64 to match the expected type in calculations
        let expected_von_mises = (((1.0_f64 - 2.0_f64).powi(2) + (2.0_f64 - 3.0_f64).powi(2) + (3.0_f64 - 1.0_f64).powi(2)) / 2.0_f64).sqrt();
        assert_eq!(von_mises, expected_von_mises);
    }
   
    #[test]
    fn test_stress_tensor() {
        let matrix = Matrix3::new(
            1.0, 0.0, 2.0,
            0.0, 0.0, 0.0,
            2.0, 0.0, 3.0,
        );
        let stress = StressTensor::new(matrix);        
        let max_principal_stress = stress.max_principal_stress();
        assert_relative_eq!(max_principal_stress, 4.2360679774997898, epsilon = 1e-6);
        let von_mises_stress = stress.von_mises_stress();
        assert_relative_eq!(von_mises_stress, 4.358898943540674, epsilon = 1e-6);
        let matrix = Matrix3::new(
            -17.863839999999999, 1.54556, 0.016324870000000002,
            1.54556, -12.711300000000002, -0.013930999999999999,
            0.016324870000000002, -0.013930999999999999, -14.825930000000001,
        );
        let stress = StressTensor::new(matrix);            
        let direction_calc = stress.principal_direction();
        let direction = Matrix3::new(
            0.267,  0.964,   -0.004 ,
            -0.006,  -0.002, -1.000,
            -0.964, 0.267,  0.006,
        );
        assert_relative_eq!(direction_calc, direction, epsilon = 1e-3);
    }

    #[test]
    fn test_update_stress_and_vector_usage() {
        let initial_matrix = Matrix3::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);
        let mut stress_tensor = StressTensor::new(initial_matrix);
        // Initial vector check
        let expected_initial_vector = Vector6::new(1.0, 5.0, 9.0, 2.0, 6.0, 3.0);
        assert_eq!(stress_tensor.vector, expected_initial_vector);
    
        // Update stress
        let updated_matrix = Matrix3::new(9.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0);
        stress_tensor.update_stress(updated_matrix);
        // Updated vector check
        let expected_updated_vector = Vector6::new(9.0, 5.0, 1.0, 8.0, 4.0, 7.0);
        assert_eq!(stress_tensor.vector, expected_updated_vector);
    }

    #[test]
    fn test_read_stress_tensors_from_file() -> io::Result<()> {
        use crate::timeseries::ParseConfig;
        use std::path::PathBuf;
        use crate::timeseries::{Interpolation, Point}; // Ensure you import your Config and LoadCaseConfig

        // Assuming LoadCaseConfig is structured something like this
        let interp = Interpolation {
            method: "LINEAR".to_string(), // Assuming interpolation method
            name: "StressTimeseries".to_string(), // Name of the time series
            path: "tests/stressfile".to_string(), // Base path to your test files
            scale: 1.0, // Scale factor
            dimension: 3, // Dimension for interpolation
            sensor: vec!["FX".into(), "FY".into(), "FZ".into()], // Sensors for interpolation
            points: vec![
                Point {
                    file: Some("Fx.usf".to_string()), // File for interpolation point
                    coordinates: vec![0.0, 0.0, 0.0], // Assuming no specific value is provided; adjust if necessary
                },
            ],
            parse_config: ParseConfig {
                header: 1, // Assuming the first line is a header
                delimiter: " ".into(), // Assuming space-delimited values
            },
        };
        
        for point in &interp.points {
            // Check if `point.file` is `Some` and then construct the path
            if let Some(ref file_name) = point.file {
                let path = PathBuf::from(&interp.path).join(file_name); // Correctly constructs the path
                let tensors = read_stress_tensors_from_file(&path)?; // Assuming the function accepts a `&Path`
                        // Assert that tensors are read correctly
                // The exact assertions will depend on the expected content of your test file
                // For example, if you know the specific tensors that should be parsed, assert those
                assert!(!tensors.is_empty(), "Tensors should not be empty");
                // Example assertion: check for a specific tensor value or node number
                // assert_eq!(tensors[0].0, expected_node_number);
                // assert_eq!(tensors[0].1.matrix, expected_matrix);
                // Do something with `tensors` here
            }
        }
        
        Ok(())
    }    
}
