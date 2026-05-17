#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;
use wkb_wkt_converter::{wkb_header_srid, wkb_to_wkt, wkb_to_wkt_split_srid, wkt_to_wkb, SridMode};

const DEPTH_LIMIT: usize = 128;
const MAX_TARGET_DEPTH: usize = DEPTH_LIMIT + 8;
const EWKB_SRID: u32 = 0x2000_0000;

fuzz_target!(|data: &[u8]| {
    let Some(case) = NestingCase::from_seed(data) else {
        return;
    };

    let wkt = nested_wkt(case.depth, case.with_srid);
    let wkb = nested_wkb(case.depth, case.with_srid);

    if case.depth < DEPTH_LIMIT {
        let encoded = wkt_to_wkb(&wkt, SridMode::Auto).expect("near-limit WKT should encode");
        let decoded = wkb_to_wkt(&wkb, SridMode::Auto).expect("near-limit WKB should decode");
        common::assert_stable_wkt_roundtrip(&decoded);

        let (_, encoded_srid) =
            wkb_to_wkt_split_srid(&encoded).expect("encoded near-limit WKB should split");
        let (_, decoded_srid) =
            wkb_to_wkt_split_srid(&wkb).expect("seed near-limit WKB should split");
        let expected_srid = case.with_srid.then_some(4326);
        assert_eq!(encoded_srid, expected_srid);
        assert_eq!(decoded_srid, expected_srid);
        assert_eq!(
            wkb_header_srid(&wkb).expect("seed WKB header should parse"),
            expected_srid
        );
    } else {
        assert!(wkt_to_wkb(&wkt, SridMode::Auto).is_err());
        assert!(wkb_to_wkt(&wkb, SridMode::Auto).is_err());
    }
});

struct NestingCase {
    depth: usize,
    with_srid: bool,
}

impl NestingCase {
    fn from_seed(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }

        if let Ok(text) = std::str::from_utf8(data) {
            if let Some(depth) = parse_depth_seed(text) {
                return Some(Self {
                    depth: depth.min(MAX_TARGET_DEPTH),
                    with_srid: text.lines().any(|line| line.trim() == "SRID"),
                });
            }
        }

        Some(Self {
            depth: data[0] as usize % (MAX_TARGET_DEPTH + 1),
            with_srid: data.get(1).is_some_and(|byte| byte & 1 == 1),
        })
    }
}

fn parse_depth_seed(text: &str) -> Option<usize> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix("DEPTH:"))
        .and_then(|depth| depth.trim().parse().ok())
}

fn nested_wkt(depth: usize, with_srid: bool) -> String {
    let mut out = String::new();
    if with_srid {
        out.push_str("SRID=4326;");
    }
    for _ in 0..depth {
        out.push_str("GEOMETRYCOLLECTION (");
    }
    out.push_str("POINT (1 2)");
    for _ in 0..depth {
        out.push(')');
    }
    out
}

fn nested_wkb(depth: usize, with_srid: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(25 + depth * 9);
    if depth == 0 {
        write_header(&mut out, 1, with_srid);
        write_point_body(&mut out);
        return out;
    }

    for collection_index in 0..depth {
        write_header(&mut out, 7, with_srid && collection_index == 0);
        out.extend_from_slice(&1u32.to_le_bytes());
    }
    write_header(&mut out, 1, false);
    write_point_body(&mut out);
    out
}

fn write_header(out: &mut Vec<u8>, geom_type: u32, with_srid: bool) {
    out.push(1);
    let type_word = geom_type | if with_srid { EWKB_SRID } else { 0 };
    out.extend_from_slice(&type_word.to_le_bytes());
    if with_srid {
        out.extend_from_slice(&4326i32.to_le_bytes());
    }
}

fn write_point_body(out: &mut Vec<u8>) {
    out.extend_from_slice(&1.0f64.to_le_bytes());
    out.extend_from_slice(&2.0f64.to_le_bytes());
}
