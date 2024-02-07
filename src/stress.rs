extern crate nalgebra as na;
use na::{Matrix3, SymmetricEigen, Vector6, Const};
use std::fs::File;
use std::io::{self, BufReader, prelude::*};
use std::path::Path;
use crate::config::{LoadCaseConfig, LoadCaseParseConfig}; // Ensure you import your Config and LoadCaseConfig

#[derive(Debug)]
pub struct StressTensor {
    matrix: Matrix3<f64>,
    vector: Vector6<f64>,
}

impl StressTensor {
    pub fn new(matrix: Matrix3<f64>) -> Self {
        let vector = Self::matrix_to_vector(&matrix);
        StressTensor { matrix, vector }
    }

    // Converts a Matrix3 to a Vector6 following Voigt notation
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

    // Converts the internal Vector6 back to a Matrix3
    fn vector_to_matrix(vector: &Vector6<f64>) -> Matrix3<f64> {
        Matrix3::new(
            vector[0], vector[3], vector[5], // Row 1: σxx, τxy, τzx
            vector[3], vector[1], vector[4], // Row 2: τxy, σyy, τyz
            vector[5], vector[4], vector[2], // Row 3: τzx, τyz, σzz
        )
    }

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

    // Add more methods as needed, e.g., to get principal directions, etc.
}

pub fn read_stress_tensors_from_file(
    load_case_config: &LoadCaseConfig,
) -> io::Result<Vec<(usize, StressTensor)>> {
    let file_path = Path::new(&load_case_config.path).join(&load_case_config.load);
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let mut tensors = Vec::new();

    let lines = reader.lines().skip(load_case_config.parse_config.header);

    for line in lines {
        let line = line?;
        let delimiter = &load_case_config.parse_config.delimiter.chars().next().unwrap_or(' ').to_string();
        let values: Vec<f64> =  line.split(delimiter)
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        if values.len() == 7 {
            let node_number = values[0] as usize;
            let matrix = Matrix3::new(
                values[1], values[4], values[6],
                values[4], values[2], values[5],
                values[6], values[5], values[3],
            );

            let tensor = StressTensor::new(matrix);
            tensors.push((node_number, tensor));
        }
    }

    Ok(tensors)
}


#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

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
    fn test_vector_to_matrix_conversion() {
        let vector = Vector6::new(1.0, 5.0, 9.0, 2.0, 6.0, 3.0);
        let expected_matrix = Matrix3::new(1.0, 2.0, 3.0, 2.0, 5.0, 6.0, 3.0, 6.0, 9.0);
        let matrix = StressTensor::vector_to_matrix(&vector);
        assert_eq!(matrix, expected_matrix);
    }

    #[test]
    fn test_read_stress_tensors_from_file() -> io::Result<()> {
        // Assuming LoadCaseConfig is structured something like this
        let load_case_config = LoadCaseConfig {
            path: "tests/stressfile".to_string(), // Base path to your test files
            load: "Fx.usf".into(), // Specific test file name
            scale: 1.0, // Assuming a scale factor
            timeseries: String::from(""), 
            parse_config: LoadCaseParseConfig {
                header: 1, // Assuming the first line is a header
                delimiter: " ".into(), // Assuming comma-delimited values, adjust as necessary
            },
        };

        // Call the function with the test config
        let tensors = read_stress_tensors_from_file(&load_case_config)?;

        // Assert that tensors are read correctly
        // The exact assertions will depend on the expected content of your test file
        // For example, if you know the specific tensors that should be parsed, assert those
        assert!(!tensors.is_empty(), "Tensors should not be empty");
        // Example assertion: check for a specific tensor value or node number
        // assert_eq!(tensors[0].0, expected_node_number);
        // assert_eq!(tensors[0].1.matrix, expected_matrix);

        Ok(())
    }    
}
