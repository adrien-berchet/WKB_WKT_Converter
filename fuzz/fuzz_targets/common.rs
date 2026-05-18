#![allow(dead_code)]

use wkb_wkt_converter::{
    decode_hex, encode_hex, hex_wkb_to_wkt, to_hex_wkb, to_wkb, to_wkt, wkb_header_srid,
    wkb_set_srid, wkb_set_srid_writer, wkb_strip_srid, wkb_strip_srid_writer, wkb_to_wkt,
    wkb_to_wkt_split_srid, wkt_to_hex_wkb, wkt_to_wkb, wkt_to_wkb_split_srid, Input, SridMode,
    WkbWriter,
};

pub const MAX_WKB_INPUT: usize = 64 * 1024;
pub const MAX_TEXT_INPUT: usize = 64 * 1024;

pub fn srid_mode(selector: u8) -> SridMode {
    match selector % 4 {
        0 => SridMode::Auto,
        1 => SridMode::Strip,
        2 => SridMode::Set(4326),
        _ => SridMode::Set(-1),
    }
}

pub fn exercise_wkb_seed(data: &[u8]) {
    if let Some(hex) = data.strip_prefix(b"HEX:") {
        if let Ok(hex) = std::str::from_utf8(hex) {
            if let Ok(decoded) = decode_hex(hex.trim()) {
                exercise_wkb(&decoded);
            }
        }
    }

    exercise_wkb(data);
}

pub fn exercise_wkb(wkb: &[u8]) {
    if wkb.len() > MAX_WKB_INPUT {
        return;
    }

    let split = wkb_to_wkt_split_srid(wkb);
    let header_srid = wkb_header_srid(wkb);
    if let (Ok((plain_wkt, split_srid)), Ok(header_srid)) = (&split, &header_srid) {
        assert_eq!(*split_srid, *header_srid);
        assert_stable_wkt_roundtrip(plain_wkt);
    }

    for mode in [
        SridMode::Auto,
        SridMode::Strip,
        SridMode::Set(4326),
        SridMode::Set(-1),
    ] {
        if let Ok(wkt) = wkb_to_wkt(wkb, mode) {
            assert_stable_wkt_roundtrip(&wkt);
        }
        let _ = to_wkb(Input::Wkb(wkb), mode);
        if let Ok(wkt) = to_wkt(Input::Wkb(wkb), mode, true) {
            assert_stable_wkt_roundtrip(&wkt);
        }
        assert_hex_shape(to_hex_wkb(Input::Wkb(wkb), mode).as_deref());
    }

    assert_writer_matches(wkb_strip_srid(wkb), wkb_strip_srid_writer(wkb));
    for srid in [-1, 0, 1, 4326, i32::MAX] {
        assert_writer_matches(wkb_set_srid(wkb, srid), wkb_set_srid_writer(wkb, srid));
    }
}

pub fn exercise_wkt(text: &str) {
    if text.len() > MAX_TEXT_INPUT {
        return;
    }

    if let Ok(wkb) = wkt_to_wkb(text, SridMode::Auto) {
        let wkt = wkb_to_wkt(&wkb, SridMode::Auto).expect("freshly encoded WKB must parse");
        assert_stable_wkt_roundtrip(&wkt);

        if let Ok(hex) = wkt_to_hex_wkb(text, SridMode::Auto) {
            assert_eq!(hex, encode_hex(&wkb));
            assert_eq!(
                hex_wkb_to_wkt(&hex, SridMode::Auto).expect("freshly encoded hex must parse"),
                wkt
            );
        }
    }

    if let Ok((wkb, _srid)) = wkt_to_wkb_split_srid(text) {
        assert_eq!(
            wkb_header_srid(&wkb).expect("freshly encoded split WKB must expose header"),
            None
        );
        let (plain_wkt, parsed_srid) =
            wkb_to_wkt_split_srid(&wkb).expect("freshly encoded split WKB must parse");
        assert_eq!(parsed_srid, None);
        assert_stable_wkt_roundtrip(&plain_wkt);
    }

    for mode in [
        SridMode::Auto,
        SridMode::Strip,
        SridMode::Set(4326),
        SridMode::Set(-1),
    ] {
        if let Ok(wkb) = wkt_to_wkb(text, mode) {
            let wkt = wkb_to_wkt(&wkb, SridMode::Auto).expect("freshly encoded WKB must parse");
            assert_stable_wkt_roundtrip(&wkt);
        }
        if let Ok(wkt) = hex_wkb_to_wkt(text, mode) {
            assert_stable_wkt_roundtrip(&wkt);
        }
        assert_hex_shape(wkt_to_hex_wkb(text, mode).as_deref());
        exercise_generic_text(text, mode, true);
        exercise_generic_text(text, mode, false);
    }
}

