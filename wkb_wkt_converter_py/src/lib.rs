// PyO3's #[pyfunction] macro expansion triggers this lint as a false positive.
#![allow(clippy::useless_conversion)]

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBool;
use wkb_wkt_converter as core;

fn to_py_err(e: core::Error) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Maps the Python `srid` argument (`None`, `False`, or a non-negative integer) to
/// a [`core::SridMode`].  `True` is rejected with a clear error message.
fn parse_srid_arg(val: Option<Bound<'_, PyAny>>) -> PyResult<core::SridMode> {
    match val {
        None => Ok(core::SridMode::Auto),
        Some(v) => {
            if v.is_instance_of::<PyBool>() {
                if v.extract::<bool>()? {
                    Err(PyValueError::new_err(
                        "srid=True is not valid; pass an integer SRID, False, or None",
                    ))
                } else {
                    Ok(core::SridMode::Strip)
                }
            } else if let Ok(n) = v.extract::<u32>() {
                Ok(core::SridMode::Set(n))
            } else {
                Err(PyValueError::new_err(
                    "srid must be None, False, or a non-negative integer",
                ))
            }
        }
    }
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

/// Converts a WKT/EWKT string or a hex-encoded WKB/EWKB string to an uppercase
/// hex-encoded EWKB string.
/// The input format is detected automatically.
///
/// *srid* controls SRID handling in the output — see ``text_to_wkb``.
#[pyfunction]
#[pyo3(signature = (text, srid=None))]
fn text_to_hex_wkb(text: &str, srid: Option<Bound<'_, PyAny>>) -> PyResult<String> {
    core::text_to_hex_wkb(text, parse_srid_arg(srid)?).map_err(to_py_err)
}

/// Converts a WKT/EWKT string or a hex-encoded WKB/EWKB string to WKB bytes.
/// The input format is detected automatically.
///
/// *srid* controls SRID handling in the output:
/// - ``None`` (default): mirror the input — SRID is kept if present, absent if not.
/// - ``False``: always strip the SRID from the output.
/// - integer: always embed this SRID, overriding whatever the input contains.
#[pyfunction]
#[pyo3(signature = (text, srid=None))]
fn text_to_wkb(text: &str, srid: Option<Bound<'_, PyAny>>) -> PyResult<Vec<u8>> {
    core::text_to_wkb(text, parse_srid_arg(srid)?).map_err(to_py_err)
}

/// Converts a WKT/EWKT string or a hex-encoded WKB/EWKB string to a WKT string.
/// The input format is detected automatically.
///
/// *srid* controls SRID handling in the output:
/// - ``None`` (default): mirror the input — ``SRID=N;`` prefix kept if present, absent if not.
/// - ``False``: always strip the ``SRID=N;`` prefix from the output.
/// - integer: always prepend ``SRID=N;``, overriding whatever the input contains.
///
/// *normalize_wkt* (default ``False``): when ``True``, WKT input is normalised
/// (canonical casing, spacing, coordinate formatting) via a round-trip through
/// WKB.  When ``False``, WKT input is returned as-is (only the SRID prefix is
/// adjusted) — **no validation is performed and malformed WKT is returned
/// without raising an error.**  Hex WKB input is always decoded to normalised
/// WKT regardless of this flag.
#[pyfunction]
#[pyo3(signature = (text, srid=None, normalize_wkt=false))]
fn text_to_wkt(
    text: &str,
    srid: Option<Bound<'_, PyAny>>,
    normalize_wkt: bool,
) -> PyResult<String> {
    core::text_to_wkt(text, parse_srid_arg(srid)?, normalize_wkt).map_err(to_py_err)
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
    m.add_function(wrap_pyfunction!(text_to_hex_wkb, m)?)?;
    Ok(())
}
