// PyO3's #[pyfunction] macro expansion triggers this lint as a false positive.
#![allow(clippy::useless_conversion)]

use pyo3::buffer::PyUntypedBuffer;
use pyo3::exceptions::{PyAttributeError, PyBufferError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyBytesMethods, PyInt, PyString};
use wkb_wkt_converter as core;

fn to_py_err(e: core::Error) -> PyErr {
    PyValueError::new_err(e.to_string())
}

fn srid_range_error() -> PyErr {
    PyValueError::new_err(
        "srid is out of the allowed range: must fit in a 32-bit integer \
         (-2147483648 to 2147483647)",
    )
}

#[derive(Clone, Copy)]
enum ExplicitSridMode {
    Strip,
    Set(i32),
}

impl From<ExplicitSridMode> for core::SridMode {
    fn from(mode: ExplicitSridMode) -> Self {
        match mode {
            ExplicitSridMode::Strip => core::SridMode::Strip,
            ExplicitSridMode::Set(n) => core::SridMode::Set(n),
        }
    }
}

fn parse_srid_value(
    val: &Bound<'_, PyAny>,
    true_error: &'static str,
    type_error: &'static str,
) -> PyResult<ExplicitSridMode> {
    if val.is_instance_of::<PyBool>() {
        if val.extract::<bool>()? {
            Err(PyValueError::new_err(true_error))
        } else {
            Ok(ExplicitSridMode::Strip)
        }
    } else if val.is_instance_of::<PyInt>() {
        parse_srid_int(val, type_error)
    } else {
        let index_method = match val.getattr("__index__") {
            Ok(index_method) => index_method,
            Err(err) if err.is_instance_of::<PyAttributeError>(val.py()) => {
                return Err(PyValueError::new_err(type_error));
            }
            Err(err) => return Err(err),
        };
        let index = index_method.call0()?;
        parse_srid_int(&index, type_error)
    }
}

fn parse_srid_int(val: &Bound<'_, PyAny>, type_error: &'static str) -> PyResult<ExplicitSridMode> {
    if val.is_instance_of::<PyBool>() || !val.is_instance_of::<PyInt>() {
        return Err(PyValueError::new_err(type_error));
    }
    val.extract::<i32>()
        .map(ExplicitSridMode::Set)
        .map_err(|_| srid_range_error())
}

/// Maps the Python `srid` argument (`None`, `False`, or an integer) to
/// a [`core::SridMode`].  `True` is rejected with a clear error message.
fn parse_srid_arg(val: Option<Bound<'_, PyAny>>) -> PyResult<core::SridMode> {
    match val {
        None => Ok(core::SridMode::Auto),
        Some(v) => parse_srid_value(
            &v,
            "srid=True is not valid; pass an integer SRID, False, or None",
            "srid must be None, False, or an integer",
        )
        .map(Into::into),
    }
}

/// Maps the required Python `srid` argument for ``to_ewkb_header``.
///
/// `False` strips the SRID, `True` is rejected to avoid ambiguity, and an
/// integer embeds or replaces the SRID.
fn parse_header_srid_arg(val: Bound<'_, PyAny>) -> PyResult<ExplicitSridMode> {
    parse_srid_value(
        &val,
        "srid=True is not valid; pass an integer SRID or False",
        "srid must be False or an integer",
    )
}

