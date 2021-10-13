//! Formatters for printing filenames and other strings in a terminal, with
//! attention paid to special characters and invalid unicode.
//!
//! They will wrap quotes around them and add the necessary escapes to make
//! them copy/paste-able into a shell.
//!
//! The [`Quotable`] trait adds `quote` and `maybe_quote` methods to string
//! types. The [`Quoted`] type has constructors for more explicit control.
//!
//! # Examples
//! ```
//! # fn example() -> Result<(), std::io::Error> {
//! use std::path::Path;
//! use os_display::{Quotable, Quoted};
//!
//! let path = Path::new("foo/bar.baz");
//!
//! // Found file 'foo/bar.baz'
//! println!("Found file {}", path.quote());
//! // foo/bar.baz: Not found
//! println!("{}: Not found", path.maybe_quote());
//! // "foo`nbar"
//! println!("{}", Quoted::windows("foo\nbar").force(false));
//! # Ok(()) }
//! ```

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use core::fmt::{self, Display, Formatter};

#[cfg(feature = "std")]
extern crate std;

// alloc was unstable in 1.31, so do some shuffling to avoid it unless necessary.
// 1.31 works with no features and with all features.
// 1.36 is the minimum version that supports alloc without std.
#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc;

#[cfg(feature = "windows")]
#[cfg(feature = "std")]
use std as alloc;

#[cfg(feature = "native")]
#[cfg(feature = "std")]
use std::{ffi::OsStr, path::Path};

#[cfg(any(feature = "unix", all(feature = "native", not(windows))))]
mod unix;
#[cfg(any(feature = "windows", all(feature = "native", windows)))]
mod windows;

/// A wrapper around string types for displaying with quoting and escaping applied.
#[derive(Debug, Copy, Clone)]
pub struct Quoted<'a> {
    source: Kind<'a>,
    force_quote: bool,
}

