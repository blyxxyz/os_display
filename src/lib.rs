//! Utilities for printing paths, with special attention paid to special
//! characters and invalid unicode.
//!
//! For displaying paths in informational messages use `Quotable::quote`. This
//! will wrap quotes around the filename and add the necessary escapes to make
//! it copy/paste-able into a shell.
//!
//! # Examples
//! ```
//! # fn example() -> Result<(), std::io::Error> {
//! use std::path::Path;
//! use os_display::Quotable;
//!
//! let path = Path::new("foo/bar.baz");
//!
//! // Found file 'foo/bar.baz'
//! println!("Found file {}", path.quote());
//! // foo/bar.baz: Not found
//! println!("{}: Not found", path.maybe_quote());
//! # Ok(()) }
//! ```

use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(target_os = "wasi")]
use std::os::wasi::ffi::OsStrExt;

use unicode_width::UnicodeWidthChar;

/// An extension trait for displaying filenames to users.
pub trait Quotable {
    /// Returns an object that implements [`Display`] for printing filenames with
    /// proper quoting and escaping for the platform.
    ///
    /// On Unix this corresponds to bash/ksh syntax, on Windows Powershell syntax
    /// is used.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use os_display::Quotable;
    ///
    /// let path = Path::new("foo/bar.baz");
    ///
    /// println!("Found file {}", path.quote()); // Prints "Found file 'foo/bar.baz'"
    /// ```
    fn quote(&self) -> Quoted<'_>;

    /// Like `quote()`, but don't actually add quotes unless necessary because of
    /// whitespace or special characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use os_display::Quotable;
    ///
    /// let foo = Path::new("foo/bar.baz");
    /// let bar = Path::new("foo bar");
    ///
    /// println!("{}: Not found", foo.maybe_quote()); // Prints "foo/bar.baz: Not found"
    /// println!("{}: Not found", bar.maybe_quote()); // Prints "'foo bar': Not found"
    /// ```
    fn maybe_quote(&self) -> Quoted<'_> {
        let mut quoted = self.quote();
        quoted.force_quote = false;
        quoted
    }
}

impl<T: AsRef<OsStr>> Quotable for T {
    fn quote(&self) -> Quoted<'_> {
        Quoted {
            text: self.as_ref(),
            force_quote: true,
        }
    }
}

/// A wrapper around [`OsStr`] for printing paths with quoting and escaping applied.
#[derive(Debug, Copy, Clone)]
pub struct Quoted<'a> {
    text: &'a OsStr,
    force_quote: bool,
}

impl<'a> Display for Quoted<'a> {
    #[cfg(any(windows, unix, target_os = "wasi"))]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use std::fmt::Write;

        // On Unix we emulate sh syntax. On Windows Powershell.
        // They're just similar enough to share some code.

        /// Characters with special meaning outside quotes.
        // https://pubs.opengroup.org/onlinepubs/9699919799/utilities/V3_chap02.html#tag_18_02
        // I don't know why % is in there. GNU doesn't quote it either.
        // zsh and fish have trouble with standalone {}.
        // ^ was used for piping in old shells and GNU quotes it.
        #[cfg(any(unix, target_os = "wasi"))]
        const SPECIAL_SHELL_CHARS: &[u8] = b"|&;<>()$`\\\"'*?[]=^{} ";
        // I'm not too familiar with PowerShell, much of this is based on
        // experimentation rather than documentation or deep understanding.
        // I have noticed that ~?*[] only get expanded in some contexts, so watch
        // out for that if doing your own tests.
        // Get-ChildItem seems unwilling to quote anything so it doesn't help.
        // The omission of \ is important because it's used in file paths.
        #[cfg(windows)]
        const SPECIAL_SHELL_CHARS: &[u8] = b"|&;<>()$`\"'*?[]=,{} ";

