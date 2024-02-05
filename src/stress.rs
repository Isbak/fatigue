extern crate nalgebra as na;
use na::{Matrix3, SymmetricEigen, Vector3, Vector6, Const};

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
        let mut stress = StressTensor::new(matrix);        
        let max_principal_stress = stress.max_principal_stress();
        assert_relative_eq!(max_principal_stress, 4.2360679774997898, epsilon = 1e-6);
        let von_mises_stress = stress.von_mises_stress();
        assert_relative_eq!(von_mises_stress, 4.358898943540674, epsilon = 1e-6);
        let matrix = Matrix3::new(
            -17.863839999999999, 1.54556, 0.016324870000000002,
            1.54556, -12.711300000000002, -0.013930999999999999,
            0.016324870000000002, -0.013930999999999999, -14.825930000000001,
        );
        let mut stress = StressTensor::new(matrix);            
        let direction_calc = stress.principal_direction();
        let direction = Matrix3::new(
            0.267,  0.964,   -0.004 ,
            -0.006,  -0.002, -1.000,
            -0.964, 0.267,  0.006,
        );
        assert_relative_eq!(direction_calc, direction, epsilon = 1e-3);
    }
}