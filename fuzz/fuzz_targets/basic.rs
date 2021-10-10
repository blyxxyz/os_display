#![no_main]
use libfuzzer_sys::fuzz_target;

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use os_display::Quotable;

const QUOTE_STARTS: &[char] = &['\'', '"', '$', '\\'];

fuzz_target!(|data: &[u8]| {
    let text = OsStr::from_bytes(data);
    let quoted = text.quote().to_string();
    let maybe_quoted = text.maybe_quote().to_string();
    assert!(quoted.starts_with(QUOTE_STARTS));
    if maybe_quoted.starts_with(QUOTE_STARTS) {
        assert_eq!(quoted, maybe_quoted);
    } else {
        let text = text.to_str().expect("should be valid unicode");
        assert_eq!(maybe_quoted, text);
        assert!(!text.chars().any(char::is_whitespace));
        assert!(!text.is_empty());
        assert!(text.width() != 0);
        assert!(text.chars().next().unwrap().width().unwrap_or(0) != 0);
    }
    for &case in &[&quoted, &maybe_quoted] {
        assert!(!case.chars().any(|ch| ch.is_ascii_control()), "{:?}", case);
        assert!(!case.contains('\n'), "{:?}", case);
    }
});
