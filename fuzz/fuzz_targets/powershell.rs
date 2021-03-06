#![no_main]
use libfuzzer_sys::fuzz_target;

use std::process::Command;

use once_cell::sync::Lazy;

use os_display::Quoted;

mod common;

use common::Shell;

// This code only runs on Linux for various reasons.
// Grab PowerShell from https://github.com/PowerShell/PowerShell.

// Set-Variable is used because `$s = ...` sees a bare string as a command.
const PWSH_SCRIPT: &str = r#"
foreach($line in [System.IO.File]::ReadLines("/dev/stdin")) {
    Invoke-Expression ("Set-Variable s {0}" -f $line)
    "{0}`0" -f $s
}
"#;
static POWERSHELL: Lazy<Shell> =
    Lazy::new(|| Shell::raw(Command::new("pwsh").arg("-c").arg(PWSH_SCRIPT)));

fuzz_target!(|data: &[u8]| {
    // Can't pass null bytes
    let data = data.split(|b| *b == 0).next().unwrap();

    if data.contains(&b'\x1B') {
        // PowerShell 7.2 strips out certain ANSI command codes, at least
        // `e^, `e_, and all `e followed by a capital letter.
        // It doesn't always do this, the invocation in powershell_external.rs
        // happens to bypass it.
        // See: https://github.com/PowerShell/PowerShell/issues/16493
        // We don't treat \x1B/`e very specially. So it's ok to just ignore these.
        return;
    }

    // PowerShell only speaks UTF-16, so we can't feed it invalid unicode.
    if let Ok(text) = std::str::from_utf8(data) {
        let quote = Quoted::windows(text).external(false).to_string();
        let maybe_quote = Quoted::windows(text)
            .external(false)
            .force(false)
            .to_string();
        assert_eq!(POWERSHELL.send(&quote), data, "{:?}", quote);
        assert_eq!(POWERSHELL.send(&maybe_quote), data, "{:?}", maybe_quote);
    }
});