        /// Characters with a special meaning at the beginning of a name.
        // ~ expands a home directory.
        // # starts a comment.
        // ! is a common extension for expanding the shell history.
        #[cfg(any(unix, target_os = "wasi"))]
        const SPECIAL_SHELL_CHARS_START: &[char] = &['~', '#', '!'];
        // Same deal as before, this is possibly incomplete.
        // A single stand-alone exclamation mark seems to have some special meaning.
        // Tildes are unclear: In Powershell on Linux, quoting a tilde keeps it from
        // expanding if passed to an external program, but not if passed to Get-ChildItem.
        #[cfg(windows)]
        const SPECIAL_SHELL_CHARS_START: &[char] = &['~', '#', '@', '!'];

        /// Characters that are interpreted specially in a double-quoted string.
        #[cfg(any(unix, target_os = "wasi"))]
        const DOUBLE_UNSAFE: &[u8] = &[b'"', b'`', b'$', b'\\'];
        #[cfg(windows)]
        const DOUBLE_UNSAFE: &[u8] = &[b'"', b'`', b'$'];

        let text = match self.text.to_str() {
            None => return write_escaped(f, self.text),
            Some(text) => text,
        };

        let mut is_single_safe = true;
        let mut is_double_safe = true;
        let mut requires_quote = self.force_quote;

        if !requires_quote {
            if let Some(first) = text.chars().next() {
                if SPECIAL_SHELL_CHARS_START.contains(&first) {
                    requires_quote = true;
                }

                // gnome-terminal (VTE), xterm, urxvt, tmux, screen, and VS Code's
                // builtin terminal all include zero-width characters at the end of the
                // selection but not at the start.
                // terminology and st seem to have trouble displaying them at all.
                // So if there's a zero-width character at the start we need quotes, but
                // if it's at the end we don't need to bother.
                // (This also ensures non-empty zero-width strings end up quoted.)
                if !requires_quote && first.width().unwrap_or(0) == 0 {
                    // .width() returns Some(1) for unassigned codepoints.
                    // This means we can't pre-emptively quote unknown codepoints in
                    // case they become zero-width in the future.
                    // (None is only returned for certain ASCII characters.)
                    requires_quote = true;
                }

                #[cfg(windows)]
                {
                    // Unlike in Unix, quoting an argument may stop it
                    // from being recognized as an option. I like that very much.
                    // But we don't want to quote "-" because that's a common
                    // special argument and PowerShell doesn't mind it.
                    if !requires_quote && first == '-' && text.len() > 1 {
                        requires_quote = true;
                    }
                }
            } else {
                // Empty string
                requires_quote = true;
            }
        }

        for ch in text.chars() {
            if ch.is_ascii() {
                let ch = ch as u8;
                if ch == b'\'' {
                    is_single_safe = false;
                }
                if is_double_safe && DOUBLE_UNSAFE.contains(&ch) {
                    is_double_safe = false;
                }
                if !requires_quote && SPECIAL_SHELL_CHARS.contains(&ch) {
                    requires_quote = true;
                }
                if ch.is_ascii_control() {
                    return write_escaped(f, self.text);
                }
            } else if !requires_quote && ch.is_whitespace() {
                // Powershell and yash both split on unicode whitespace characters.
                // Therefore we have to quote them.
                // Quoting them also has advantages for readability, but some codepoints
                // are blank yet not whitespace, like U+2800 BRAILLE PATTERN BLANK,
                // so this is not a complete solution.
                // This functionality goes stale: if new whitespace codepoints are
                // introduced the binary has to be recompiled.
                requires_quote = true;
            }
        }

        if !requires_quote {
            return f.write_str(text);
        } else if is_single_safe {
            return write_simple(f, text, '\'');
        } else if is_double_safe {
            return write_simple(f, text, '\"');
        } else {
            return write_single_escaped(f, text);
        }

        fn write_simple(f: &mut Formatter<'_>, text: &str, quote: char) -> fmt::Result {
            f.write_char(quote)?;
            f.write_str(text)?;
            f.write_char(quote)?;
            Ok(())
        }

        #[cfg(any(unix, target_os = "wasi"))]
        fn write_single_escaped(f: &mut Formatter<'_>, text: &str) -> fmt::Result {
            let mut iter = text.split('\'');
            if let Some(chunk) = iter.next() {
                if !chunk.is_empty() {
                    write_simple(f, chunk, '\'')?;
                }
            }
            for chunk in iter {
                f.write_str("\\'")?;
                if !chunk.is_empty() {
                    write_simple(f, chunk, '\'')?;
                }
            }
            Ok(())
        }

