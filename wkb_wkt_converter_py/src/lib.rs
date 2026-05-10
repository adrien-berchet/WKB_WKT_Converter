// PyO3's #[pyfunction] macro expansion triggers this lint as a false positive.
#![allow(clippy::useless_conversion)]

use pyo3::buffer::PyUntypedBuffer;
use pyo3::exceptions::{PyBufferError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyBytesMethods};
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

fn with_wkb_buffer<R>(wkb: &Bound<'_, PyAny>, f: impl FnOnce(&[u8]) -> PyResult<R>) -> PyResult<R> {
    if let Ok(bytes) = wkb.cast::<PyBytes>() {
        return f(bytes.as_bytes());
    }

    let buffer = PyUntypedBuffer::get(wkb)?;
    if buffer.item_size() != 1 || !buffer.is_c_contiguous() {
        return Err(PyBufferError::new_err(
            "wkb must be a contiguous one-byte buffer",
        ));
    }

    let len = buffer.len_bytes();
    let owned = if len == 0 {
        Vec::new()
    } else {
        // SAFETY: `PyUntypedBuffer::get` pins the exporter for the lifetime of
        // `buffer`, and the checks above guarantee a C-contiguous memory region
        // containing exactly `len` one-byte elements. The slice is copied
        // immediately, so mutable Python exporters cannot alias the immutable
        // WKB slice passed to core.
        unsafe { std::slice::from_raw_parts(buffer.buf_ptr().cast::<u8>(), len).to_vec() }
    };
    f(&owned)
}

/// Converts WKB/EWKB bytes-like input to a WKT/EWKT string.
///
/// ``wkb`` may be ``bytes``, ``bytearray``, ``memoryview``, or another
/// C-contiguous one-byte buffer object. It is always treated as raw WKB/EWKB.
///
/// *srid* controls SRID handling in the output:
/// - ``None`` (default): mirror the input — ``SRID=N;`` prefix kept if present, absent if not.
/// - ``False``: always strip the ``SRID=N;`` prefix from the output.
/// - integer: always prepend ``SRID=N;``, overriding whatever the input contains.
#[pyfunction]
#[pyo3(signature = (wkb, srid=None))]
fn wkb_to_wkt(wkb: Bound<'_, PyAny>, srid: Option<Bound<'_, PyAny>>) -> PyResult<String> {
    with_wkb_buffer(&wkb, |wkb| {
        core::wkb_to_wkt(wkb, parse_srid_arg(srid)?).map_err(to_py_err)
    })
}

/// Converts WKB/EWKB bytes-like input to a WKT string and returns the SRID separately.
/// The returned WKT string does not contain a `SRID=N;` prefix.
#[pyfunction]
fn wkb_to_wkt_split_srid(wkb: Bound<'_, PyAny>) -> PyResult<(String, Option<u32>)> {
    with_wkb_buffer(&wkb, |wkb| {
        core::wkb_to_wkt_split_srid(wkb).map_err(to_py_err)
    })
}

/// Converts a WKT/EWKT string to EWKB bytes.
///
/// *srid* controls SRID handling in the output:
/// - ``None`` (default): mirror the input — SRID is kept if present, absent if not.
/// - ``False``: always strip the SRID from the output.
/// - integer: always embed this SRID, overriding whatever the input contains.
#[pyfunction]
#[pyo3(signature = (wkt, srid=None))]
fn wkt_to_wkb(wkt: &str, srid: Option<Bound<'_, PyAny>>) -> PyResult<Vec<u8>> {
    core::wkt_to_wkb(wkt, parse_srid_arg(srid)?).map_err(to_py_err)
}

/// Converts a WKT/EWKT string to EWKB bytes and returns the SRID separately.
/// The SRID is not embedded in the returned bytes.
#[pyfunction]
fn wkt_to_wkb_split_srid(wkt: &str) -> PyResult<(Vec<u8>, Option<u32>)> {
    core::wkt_to_wkb_split_srid(wkt).map_err(to_py_err)
}

/// Converts a WKT/EWKT string to an uppercase hex-encoded EWKB string.
///
/// *srid* controls SRID handling in the output:
/// - ``None`` (default): mirror the input — SRID is kept if present, absent if not.
/// - ``False``: always strip the SRID from the output.
/// - integer: always embed this SRID, overriding whatever the input contains.
#[pyfunction]
#[pyo3(signature = (wkt, srid=None))]
fn wkt_to_hex_wkb(wkt: &str, srid: Option<Bound<'_, PyAny>>) -> PyResult<String> {
    core::wkt_to_hex_wkb(wkt, parse_srid_arg(srid)?).map_err(to_py_err)
}

