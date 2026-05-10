// PyO3's #[pyfunction] macro expansion triggers this lint as a false positive.
#![allow(clippy::useless_conversion)]

use pyo3::buffer::PyUntypedBuffer;
use pyo3::exceptions::{PyBufferError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyBytesMethods, PyString};
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

fn with_wkb_buffer<R>(
    input: &Bound<'_, PyAny>,
    arg_name: &str,
    f: impl FnOnce(&[u8]) -> PyResult<R>,
) -> PyResult<R> {
    if let Ok(bytes) = input.cast::<PyBytes>() {
        return f(bytes.as_bytes());
    }

    let buffer = match PyUntypedBuffer::get(input) {
        Ok(buffer) => buffer,
        Err(err) if err.is_instance_of::<PyTypeError>(input.py()) => {
            return Err(PyBufferError::new_err(format!(
                "{arg_name} must be a contiguous one-byte buffer"
            )));
        }
        Err(err) => return Err(err),
    };
    if buffer.item_size() != 1 || !buffer.is_c_contiguous() {
        return Err(PyBufferError::new_err(format!(
            "{arg_name} must be a contiguous one-byte buffer"
        )));
    }

    let len = buffer.len_bytes();
    let borrowed: &[u8] = if len == 0 {
        &[]
    } else {
        // SAFETY: `PyUntypedBuffer::get` keeps the exporter alive for the
        // lifetime of `buffer`, and the checks above guarantee a C-contiguous
        // memory region containing exactly `len` one-byte elements. The slice
        // is borrowed only for the callback below, while `buffer` remains in
        // scope. Callers keep Python argument extraction outside this borrowed
        // window, and the callback must not re-enter Python or release the GIL.
        // For performance, writable exporters are borrowed too; the public API
        // documents that mutating the buffer during conversion is unsupported.
        unsafe { std::slice::from_raw_parts(buffer.buf_ptr().cast::<u8>(), len) }
    };
    f(borrowed)
}

/// Converts WKB/EWKB bytes-like input to a WKT/EWKT string.
///
/// ``wkb`` may be ``bytes``, ``bytearray``, ``memoryview``, or another
/// C-contiguous one-byte buffer object. It is always treated as raw WKB/EWKB.
/// For performance, bytes-like inputs may be borrowed directly without
/// copying; mutating a writable buffer during conversion is unsupported and may
/// produce invalid or inconsistent results.
///
/// *srid* controls SRID handling in the output:
/// - ``None`` (default): mirror the input — ``SRID=N;`` prefix kept if present, absent if not.
/// - ``False``: always strip the ``SRID=N;`` prefix from the output.
/// - integer: always prepend ``SRID=N;``, overriding whatever the input contains.
#[pyfunction]
#[pyo3(signature = (wkb, srid=None))]
fn wkb_to_wkt(wkb: Bound<'_, PyAny>, srid: Option<Bound<'_, PyAny>>) -> PyResult<String> {
    let srid = parse_srid_arg(srid)?;
    with_wkb_buffer(&wkb, "wkb", |wkb| {
        core::wkb_to_wkt(wkb, srid).map_err(to_py_err)
    })
}