        /// Write using the syntax described here:
        /// https://www.gnu.org/software/bash/manual/html_node/ANSI_002dC-Quoting.html
        ///
        /// Supported by these shells:
        /// - bash
        /// - zsh
        /// - busybox sh
        /// - mksh
        /// - ksh93
        ///
        /// Not supported by these:
        /// - fish
        /// - dash
        /// - tcsh
        ///
        /// There's a proposal to add it to POSIX:
        /// https://www.austingroupbugs.net/view.php?id=249
        #[cfg(any(unix, target_os = "wasi"))]
        fn write_escaped(f: &mut Formatter<'_>, text: &OsStr) -> fmt::Result {
            f.write_str("$'")?;
            // ksh variants accept more than two digits for a \x escape code,
            // e.g. \xA691. We have to take care to not accidentally output
            // something like that. If necessary we interrupt the quoting with
            // `'$'`.
            let mut in_escape = false;
            for chunk in from_utf8_iter(text.as_bytes()) {
                match chunk {
                    Ok(chunk) => {
                        for ch in chunk.chars() {
                            let was_escape = in_escape;
                            in_escape = false;
                            match ch {
                                '\n' => f.write_str("\\n")?,
                                '\t' => f.write_str("\\t")?,
                                '\r' => f.write_str("\\r")?,
                                // We could do \a, \b, \f, \v, but those are
                                // rare enough to be confusing.
                                // \0 is actually a case of the octal \nnn syntax,
                                // and null bytes can't appear in arguments anyway,
                                // so let's stay clear of that.
                                // Some but not all shells have \e for \x1B.
                                ch if ch.is_ascii_control() => {
                                    write!(f, "\\x{:02X}", ch as u8)?;
                                    in_escape = true;
                                }
                                '\\' | '\'' => {
                                    // '?' and '"' can also be escaped this way
                                    // but AFAICT there's no reason to do so.
                                    f.write_char('\\')?;
                                    f.write_char(ch)?;
                                }
                                ch if was_escape && ch.is_ascii_hexdigit() => {
                                    f.write_str("'$'")?;
                                    f.write_char(ch)?;
                                }
                                ch => {
                                    f.write_char(ch)?;
                                }
                            }
                        }
                    }
                    Err(unit) => {
                        write!(f, "\\x{:02X}", unit)?;
                        in_escape = true;
                    }
                }
            }
            f.write_char('\'')?;
            Ok(())
        }

        #[cfg(windows)]
        fn write_single_escaped(f: &mut Formatter<'_>, text: &str) -> fmt::Result {
            // Quotes in Powershell can be escaped by doubling them
            f.write_char('\'')?;
            let mut iter = text.split('\'');
            if let Some(chunk) = iter.next() {
                f.write_str(chunk)?;
            }
            for chunk in iter {
                f.write_str("''")?;
                f.write_str(chunk)?;
            }
            f.write_char('\'')?;
            Ok(())
        }