pub fn exercise_generic_text(text: &str, mode: SridMode, normalize_wkt: bool) {
    if text.len() > MAX_TEXT_INPUT {
        return;
    }

    let _ = to_wkb(Input::Text(text), mode);
    if let Ok(wkt) = to_wkt(Input::Text(text), mode, normalize_wkt) {
        if normalize_wkt || decode_hex(text.trim()).is_ok() {
            assert_stable_wkt_roundtrip(&wkt);
        } else {
            assert_eq!(wkt, expected_unvalidated_text_to_wkt(text, mode));
        }
    }
    assert_hex_shape(to_hex_wkb(Input::Text(text), mode).as_deref());
}

pub fn exercise_generic_seed(data: &[u8]) {
    if let Ok(seed) = std::str::from_utf8(data) {
        if let Some((kind, mode, normalize_wkt, body)) = parse_generic_seed(seed) {
            match kind {
                GenericSeedKind::Text => exercise_generic_text(body, mode, normalize_wkt),
                GenericSeedKind::Wkb => {
                    let body = body.trim();
                    if let Some(hex) = body.strip_prefix("HEX:") {
                        if let Ok(wkb) = decode_hex(hex.trim()) {
                            exercise_generic_bytes(&wkb, mode, normalize_wkt);
                        }
                    } else {
                        exercise_generic_bytes(body.as_bytes(), mode, normalize_wkt);
                    }
                }
            }
            return;
        }

        if looks_like_text_seed(seed) {
            exercise_generic_text(seed, SridMode::Auto, true);
            exercise_generic_text(seed, SridMode::Auto, false);
        }
    }

    let Some((&control, rest)) = data.split_first() else {
        return;
    };
    let mode = srid_mode(control);
    let normalize_wkt = control & 0b0000_0100 != 0;

    if control & 0b0000_1000 == 0 {
        if rest.len() > MAX_TEXT_INPUT {
            return;
        }
        if let Ok(text) = std::str::from_utf8(rest) {
            exercise_generic_text(text, mode, normalize_wkt);
        }
    } else {
        exercise_generic_bytes(rest, mode, normalize_wkt);
    }
}

pub fn exercise_generic_bytes(bytes: &[u8], mode: SridMode, normalize_wkt: bool) {
    if bytes.len() > MAX_WKB_INPUT {
        return;
    }

    let _ = to_wkb(Input::Wkb(bytes), mode);
    if let Ok(wkt) = to_wkt(Input::Wkb(bytes), mode, normalize_wkt) {
        assert_stable_wkt_roundtrip(&wkt);
    }
    assert_hex_shape(to_hex_wkb(Input::Wkb(bytes), mode).as_deref());
}

#[derive(Clone, Copy)]
enum GenericSeedKind {
    Text,
    Wkb,
}

fn parse_generic_seed(seed: &str) -> Option<(GenericSeedKind, SridMode, bool, &str)> {
    let (header, body) = seed.split_once("\n\n")?;
    let mut kind = None;
    let mut mode = SridMode::Auto;
    let mut normalize_wkt = false;

    for line in header.lines() {
        let (key, value) = line.split_once('=')?;
        match (key.trim(), value.trim()) {
            ("kind", "text") => kind = Some(GenericSeedKind::Text),
            ("kind", "wkb") => kind = Some(GenericSeedKind::Wkb),
            ("srid", value) => mode = parse_seed_srid_mode(value)?,
            ("normalize", "true") => normalize_wkt = true,
            ("normalize", "false") => normalize_wkt = false,
            _ => return None,
        }
    }

    Some((kind?, mode, normalize_wkt, body))
}