#[derive(Debug, Copy, Clone)]
enum Kind<'a> {
    #[cfg(any(feature = "unix", all(feature = "native", not(windows))))]
    Unix(&'a str),
    #[cfg(feature = "unix")]
    UnixRaw(&'a [u8]),
    #[cfg(any(feature = "windows", all(feature = "native", windows)))]
    Windows(&'a str),
    #[cfg(feature = "windows")]
    #[cfg(feature = "alloc")]
    WindowsRaw(&'a [u16]),
    #[cfg(feature = "native")]
    #[cfg(feature = "std")]
    NativeRaw(&'a std::ffi::OsStr),
}

impl<'a> Quoted<'a> {
    fn new(source: Kind<'a>) -> Self {
        Quoted {
            source,
            force_quote: true,
        }
    }

    /// Quote a string with the default style for the platform.
    ///
    /// On Windows this is PowerShell syntax, on all other platforms this is
    /// bash/ksh syntax.
    #[cfg(feature = "native")]
    pub fn native(text: &'a str) -> Self {
        #[cfg(windows)]
        return Quoted::new(Kind::Windows(text));
        #[cfg(not(windows))]
        return Quoted::new(Kind::Unix(text));
    }

    /// Quote an `OsStr` with the default style for the platform.
    ///
    /// On platforms other than Windows, Unix and WASI, if the encoding is
    /// invalid, the `Debug` representation will be used.
    #[cfg(feature = "native")]
    #[cfg(feature = "std")]
    pub fn native_raw(text: &'a OsStr) -> Self {
        Quoted::new(Kind::NativeRaw(text))
    }

    /// Quote a string using bash/ksh syntax.
    ///
    /// This requires the optional `unix` feature.
    #[cfg(feature = "unix")]
    pub fn unix(text: &'a str) -> Self {
        Quoted::new(Kind::Unix(text))
    }

    /// Quote possibly invalid UTF-8 using bash/ksh syntax.
    ///
    /// This requires the optional `unix` feature.
    #[cfg(feature = "unix")]
    pub fn unix_raw(bytes: &'a [u8]) -> Self {
        Quoted::new(Kind::UnixRaw(bytes))
    }

    /// Quote a string using PowerShell syntax.
    ///
    /// This requires the optional `windows` feature.
    #[cfg(feature = "windows")]
    pub fn windows(text: &'a str) -> Self {
        Quoted::new(Kind::Windows(text))
    }

    /// Quote possibly invalid UTF-16 using PowerShell syntax.
    ///
    /// This requires the optional `windows` feature.
    ///
    /// It also requires the (default) `alloc` feature.
    #[cfg(feature = "windows")]
    #[cfg(feature = "alloc")]
    pub fn windows_raw(units: &'a [u16]) -> Self {
        Quoted::new(Kind::WindowsRaw(units))
    }

    /// Toggle forced quoting. If `true`, quotes are added even if no special
    /// characters are present.
    ///
    /// Defaults to `true`.
    pub fn force(mut self, force: bool) -> Self {
        self.force_quote = force;
        self
    }
}

impl<'a> Display for Quoted<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.source {
            #[cfg(feature = "native")]
            #[cfg(feature = "std")]
            Kind::NativeRaw(text) => {
                #[cfg(unix)]
                use std::os::unix::ffi::OsStrExt;
                #[cfg(target_os = "wasi")]
                use std::os::wasi::ffi::OsStrExt;
                #[cfg(windows)]
                use std::os::windows::ffi::OsStrExt;

                #[cfg(windows)]
                match text.to_str() {
                    Some(text) => windows::write(f, text, self.force_quote),
                    None => windows::write_escaped(f, decode_utf16(text.encode_wide())),
                }
                #[cfg(any(unix, target_os = "wasi"))]
                match text.to_str() {
                    Some(text) => unix::write(f, text, self.force_quote),
                    None => unix::write_escaped(f, text.as_bytes()),
                }
                #[cfg(not(any(windows, unix, target_os = "wasi")))]
                match text.to_str() {
                    Some(text) => unix::write(f, text, self.force_quote),
                    // Debug is our best shot for not losing information.
                    // But you probably can't paste it into a shell.
                    None => write!(f, "{:?}", text),
                }
            }

            #[cfg(any(feature = "unix", all(feature = "native", not(windows))))]
            Kind::Unix(text) => unix::write(f, text, self.force_quote),

            #[cfg(feature = "unix")]
            Kind::UnixRaw(bytes) => match core::str::from_utf8(bytes) {
                Ok(text) => unix::write(f, text, self.force_quote),
                Err(_) => unix::write_escaped(f, bytes),
            },

            #[cfg(any(feature = "windows", all(feature = "native", windows)))]
            Kind::Windows(text) => windows::write(f, text, self.force_quote),

            #[cfg(feature = "windows")]
            #[cfg(feature = "alloc")]
            // Avoiding this allocation is possible in theory, but it'd require either
            // complicating or slowing down the common case.
            // Perhaps we could offer a non-allocating API for known-invalid UTF-16 strings
            // that we pass straight to write_escaped(), but it seems a bit awkward.
            // Please open an issue if you have a need for this.
            Kind::WindowsRaw(units) => match alloc::string::String::from_utf16(units) {
                Ok(text) => windows::write(f, &text, self.force_quote),
                Err(_) => windows::write_escaped(f, decode_utf16(units.iter().cloned())),
            },
        }
    }
}

#[cfg(any(feature = "windows", all(feature = "native", feature = "std", windows)))]
#[cfg(feature = "alloc")]
fn decode_utf16(units: impl IntoIterator<Item = u16>) -> impl Iterator<Item = Result<char, u16>> {
    core::char::decode_utf16(units).map(|res| res.map_err(|err| err.unpaired_surrogate()))
}

#[cfg(feature = "native")]
mod native {
    use super::*;

    /// An extension trait to apply quoting to strings.
    ///
    /// This is implemented on `str`, `OsStr` and `Path`.
    ///
    /// For finer control, see the constructors on [`Quoted`].
    pub trait Quotable {
        /// Returns an object that implements [`Display`] for printing strings with
        /// proper quoting and escaping for the platform.
        ///
        /// On Unix this corresponds to bash/ksh syntax, on Windows PowerShell syntax
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
        /// let bar = "foo bar";
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

    impl Quotable for str {
        fn quote(&self) -> Quoted<'_> {
            Quoted::native(self)
        }
    }

    #[cfg(feature = "std")]
    impl Quotable for OsStr {
        fn quote(&self) -> Quoted<'_> {
            Quoted::native_raw(self)
        }
    }

    #[cfg(feature = "std")]
    impl Quotable for Path {
        fn quote(&self) -> Quoted<'_> {
            Quoted::native_raw(self.as_ref())
        }
    }

    impl<'a, T: Quotable + ?Sized> From<&'a T> for Quoted<'a> {
        fn from(val: &'a T) -> Self {
            val.quote()
        }
    }
}

#[cfg(feature = "native")]
pub use crate::native::Quotable;

#[cfg(feature = "std")]
#[cfg(test)]
mod tests {
    #![allow(unused)]

    use super::*;

    use std::string::ToString;