/// Converts a hex-encoded WKB/EWKB string to a WKT/EWKT string.
///
/// *srid* controls SRID handling in the output:
/// - ``None`` (default): mirror the input — ``SRID=N;`` prefix kept if present, absent if not.
/// - ``False``: always strip the ``SRID=N;`` prefix from the output.
/// - integer: always prepend ``SRID=N;``, overriding whatever the input contains.
#[pyfunction]
#[pyo3(signature = (hex, srid=None))]
fn hex_wkb_to_wkt(hex: &str, srid: Option<Bound<'_, PyAny>>) -> PyResult<String> {
    core::hex_wkb_to_wkt(hex, parse_srid_arg(srid)?).map_err(to_py_err)
}

/// Converts a WKT/EWKT string or a hex-encoded WKB/EWKB string to an uppercase
/// hex-encoded EWKB string.
/// The input format is detected automatically: non-empty, even-length all-hex
/// text is treated as hex WKB; anything else is treated as WKT.
/// With ``srid=None``, hex input is validated as hex text and uppercased
/// without WKB structure validation.
///
/// *srid* controls SRID handling in the output — see ``to_wkb``.
#[pyfunction]
#[pyo3(signature = (text, srid=None))]
fn to_hex_wkb(text: &str, srid: Option<Bound<'_, PyAny>>) -> PyResult<String> {
    core::to_hex_wkb(text, parse_srid_arg(srid)?).map_err(to_py_err)
}

/// Converts a WKT/EWKT string or a hex-encoded WKB/EWKB string to WKB bytes.
/// The input format is detected automatically: non-empty, even-length all-hex
/// text is treated as hex WKB; anything else is treated as WKT.
///
/// *srid* controls SRID handling in the output:
/// - ``None`` (default): mirror the input — SRID is kept if present, absent if not.
///   Hex WKB bytes are returned as-is without WKB structure validation.
/// - ``False``: always strip the SRID from the output.
/// - integer: always embed this SRID, overriding whatever the input contains.
///
/// For ``False`` and integer SRIDs, canonical little-endian Point, LineString,
/// and Polygon EWKB hex input is patched at the top-level header without
/// scanning the geometry body. Malformed coordinate bodies or trailing bytes in
/// those simple fast paths can pass through as invalid output bytes.
#[pyfunction]
#[pyo3(signature = (text, srid=None))]
fn to_wkb(text: &str, srid: Option<Bound<'_, PyAny>>) -> PyResult<Vec<u8>> {
    core::to_wkb(text, parse_srid_arg(srid)?).map_err(to_py_err)
}

/// Converts a WKT/EWKT string or a hex-encoded WKB/EWKB string to a WKT string.
/// The input format is detected automatically: non-empty, even-length all-hex
/// text is treated as hex WKB; anything else is treated as WKT.
///
/// *srid* controls SRID handling in the output:
/// - ``None`` (default): mirror the input — ``SRID=N;`` prefix kept if present, absent if not.
/// - ``False``: always strip the ``SRID=N;`` prefix from the output.
/// - integer: always prepend ``SRID=N;``, overriding whatever the input contains.
///
/// *normalize_wkt* (default ``False``): when ``True``, WKT input is normalised
/// (canonical casing, spacing, coordinate formatting) via a round-trip through
/// WKB.  When ``False``, only the SRID prefix is adjusted — **no validation is
/// performed and malformed WKT is returned without raising an error.**
/// Leading/trailing whitespace is always stripped regardless of this flag.
/// Hex WKB input is always decoded to normalised WKT regardless of this flag.
/// Odd-length all-hex input is not hex WKB; with ``normalize_wkt=False`` it
/// follows the same unvalidated WKT fast path.
#[pyfunction]
#[pyo3(signature = (text, srid=None, normalize_wkt=false))]
fn to_wkt(text: &str, srid: Option<Bound<'_, PyAny>>, normalize_wkt: bool) -> PyResult<String> {
    core::to_wkt(text, parse_srid_arg(srid)?, normalize_wkt).map_err(to_py_err)
}

#[pymodule(name = "wkb_wkt_converter")]
fn wkb_wkt_converter_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(wkb_to_wkt, m)?)?;
    m.add_function(wrap_pyfunction!(wkb_to_wkt_split_srid, m)?)?;
    m.add_function(wrap_pyfunction!(wkt_to_wkb, m)?)?;
    m.add_function(wrap_pyfunction!(wkt_to_wkb_split_srid, m)?)?;
    m.add_function(wrap_pyfunction!(wkt_to_hex_wkb, m)?)?;
    m.add_function(wrap_pyfunction!(hex_wkb_to_wkt, m)?)?;
    m.add_function(wrap_pyfunction!(to_wkb, m)?)?;
    m.add_function(wrap_pyfunction!(to_wkt, m)?)?;
    m.add_function(wrap_pyfunction!(to_hex_wkb, m)?)?;
    Ok(())
}
