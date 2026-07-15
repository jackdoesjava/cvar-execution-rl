use pyo3::prelude::*;

mod lob;

/// Expose our Rust OrderBook to Python using PyO3 0.20 syntax
#[pymodule]
fn env_engine(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<lob::OrderBook>()?;
    Ok(())
}