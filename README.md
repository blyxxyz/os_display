# `os_display`

[![Crates.io](https://img.shields.io/crates/v/os_display.svg)](https://crates.io/crates/os_display)
[![API reference](https://docs.rs/os_display/badge.svg)](https://docs.rs/os_display/)
[![MSRV](https://img.shields.io/badge/MSRV-1.31-blue)](https://blog.rust-lang.org/2018/12/06/Rust-1.31-and-rust-2018.html)
[![CI](https://img.shields.io/github/workflow/status/blyxxyz/os_display/CI/master)](https://github.com/blyxxyz/os_display/actions)

Printing filenames can be tricky. They may contain control codes that mess up the message or even the whole terminal. They may not be safe to use in a command without quoting or escaping. They may also contain invalid unicode.

This library lets you add quoting to filenames (and other strings) to display them more safely and usefully. The goal is to render them in such a way that they can always be copied and pasted back into a shell without information loss.

On Unix values are quoted using bash/ksh syntax, while on Windows PowerShell syntax is used. Other platforms currently default to the Unix style.

## When should I use this?

This library is best suited for command line programs that deal with arbitrary filenames or other "dirty" text. `mv` for example is the very tool you use to rename files with problematic names, so it's nice if its messages handle them well.

Programs that aren't expected to deal with weird data don't get as much benefit.

The output is made for shells, so displaying it in e.g. a GUI may not make sense.

Most programs get along fine without this. You likely don't strictly need it, but you may find it a nice improvement.

## Usage
Import the `Quotable` trait:

```rust
use os_display::Quotable;
```

This adds two methods to the common string types (including `OsStr`): `.quote()` and `.maybe_quote()`. They return `Quoted`, a wrapper with a custom `Display` implementation.

`.quote()` always puts quotes around the text:

```rust
// Found file 'filename'
println!("Found file {}", "filename".quote());

// Found file "foo'bar"
println!("Found file {}", "foo'bar".quote());

// Unix: Found file $'foo\nbar'
// Windows: Found file "foo`nbar"
println!("Found file {}", "ab\ncd".quote());
```

`.maybe_quote()` only adds them if necessary because of whitespace or special characters:

```rust
// filename: Not found
println!("{}: Not found", "filename".maybe_quote());

// 'foo bar': Not found
println!("{}: Not found", "foo bar".maybe_quote());

// '*?$': Not found
println!("{}: Not found", "*?$".maybe_quote());
```

`.quote()` is best used inside longer sentences while `.maybe_quote()` can be used for text that's already separated some other way (like by a colon).

## Limitations
- Unicode may be quoted but is not escaped. The printed text can still look weird, and a few (buggy) terminals may drop certain characters.
- This library should **not** be used to interpolate text into shell scripts. It's designed for readability, not absolute safety. Consider using the [`shell-escape`](https://crates.io/crates/shell-escape) crate instead (or ideally, passing in the values in some other way).
- The output is not compatible with every single shell.
- [PowerShell treats quotes differently in arguments to external commands](https://stackoverflow.com/questions/6714165). This library is tuned for arguments to internal commandlets.

## Invalid unicode
On Unix:

```rust
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

// \xFF makes this invalid UTF-8, so to_str() would fail
let bad_string = OsStr::from_bytes(b"bar\xFFqux");
assert_eq!(bad_string.quote().to_string(), "$'bar\xFFqux'");
```

On Windows:

```rust
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

// 0xD800 is an unpaired surrogate, making this invalid UTF-16
let bad_string = OsString::from_wide(&[b'a' as u16, 0xD800, b'b' as u16]);
assert_eq!(bad_string.quote().to_string(), r#""a`u{D800}b""#)
```

## Zero-width unicode
Some codepoints are zero-width. They can make a string invisible, or they can make it hard to select. GNU tools struggle with this:

```console
$ wc $'\u200b'
wc: ​: No such file or directory
```

`os_display` adds quotes in such cases:

```rust
assert_eq!("\u{200B}".maybe_quote().to_string(), "'\u{200B}'");
```

It still misleadingly looks like `''` when printed, but it's possible to copy and paste it and get the right result.

## Cross-platform usage
`Quoted` has constructors for specific platforms. `Quoted::unix("some string")` will quote with bash/ksh syntax no matter the platform, and `Quoted::windows("etc")` uses PowerShell syntax.

`Quoted::unix_raw` and `Quoted::windows_raw` take `&[u8]` (for malformed UTF-8) and `&[u16]` (for malformed UTF-16), respectively.

`Quoted::native(&str)` and `Quoted::native_raw(&OsStr)` can be used as an alternative to the extension trait if you prefer boring monomorphic functions.

## `no_std`
This crate is `no_std`-compatible if the `alloc` and/or `std` features are disabled.

## Testing
The Unix implementation has been [fuzzed](https://github.com/rust-fuzz/cargo-fuzz) against bash, zsh, mksh, ksh93 and busybox to ensure all output is interpreted back as the original string. It has been fuzzed to a more limited extent against fish, dash, tcsh, posh, and yash (which don't support all of the required syntax).

The PowerShell implementation has been fuzzed against PowerShell Core 7.1.4 running on Linux.

## Acknowledgments
This library is modeled after the quoting done by [Gnulib](https://www.gnu.org/software/gnulib/) as seen in the GNU coreutils. The behavior is not identical, however:
- GNU uses octal escapes, like `\377` instead of `\xFF`.
- GNU eagerly switches quoting style midway through, like `''$'\n''xyz'` instead of `$'\nxyz'`. `os_display` avoids this unless necessary.
- GNU escapes unassigned codepoints instead of leaving their handling up to the terminal.
- GNU doesn't handle zero-width codepoints specially.

The first version of this code was written for the [uutils project](https://github.com/uutils/coreutils). The feedback and the opportunity to use it in a large codebase were helpful.