/// Converts WKB/EWKB bytes-like input to a WKT string and returns the SRID separately.
/// The returned WKT string does not contain a `SRID=N;` prefix.
///
/// For performance, bytes-like inputs may be borrowed directly without
/// copying; mutating a writable buffer during conversion is unsupported and may
/// produce invalid or inconsistent results.
#[pyfunction]
fn wkb_to_wkt_split_srid(wkb: Bound<'_, PyAny>) -> PyResult<(String, Option<u32>)> {
    with_wkb_buffer(&wkb, "wkb", |wkb| {
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
#[pyo3(signature = (hex_wkb, srid=None))]
fn hex_wkb_to_wkt(hex_wkb: &str, srid: Option<Bound<'_, PyAny>>) -> PyResult<String> {
    core::hex_wkb_to_wkt(hex_wkb, parse_srid_arg(srid)?).map_err(to_py_err)
}

/// Converts a WKT/EWKT string, a hex-encoded WKB/EWKB string, or bytes-like
/// WKB/EWKB input to an uppercase hex-encoded EWKB string.
/// The string input format is detected automatically: non-empty, even-length
/// all-hex text is treated as hex WKB; anything else is treated as WKT.
/// Bytes-like input is always treated as raw WKB/EWKB.
/// With ``srid=None``, hex input is validated as hex text and uppercased
/// without WKB structure validation, and bytes-like input is hex-encoded
/// without WKB structure validation.
/// For performance, bytes-like inputs may be borrowed directly without
/// copying; mutating a writable buffer during conversion is unsupported and may
/// produce invalid or inconsistent results.
///
/// *srid* controls SRID handling in the output — see ``to_wkb``.
#[pyfunction]
#[pyo3(signature = (source, srid=None))]
fn to_hex_wkb(source: Bound<'_, PyAny>, srid: Option<Bound<'_, PyAny>>) -> PyResult<String> {
    let srid = parse_srid_arg(srid)?;
    if let Ok(text) = source.cast::<PyString>() {
        return core::to_hex_wkb(core::Input::Text(text.to_str()?), srid).map_err(to_py_err);
    }
    with_wkb_buffer(&source, "source", |wkb| {
        core::to_hex_wkb(core::Input::Wkb(wkb), srid).map_err(to_py_err)
    })
}

/// Converts a WKT/EWKT string, a hex-encoded WKB/EWKB string, or bytes-like
/// WKB/EWKB input to WKB bytes.
/// The string input format is detected automatically: non-empty, even-length
/// all-hex text is treated as hex WKB; anything else is treated as WKT.
/// Bytes-like input is always treated as raw WKB/EWKB.
///
/// *srid* controls SRID handling in the output:
/// - ``None`` (default): mirror the input — SRID is kept if present, absent if not.
///   Hex WKB and bytes-like WKB bytes are returned as-is without WKB structure validation.
/// - ``False``: always strip the SRID from the output.
/// - integer: always embed this SRID, overriding whatever the input contains.
///
/// For performance, bytes-like inputs may be borrowed directly without
/// copying; mutating a writable buffer during conversion is unsupported and may
/// produce invalid or inconsistent results.
///
/// For ``False`` and integer SRIDs, canonical little-endian Point, LineString,
/// and Polygon EWKB hex or bytes-like input is patched at the top-level header
/// without scanning the geometry body. Malformed coordinate bodies or trailing
/// bytes in those simple fast paths can pass through as invalid output bytes.
#[pyfunction]
#[pyo3(signature = (source, srid=None))]
fn to_wkb(source: Bound<'_, PyAny>, srid: Option<Bound<'_, PyAny>>) -> PyResult<Vec<u8>> {
    let srid = parse_srid_arg(srid)?;
    if let Ok(text) = source.cast::<PyString>() {
        return core::to_wkb(core::Input::Text(text.to_str()?), srid).map_err(to_py_err);
    }
    with_wkb_buffer(&source, "source", |wkb| {
        core::to_wkb(core::Input::Wkb(wkb), srid).map_err(to_py_err)
    })
}

/// Converts a WKT/EWKT string, a hex-encoded WKB/EWKB string, or bytes-like
/// WKB/EWKB input to a WKT string.
/// The string input format is detected automatically: non-empty, even-length
/// all-hex text is treated as hex WKB; anything else is treated as WKT.
/// Bytes-like input is always treated as raw WKB/EWKB.
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
/// Hex WKB and bytes-like WKB input are always decoded to normalised WKT
/// regardless of this flag.
/// Odd-length all-hex input is not hex WKB; with ``normalize_wkt=False`` it
/// follows the same unvalidated WKT fast path.
/// For performance, bytes-like inputs may be borrowed directly without
/// copying; mutating a writable buffer during conversion is unsupported and may
/// produce invalid or inconsistent results.
#[pyfunction]
#[pyo3(signature = (source, srid=None, normalize_wkt=false))]
fn to_wkt(
    source: Bound<'_, PyAny>,
    srid: Option<Bound<'_, PyAny>>,
    normalize_wkt: bool,
) -> PyResult<String> {
    let srid = parse_srid_arg(srid)?;
    if let Ok(text) = source.cast::<PyString>() {
        return core::to_wkt(core::Input::Text(text.to_str()?), srid, normalize_wkt)
            .map_err(to_py_err);
    }
    with_wkb_buffer(&source, "source", |wkb| {
        core::to_wkt(core::Input::Wkb(wkb), srid, normalize_wkt).map_err(to_py_err)
    })
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