fn parse_seed_srid_mode(value: &str) -> Option<SridMode> {
    match value {
        "auto" => Some(SridMode::Auto),
        "strip" => Some(SridMode::Strip),
        _ => value
            .strip_prefix("set:")
            .and_then(|srid| srid.parse::<i32>().ok())
            .map(SridMode::Set),
    }
}

fn looks_like_text_seed(seed: &str) -> bool {
    let trimmed = seed.trim();
    if decode_hex(trimmed).is_ok() {
        return true;
    }

    let start = seed.trim_start();
    [
        "SRID=",
        "POINT",
        "LINESTRING",
        "POLYGON",
        "MULTIPOINT",
        "MULTILINESTRING",
        "MULTIPOLYGON",
        "GEOMETRYCOLLECTION",
    ]
    .into_iter()
    .any(|keyword| {
        start
            .get(..keyword.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(keyword))
    })
}

fn expected_unvalidated_text_to_wkt(text: &str, mode: SridMode) -> String {
    let trimmed = text.trim();
    match normalize_srid_mode(mode) {
        SridMode::Auto => strip_unknown_srid_prefix(trimmed).to_owned(),
        SridMode::Strip => strip_ewkt_prefix(trimmed).to_owned(),
        SridMode::Set(srid) => format!("SRID={srid};{}", strip_ewkt_prefix(trimmed)),
    }
}

fn normalize_srid_mode(mode: SridMode) -> SridMode {
    match mode {
        SridMode::Set(srid) if srid <= 0 => SridMode::Strip,
        other => other,
    }
}

fn strip_unknown_srid_prefix(wkt: &str) -> &str {
    if wkt
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("SRID="))
    {
        if let Some(relative_semicolon) = wkt[5..].find(';') {
            if let Ok(srid) = wkt[5..5 + relative_semicolon].trim().parse::<i32>() {
                if srid <= 0 {
                    return wkt[6 + relative_semicolon..].trim_start();
                }
            }
        }
    }
    wkt
}

fn strip_ewkt_prefix(wkt: &str) -> &str {
    if wkt
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("SRID="))
    {
        if let Some(relative_semicolon) = wkt[5..].find(';') {
            return wkt[6 + relative_semicolon..].trim_start();
        }
    }
    wkt
}

fn assert_writer_matches(
    expected: wkb_wkt_converter::Result<Vec<u8>>,
    writer_result: wkb_wkt_converter::Result<(usize, WkbWriter<'_>)>,
) {
    match (expected, writer_result) {
        (Ok(expected), Ok((len, writer))) => {
            assert_eq!(len, expected.len());
            let mut out = vec![0; len];
            writer.write_into(&mut out);
            assert_eq!(out, expected);
        }
        (Err(_), Err(_)) => {}
        (Ok(_), Err(err)) => panic!("writer rejected input accepted by allocating API: {err:?}"),
        (Err(err), Ok(_)) => panic!("writer accepted input rejected by allocating API: {err:?}"),
    }
}

pub fn assert_stable_wkt_roundtrip(wkt: &str) {
    if !is_parseable_wkt_output(wkt) {
        return;
    }

    if let Ok(wkb) = wkt_to_wkb(wkt, SridMode::Auto) {
        let reparsed = wkb_to_wkt(&wkb, SridMode::Auto)
            .expect("WKT accepted by encoder must produce parseable WKB");
        assert_eq!(reparsed, wkt);
    }
}

fn is_parseable_wkt_output(wkt: &str) -> bool {
    !wkt.contains("NaN") && !wkt.contains("inf")
}

fn assert_hex_shape(hex: Result<&str, &wkb_wkt_converter::Error>) {
    if let Ok(hex) = hex {
        assert_eq!(hex.len() % 2, 0);
        assert!(hex
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'A'..=b'F')));
    }
}