fn apply_header_srid_arg(wkb: &[u8], srid: ExplicitSridMode) -> Result<Vec<u8>, core::Error> {
    match srid {
        ExplicitSridMode::Strip => core::wkb_strip_srid(wkb),
        ExplicitSridMode::Set(n) => core::wkb_set_srid(wkb, n),
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
fn wkb_to_wkt_split_srid(wkb: Bound<'_, PyAny>) -> PyResult<(String, Option<i32>)> {
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
fn wkt_to_wkb_split_srid(wkt: &str) -> PyResult<(Vec<u8>, Option<i32>)> {
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
/// For ``False`` and integer SRIDs, canonical little-endian Point,
/// LineString, and Polygon EWKB hex or bytes-like input is patched at the
/// top-level header without scanning the geometry body. Malformed coordinate
/// bodies or trailing bytes in those simple fast paths can pass through as
/// invalid output bytes.
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

/// Returns the SRID embedded in the top-level WKB/EWKB header, or ``None``
/// if no SRID is present.
///
/// ``source`` may be a bytes-like WKB/EWKB value or a hex-encoded WKB/EWKB
/// string.  For bytes-like input it is borrowed without copying.
///
/// For any byte order and any geometry type with only the three canonical EWKB
/// flag bits (Z, M, SRID) in the high type-word half, the SRID is read
/// directly from the 9-byte header without parsing the geometry body.  This
/// covers EWKB types 1–7, plain WKB, and ISO-dimensional WKB (which has no
/// SRID flag, so ``None`` is returned immediately without a full parse).
/// Inputs shorter than the 5-byte WKB header, inputs with unknown flag bits,
/// and inputs with an invalid byte-order marker fall back to a full parse.
///
/// Note: this function reads only the top-level header and does not validate
/// the base geometry type code.  An input whose header has valid EWKB flag bits
/// but an unsupported geometry type (e.g. type code 99) will have its SRID
/// returned without error; attempting to convert such bytes to WKT will still
/// fail.  This is intentional — extracting the SRID from a partially valid
/// header can be useful even when the full geometry is unrecognised.
#[pyfunction]
fn wkb_header_srid(source: Bound<'_, PyAny>) -> PyResult<Option<i32>> {
    if let Ok(text) = source.cast::<PyString>() {
        let bytes = core::decode_hex(text.to_str()?).map_err(to_py_err)?;
        return core::wkb_header_srid(&bytes).map_err(to_py_err);
    }
    with_wkb_buffer(&source, "source", |wkb| {
        core::wkb_header_srid(wkb).map_err(to_py_err)
    })
}

/// Strips the top-level SRID flag and SRID field from WKB/EWKB input.
///
/// ``source`` may be a bytes-like WKB/EWKB value or a hex-encoded WKB/EWKB
/// string.  The return type mirrors the input type: binary input returns
/// ``bytes``, hex-string input returns an uppercase hex string.
///
/// For canonical little-endian EWKB with Point, LineString, or Polygon type
/// codes, the header is rewritten without parsing the geometry body.
/// Collection types, big-endian inputs, and ISO-dimensional inputs fall back
/// to a full WKB→WKT→WKB round-trip which normalises the representation.
#[pyfunction]
fn to_wkb_no_srid_header(source: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let py = source.py();
    if let Ok(text) = source.cast::<PyString>() {
        let bytes = core::decode_hex(text.to_str()?).map_err(to_py_err)?;
        let result = core::wkb_strip_srid(&bytes).map_err(to_py_err)?;
        let hex = core::encode_hex(&result);
        return Ok(PyString::new(py, &hex).into_any().unbind());
    }
    with_wkb_buffer(&source, "source", |wkb| {
        let (len, write) = core::wkb_strip_srid_writer(wkb).map_err(to_py_err)?;
        PyBytes::new_with(py, len, |buf| {
            write(buf);
            Ok(())
        })
        .map(|b| b.into_any().unbind())
    })
}

/// Embeds or replaces the SRID in WKB/EWKB input.
///
/// ``source`` may be a bytes-like WKB/EWKB value or a hex-encoded WKB/EWKB
/// string.  The return type mirrors the input type: binary input returns
/// ``bytes``, hex-string input returns an uppercase hex string.
///
/// An existing embedded SRID is replaced rather than doubled.  Pass ``False``
/// to strip the SRID; pass an integer to embed or replace it.  ``True`` is
/// rejected to avoid ambiguity.  Integer SRID values ≤ 0 strip the SRID per
/// the PostGIS convention (``SRID_IS_UNKNOWN(x) ((int)x<=0)``), identical to
/// ``to_wkb_no_srid_header``.
///
/// For canonical little-endian EWKB with Point, LineString, or Polygon type
/// codes, the header is rewritten without parsing the geometry body.
/// Collection types, big-endian inputs, and ISO-dimensional inputs fall back
/// to a full WKB→WKT→WKB round-trip which normalises the representation.
#[pyfunction]
fn to_ewkb_header(source: Bound<'_, PyAny>, srid: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let py = source.py();
    let srid = parse_header_srid_arg(srid)?;
    if let Ok(text) = source.cast::<PyString>() {
        let bytes = core::decode_hex(text.to_str()?).map_err(to_py_err)?;
        let result = apply_header_srid_arg(&bytes, srid).map_err(to_py_err)?;
        let hex = core::encode_hex(&result);
        return Ok(PyString::new(py, &hex).into_any().unbind());
    }
    with_wkb_buffer(&source, "source", |wkb| {
        let (len, write) = match srid {
            ExplicitSridMode::Strip => core::wkb_strip_srid_writer(wkb),
            ExplicitSridMode::Set(n) => core::wkb_set_srid_writer(wkb, n),
        }
        .map_err(to_py_err)?;
        PyBytes::new_with(py, len, |buf| {
            write(buf);
            Ok(())
        })
        .map(|b| b.into_any().unbind())
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
    m.add_function(wrap_pyfunction!(wkb_header_srid, m)?)?;
    m.add_function(wrap_pyfunction!(to_wkb_no_srid_header, m)?)?;
    m.add_function(wrap_pyfunction!(to_ewkb_header, m)?)?;
    Ok(())
}
