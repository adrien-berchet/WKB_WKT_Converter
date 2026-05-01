// PyO3's #[pyfunction] macro expansion triggers this lint as a false positive.
#![allow(clippy::useless_conversion)]

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use wkb_wkt_converter as core;

fn to_py_err(e: core::Error) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Converts WKB/EWKB bytes to a WKT/EWKT string.
/// If the input is EWKB with an SRID, the output includes a `SRID=N;` prefix.
#[pyfunction]
fn wkb_to_wkt(wkb: &[u8]) -> PyResult<String> {
    core::wkb_to_wkt(wkb).map_err(to_py_err)
}

/// Converts WKB/EWKB bytes to a WKT string and returns the SRID separately.
/// The returned WKT string does not contain a `SRID=N;` prefix.
#[pyfunction]
fn wkb_to_wkt_split_srid(wkb: &[u8]) -> PyResult<(String, Option<u32>)> {
    core::wkb_to_wkt_split_srid(wkb).map_err(to_py_err)
}

/// Converts a WKT/EWKT string to EWKB bytes.
/// If the input includes a `SRID=N;` prefix, the SRID is embedded in the output.
#[pyfunction]
fn wkt_to_wkb(wkt: &str) -> PyResult<Vec<u8>> {
    core::wkt_to_wkb(wkt).map_err(to_py_err)
}

/// Converts a WKT/EWKT string to EWKB bytes and returns the SRID separately.
/// The SRID is not embedded in the returned bytes.
#[pyfunction]
fn wkt_to_wkb_split_srid(wkt: &str) -> PyResult<(Vec<u8>, Option<u32>)> {
    core::wkt_to_wkb_split_srid(wkt).map_err(to_py_err)
}

/// Converts a WKT/EWKT string to an uppercase hex-encoded EWKB string.
#[pyfunction]
fn wkt_to_hex_wkb(wkt: &str) -> PyResult<String> {
    core::wkt_to_hex_wkb(wkt).map_err(to_py_err)
}

/// Converts a hex-encoded WKB/EWKB string to a WKT/EWKT string.
#[pyfunction]
fn hex_wkb_to_wkt(hex: &str) -> PyResult<String> {
    core::hex_wkb_to_wkt(hex).map_err(to_py_err)
}

/// Converts a WKT/EWKT string or a hex-encoded WKB/EWKB string to WKB bytes.
/// The input format is detected automatically.
///
/// *extended* controls SRID handling in the output:
/// - ``None`` (default): mirror the input — SRID is kept if present, absent if not.
/// - ``True``: force EWKB — SRID is embedded when present in the input.
/// - ``False``: force plain WKB — SRID is always stripped.
#[pyfunction]
#[pyo3(signature = (text, extended=None))]
fn text_to_wkb(text: &str, extended: Option<bool>) -> PyResult<Vec<u8>> {
    core::text_to_wkb(text, extended).map_err(to_py_err)
}

/// Converts a WKT/EWKT string or a hex-encoded WKB/EWKB string to a WKT string.
/// The input format is detected automatically; WKT input is normalised.
///
/// *extended* controls SRID handling in the output:
/// - ``None`` (default): mirror the input — ``SRID=N;`` prefix is kept if present, absent if not.
/// - ``True``: force EWKT — ``SRID=N;`` prefix is included when present in the input.
/// - ``False``: force plain WKT — ``SRID=N;`` prefix is always stripped.
#[pyfunction]
#[pyo3(signature = (text, extended=None))]
fn text_to_wkt(text: &str, extended: Option<bool>) -> PyResult<String> {
    core::text_to_wkt(text, extended).map_err(to_py_err)
}

#[pymodule(name = "wkb_wkt_converter")]
fn wkb_wkt_converter_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(wkb_to_wkt, m)?)?;
    m.add_function(wrap_pyfunction!(wkb_to_wkt_split_srid, m)?)?;
    m.add_function(wrap_pyfunction!(wkt_to_wkb, m)?)?;
    m.add_function(wrap_pyfunction!(wkt_to_wkb_split_srid, m)?)?;
    m.add_function(wrap_pyfunction!(wkt_to_hex_wkb, m)?)?;
    m.add_function(wrap_pyfunction!(hex_wkb_to_wkt, m)?)?;
    m.add_function(wrap_pyfunction!(text_to_wkb, m)?)?;
    m.add_function(wrap_pyfunction!(text_to_wkt, m)?)?;
    Ok(())
}