        #[cfg(windows)]
        fn write_escaped(f: &mut Formatter<'_>, text: &OsStr) -> fmt::Result {
            // ` takes the role of \ since \ is already used as the path separator.
            // Things are UTF-16-oriented, so we escape code units as "`u{1234}".
            use std::char::decode_utf16;
            use std::os::windows::ffi::OsStrExt;

            f.write_char('"')?;
            for ch in decode_utf16(text.encode_wide()) {
                match ch {
                    Ok(ch) => match ch {
                        '\0' => f.write_str("`0")?,
                        '\r' => f.write_str("`r")?,
                        '\n' => f.write_str("`n")?,
                        '\t' => f.write_str("`t")?,
                        // Code unit escapes are only supported in PowerShell Core,
                        // so we're more willing to use weird escapes here than on Unix.
                        // There's also `e, for \x1B, but that one's Core-exclusive.
                        '\x07' => f.write_str("`a")?,
                        '\x08' => f.write_str("`b")?,
                        '\x0b' => f.write_str("`v")?,
                        '\x0c' => f.write_str("`f")?,
                        ch if ch.is_ascii_control() => write!(f, "`u{{{:02X}}}", ch as u8)?,
                        '`' => f.write_str("``")?,
                        '$' => f.write_str("`$")?,
                        '"' => f.write_str("\"\"")?,
                        ch => f.write_char(ch)?,
                    },
                    Err(err) => write!(f, "`u{{{:04X}}}", err.unpaired_surrogate())?,
                }
            }
            f.write_char('"')?;
            Ok(())
        }
    }

    #[cfg(not(any(unix, target_os = "wasi", windows)))]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // As a fallback, use Rust's own escaping syntax.
        // This is reasonably sane and very easy to implement.

        // Err on the side of caution: the only special character assumed to
        // be safe is . because it's essential in filenames.
        // Even _ is used as a wildcard in SQL so I don't trust it.
        fn is_special(ch: char) -> bool {
            !(ch.is_alphanumeric() || ch == '.')
        }

        match self.text.to_str() {
            Some(text) => {
                if self.force_quote
                    || text.is_empty()
                    || text.chars().next().and_then(char::width).unwrap_or(0) == 0
                    || text.chars().any(is_special)
                {
                    // We use single quotes because that's what the other versions are
                    // biased towards. This makes it safe to hardcode single quotes
                    // in tests.
                    // Unlike escape_default(), escape_debug() doesn't obfuscate
                    // unicode.
                    write!(f, "'{}'", text.escape_debug())
                } else {
                    f.write_str(text)
                }
            }
            None => {
                // Invalid unicode, this is the only way to avoid losing information.
                // (Unless the Debug impl is lossy, that has sometimes been the case
                // in the past: https://github.com/rust-lang/rust/issues/22766)
                // This uses double quotes, unlike the quoting above.
                write!(f, "{:?}", self.text)
            }
        }
    }
}

