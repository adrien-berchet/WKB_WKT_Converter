#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() > common::MAX_TEXT_INPUT {
        return;
    }
    if let Ok(text) = std::str::from_utf8(data) {
        common::exercise_wkt(text);
    }
});