    const BOTH_ALWAYS: &[(&str, &str)] = &[
        ("foo", "'foo'"),
        ("", "''"),
        ("foo/bar.baz", "'foo/bar.baz'"),
        ("can't", r#""can't""#),
    ];
    const BOTH_MAYBE: &[(&str, &str)] = &[
        ("foo", "foo"),
        ("", "''"),
        ("foo bar", "'foo bar'"),
        ("$foo", "'$foo'"),
        ("-", "-"),
        ("a#b", "a#b"),
        ("#ab", "'#ab'"),
        ("a~b", "a~b"),
        ("!", "'!'"),
        ("}", ("'}'")),
        ("\u{200B}", "'\u{200B}'"),
        ("\u{200B}a", "'\u{200B}a'"),
        ("a\u{200B}", "a\u{200B}"),
        ("\u{2000}", "'\u{2000}'"),
    ];

    const UNIX_ALWAYS: &[(&str, &str)] = &[
        (r#"can'"t"#, r#"'can'\''"t'"#),
        (r#"can'$t"#, r#"'can'\''$t'"#),
        ("foo\nb\ta\r\\\0`r", r#"$'foo\nb\ta\r\\\x00`r'"#),
        ("foo\x02", r#"$'foo\x02'"#),
        (r#"'$''"#, r#"\''$'\'\'"#),
    ];
    const UNIX_MAYBE: &[(&str, &str)] = &[
        ("-x", "-x"),
        ("a,b", "a,b"),
        ("a\\b", "'a\\b'"),
        ("\x02AB", "$'\\x02'$'AB'"),
        ("\x02GH", "$'\\x02GH'"),
        ("\t", r#"$'\t'"#),
        ("\r", r#"$'\r'"#),
    ];
    const UNIX_RAW: &[(&[u8], &str)] = &[
        (b"foo\xFF", r#"$'foo\xFF'"#),
        (b"foo\xFFbar", r#"$'foo\xFF'$'bar'"#),
    ];

    #[cfg(feature = "unix")]
    #[test]
    fn unix() {
        for &(orig, expected) in UNIX_ALWAYS.iter().chain(BOTH_ALWAYS) {
            assert_eq!(Quoted::unix(orig).to_string(), expected);
        }
        for &(orig, expected) in UNIX_MAYBE.iter().chain(BOTH_MAYBE) {
            assert_eq!(Quoted::unix(orig).force(false).to_string(), expected);
        }
        for &(orig, expected) in UNIX_RAW {
            assert_eq!(Quoted::unix_raw(orig).to_string(), expected);
        }
    }

    const WINDOWS_ALWAYS: &[(&str, &str)] = &[
        (r#"foo\bar"#, r#"'foo\bar'"#),
        (r#"can'"t"#, r#"'can''"t'"#),
        (r#"can'$t"#, r#"'can''$t'"#),
        ("foo\nb\ta\r\\\0`r", r#""foo`nb`ta`r\`0``r""#),
        ("foo\x02", r#""foo`u{02}""#),
        (r#"'$''"#, r#"'''$'''''"#),
    ];
    const WINDOWS_MAYBE: &[(&str, &str)] = &[
        ("-x", "'-x'"),
        ("—x", "'—x'"),
        ("a,b", "'a,b'"),
        ("a\\b", "a\\b"),
        ("‘", r#""‘""#),
        ("‘\"", r#"''‘"'"#),
        ("„\0", r#""`„`0""#),
        ("\t", r#""`t""#),
        ("\r", r#""`r""#),
    ];
    const WINDOWS_RAW: &[(&[u16], &str)] = &[(&[b'x' as u16, 0xD800], r#""x`u{D800}""#)];

    #[cfg(feature = "windows")]
    #[test]
    fn windows() {
        for &(orig, expected) in WINDOWS_ALWAYS.iter().chain(BOTH_ALWAYS) {
            assert_eq!(Quoted::windows(orig).to_string(), expected);
        }
        for &(orig, expected) in WINDOWS_MAYBE.iter().chain(BOTH_MAYBE) {
            assert_eq!(Quoted::windows(orig).force(false).to_string(), expected);
        }
        for &(orig, expected) in WINDOWS_RAW {
            assert_eq!(Quoted::windows_raw(orig).to_string(), expected);
        }
    }

    #[cfg(feature = "native")]
    #[cfg(windows)]
    #[test]
    fn native() {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        assert_eq!("'\"".quote().to_string(), r#"'''"'"#);
        assert_eq!("x\0".quote().to_string(), r#""x`0""#);
        assert_eq!(
            OsString::from_wide(&[b'x' as u16, 0xD800])
                .quote()
                .to_string(),
            r#""x`u{D800}""#
        );
    }

    #[cfg(feature = "native")]
    #[cfg(any(unix, target_os = "wasi"))]
    #[test]
    fn native() {
        #[cfg(unix)]
        use std::os::unix::ffi::OsStrExt;
        #[cfg(target_os = "wasi")]
        use std::os::wasi::ffi::OsStrExt;

        assert_eq!("'\"".quote().to_string(), r#"\''"'"#);
        assert_eq!("x\0".quote().to_string(), r#"$'x\x00'"#);
        assert_eq!(
            OsStr::from_bytes(b"x\xFF").quote().to_string(),
            r#"$'x\xFF'"#
        );
    }

    #[cfg(feature = "native")]
    #[cfg(not(any(windows, unix, target_os = "wasi")))]
    #[test]
    fn native() {
        assert_eq!("'\"".quote().to_string(), r#"\''"'"#);
        assert_eq!("x\0".quote().to_string(), r#"$'x\x00'"#);
    }

    #[cfg(feature = "native")]
    #[test]
    fn can_quote_types() {
        use std::borrow::{Cow, ToOwned};

        "foo".quote();
        "foo".to_owned().quote();
        Cow::Borrowed("foo").quote();

        OsStr::new("foo").quote();
        OsStr::new("foo").to_owned().quote();
        Cow::Borrowed(OsStr::new("foo")).quote();

        Path::new("foo").quote();
        Path::new("foo").to_owned().quote();
        Cow::Borrowed(Path::new("foo")).quote();
    }
}
