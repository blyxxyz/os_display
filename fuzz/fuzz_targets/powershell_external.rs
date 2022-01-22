#![no_main]
use libfuzzer_sys::fuzz_target;

use std::process::Command;

use once_cell::sync::Lazy;

use os_display::Quoted;

mod common;

use common::Shell;

// This test assumes PSNativeCommandArgumentPassing is disabled.

// For some reason /usr/bin/printf is ~8 times as fast as printf.
// This is still 4 times slower than the internal method, but judging
// by the rates I get from other shells it's probably the maximum
// possible without parallelism.
const PWSH_SCRIPT: &str = r#"
foreach($line in [System.IO.File]::ReadLines("/dev/stdin")) {
    Invoke-Expression ("/usr/bin/printf '%s\0\n' {0}" -f $line)
}
"#;
static POWERSHELL: Lazy<Shell> =
    Lazy::new(|| Shell::raw(Command::new("pwsh").arg("-c").arg(PWSH_SCRIPT)));

fuzz_target!(|data: &[u8]| {
    // Can't pass null bytes
    let data = data.split(|b| *b == 0).next().unwrap();

    // PowerShell only speaks UTF-16, so we can't feed it invalid unicode.
    if let Ok(text) = std::str::from_utf8(data) {
        let quote = Quoted::windows(text).external(true).to_string();
        let maybe_quote = Quoted::windows(text)
            .external(true)
            .force(false)
            .to_string();
        assert_eq!(POWERSHELL.send(&quote), data, "{:?}", quote);
        assert_eq!(POWERSHELL.send(&maybe_quote), data, "{:?}", maybe_quote);
    }
});
