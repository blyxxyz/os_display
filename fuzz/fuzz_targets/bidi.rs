#![no_main]
use libfuzzer_sys::fuzz_target;

use os_display::Quoted;

fn assert_bidi_safe(text: &str) {
    // The prefix stops unic_bidi from RTL-ing the context if there are
    // RTL codepoints inside text. Without the prefix the suffix may end up
    // at the start.

    // I think that for certain combinations of terminal, text, and context it
    // may be possible to mangle the context using only "normal" RTL codepoints.
    // With VTE it only seems possible if the context contains RTL codepoints,
    // but other terminals may follow different rules (e.g. Terminal.app).

    // The context mangling is unfortunate, but it's not nearly as dangerous as
    // a proper trojan-source attack (it's weak and conspicuous), and I think
    // it's flat-out impossible to prevent within this crate's constraints.
    // Escaping all RTL codepoints renders text unreadable, and adding LTR markers
    // means it can't be copied faithfully.

    let text = format!("a {} b", text);

    let info = unic_bidi::BidiInfo::new(&text, None);
    assert_eq!(info.paragraphs.len(), 1);
    let para = &info.paragraphs[0];
    let reordered = info.reorder_line(para, para.range.clone());

    assert!(reordered.ends_with('b'), "{:?} → {:?}", text, reordered);
    assert!(reordered.starts_with('a'));
}

const WEIRD_CHARS: &[char] = &[
    // Explicitly called out in the paper.
    '\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}', '\u{2066}', '\u{2067}', '\u{2068}',
    '\u{2069}',
    // Other bidi. Not as dangerous (and ~equivalent to certain ordinary characters), but can't hurt to test.
    '\u{061C}', '\u{200E}', '\u{200F}',
];

fuzz_target!(|data: &[u8]| {
    let mut owned = Vec::new();
    for ch in data {
        match *ch {
            b'a'..=b'l' => owned.extend(
                WEIRD_CHARS[(*ch - b'a') as usize]
                    .encode_utf8(&mut [0; 4])
                    .as_bytes(),
            ),
            _ => owned.push(*ch),
        }
    }
    let data = owned;
    let unix = Quoted::unix_raw(&data).force(false).to_string();
    assert_bidi_safe(&unix);
    if let Ok(text) = String::from_utf8(data) {
        let windows = Quoted::windows(&text).force(false).to_string();
        assert_bidi_safe(&windows);
    }
});
