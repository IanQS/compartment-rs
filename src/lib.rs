use pyo3::prelude::*;
mod swc_reader;

/// A Python module implemented in Rust.
#[pymodule]
mod compartment_rs {
    use pyo3::prelude::*;

    /// Formats the sum of two numbers as string.
    #[pyfunction]
    fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
        Ok((a + b).to_string())
    }
}