#[cfg(any(unix, target_os = "wasi"))]
fn from_utf8_iter(bytes: &[u8]) -> impl Iterator<Item = Result<&str, u8>> {
    struct Iter<'a> {
        bytes: &'a [u8],
    }

    impl<'a> Iterator for Iter<'a> {
        type Item = Result<&'a str, u8>;

        fn next(&mut self) -> Option<Self::Item> {
            use std::str::from_utf8;

            if self.bytes.is_empty() {
                return None;
            }
            match from_utf8(self.bytes) {
                Ok(text) => {
                    self.bytes = &[];
                    Some(Ok(text))
                }
                Err(err) if err.valid_up_to() == 0 => {
                    let res = self.bytes[0];
                    self.bytes = &self.bytes[1..];
                    Some(Err(res))
                }
                Err(err) => {
                    let (valid, rest) = self.bytes.split_at(err.valid_up_to());
                    self.bytes = rest;
                    Some(Ok(from_utf8(valid).unwrap()))
                }
            }
        }
    }

    Iter { bytes }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;

    fn verify_quote(cases: &[(impl Quotable, &str)]) {
        for (case, expected) in cases {
            assert_eq!(case.quote().to_string(), *expected);
        }
    }

    fn verify_maybe(cases: &[(impl Quotable, &str)]) {
        for (case, expected) in cases {
            assert_eq!(case.maybe_quote().to_string(), *expected);
        }
    }

    /// This should hold on any platform.
    #[test]
    fn test_basic() {
        verify_quote(&[
            ("foo", "'foo'"),
            ("", "''"),
            ("foo/bar.baz", "'foo/bar.baz'"),
        ]);
        verify_maybe(&[
            ("foo", "foo"),
            ("", "''"),
            ("foo bar", "'foo bar'"),
            ("$foo", "'$foo'"),
            ("-", "-"),
        ]);
    }

    #[cfg(any(unix, target_os = "wasi", windows))]
    #[test]
    fn test_common() {
        verify_maybe(&[
            ("a#b", "a#b"),
            ("#ab", "'#ab'"),
            ("a~b", "a~b"),
            ("!", "'!'"),
        ]);
    }

    #[cfg(any(unix, target_os = "wasi"))]
    #[test]
    fn test_unix() {
        verify_quote(&[
            ("can't", r#""can't""#),
            (r#"can'"t"#, r#"'can'\''"t'"#),
            (r#"can'$t"#, r#"'can'\''$t'"#),
            ("foo\nb\ta\r\\\0`r", r#"$'foo\nb\ta\r\\\x00`r'"#),
            ("foo\x02", r#"$'foo\x02'"#),
            (r#"'$''"#, r#"\''$'\'\'"#),
        ]);
        verify_quote(&[(OsStr::from_bytes(b"foo\xFF"), r#"$'foo\xFF'"#)]);
        verify_maybe(&[
            ("-x", "-x"),
            ("a,b", "a,b"),
            ("a\\b", "'a\\b'"),
            ("}", ("'}'")),
        ]);
    }

    #[cfg(windows)]
    #[test]
    fn test_windows() {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        verify_quote(&[
            (r#"foo\bar"#, r#"'foo\bar'"#),
            ("can't", r#""can't""#),
            (r#"can'"t"#, r#"'can''"t'"#),
            (r#"can'$t"#, r#"'can''$t'"#),
            ("foo\nb\ta\r\\\0`r", r#""foo`nb`ta`r\`0``r""#),
            ("foo\x02", r#""foo`u{02}""#),
            (r#"'$''"#, r#"'''$'''''"#),
        ]);
        verify_quote(&[(
            OsString::from_wide(&[b'x' as u16, 0xD800]),
            r#""x`u{D800}""#,
        )]);
        verify_maybe(&[
            ("-x", "'-x'"),
            ("a,b", "'a,b'"),
            ("a\\b", "a\\b"),
            ("}", "'}'"),
        ]);
    }

    #[cfg(any(unix, target_os = "wasi"))]
    #[test]
    fn test_utf8_iter() {
        type ByteStr = &'static [u8];
        type Chunk = Result<&'static str, u8>;
        const CASES: &[(ByteStr, &[Chunk])] = &[
            (b"", &[]),
            (b"hello", &[Ok("hello")]),
            // Immediately invalid
            (b"\xFF", &[Err(b'\xFF')]),
            // Incomplete UTF-8
            (b"\xC2", &[Err(b'\xC2')]),
            (b"\xF4\x8F", &[Err(b'\xF4'), Err(b'\x8F')]),
            (b"\xFF\xFF", &[Err(b'\xFF'), Err(b'\xFF')]),
            (b"hello\xC2", &[Ok("hello"), Err(b'\xC2')]),
            (b"\xFFhello", &[Err(b'\xFF'), Ok("hello")]),
            (b"\xFF\xC2hello", &[Err(b'\xFF'), Err(b'\xC2'), Ok("hello")]),
            (b"foo\xFFbar", &[Ok("foo"), Err(b'\xFF'), Ok("bar")]),
            (
                b"foo\xF4\x8Fbar",
                &[Ok("foo"), Err(b'\xF4'), Err(b'\x8F'), Ok("bar")],
            ),
            (
                b"foo\xFF\xC2bar",
                &[Ok("foo"), Err(b'\xFF'), Err(b'\xC2'), Ok("bar")],
            ),
        ];
        for &(case, expected) in CASES {
            assert_eq!(
                from_utf8_iter(case).collect::<Vec<_>>().as_slice(),
                expected
            );
        }
    }

    #[test]
    fn can_quote_types() {
        "foo".quote();
        String::from("foo").quote();
        OsStr::new("foo").quote();
        OsStr::new("foo").to_owned().quote();
        Cow::Borrowed("foo").as_ref().quote();
    }

    #[test]
    fn leading_zero_width() {
        verify_maybe(&[
            ("\u{200B}", "'\u{200B}'"),
            ("\u{200B}a", "'\u{200B}a'"),
            ("a\u{200B}", "a\u{200B}"),
        ]);
    }

    #[test]
    fn unicode_whitespace() {
        verify_maybe(&[("\u{2000}", "'\u{2000}'")]);
    }

    #[test]
    fn interrupted_hex_escapes() {
        verify_quote(&[("\x02AB", "$'\\x02'$'AB'"), ("\x02GH", "$'\\x02GH'")]);
    }
}
