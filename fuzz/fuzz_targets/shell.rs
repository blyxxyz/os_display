#![no_main]
use libfuzzer_sys::fuzz_target;

use std::ffi::OsStr;
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use once_cell::sync::Lazy;

use os_display::Quotable;

struct Shell(Mutex<Child>);

impl Shell {
    fn new(cmd: &mut Command) -> Self {
        Shell(Mutex::new(
            cmd.stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap(),
        ))
    }

    fn send(&self, input: impl std::fmt::Display) -> Vec<u8> {
        let mut child = self.0.lock().unwrap();
        // \0 as an unambiguous separator, \n in case there's line buffering
        write!(
            child.stdin.as_mut().unwrap(),
            "printf '%s\\0\\n' {} || exit 1;\n",
            input
        )
        .unwrap();
        let mut output = Vec::new();
        let mut pos = 0;
        fn read_uninterrupted(child: &mut Child, buf: &mut [u8]) -> usize {
            let stdout = child.stdout.as_mut().unwrap();
            loop {
                match stdout.read(buf) {
                    Ok(0) => panic!("empty read"),
                    Ok(n) => return n,
                    Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(err) => panic!("{:?}", err),
                }
            }
        }
        loop {
            output.resize(pos + 8192, 0);
            pos += read_uninterrupted(&mut child, &mut output[pos..]);
            output.truncate(pos);
            if output.contains(&0) {
                if output.last().unwrap() == &0 {
                    output.push(0);
                    assert_eq!(read_uninterrupted(&mut child, &mut output[pos..]), 1);
                }
                assert!(output.ends_with(&[b'\0', b'\n']));
                output.pop();
                output.pop();
                return output;
            }
        }
    }
}

// All these are packaged on Debian.
// apt install bash zsh ksh mksh busybox dash posh yash fish csh tcsh

// ksh-compatible shells
static BASH: Lazy<Shell> = Lazy::new(|| Shell::new(&mut Command::new("bash")));
static ZSH: Lazy<Shell> = Lazy::new(|| Shell::new(&mut Command::new("zsh")));
static KSH: Lazy<Shell> = Lazy::new(|| Shell::new(&mut Command::new("ksh93")));
static MKSH: Lazy<Shell> = Lazy::new(|| Shell::new(&mut Command::new("mksh")));
static BUSYBOX: Lazy<Shell> = Lazy::new(|| Shell::new(Command::new("busybox").arg("sh")));

// Shells without $'' but with everything else
static DASH: Lazy<Shell> = Lazy::new(|| Shell::new(&mut Command::new("dash")));
static POSH: Lazy<Shell> = Lazy::new(|| Shell::new(&mut Command::new("posh")));
// I didn't know about yash until running `apt search shell`, but it claims
// POSIX compliance, and it's good to throw obscure implementations in here.
// The rust port may be interesting? https://github.com/magicant/yash-rs
static YASH: Lazy<Shell> = Lazy::new(|| Shell::new(&mut Command::new("yash")));

static FISH: Lazy<Shell> = Lazy::new(|| {
    Shell::new(
        // Fish reads the whole script before executing, which is sane but not
        // what we need right now.
        Command::new("fish")
            .arg("-c")
            .arg("while read line; eval $line; end"),
    )
});
// tcsh seems to leak memory at ~100MB/h, so maybe don't include it in long runs.
static TCSH: Lazy<Shell> = Lazy::new(|| Shell::new(&mut Command::new("tcsh")));
// Debian has a port of OpenBSD's csh.
// It's omitted for now because `printf '%s\0\n' ܠ` consumes gigabytes of memory:
// https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=995013
// static CSH: Lazy<Shell> = Lazy::new(|| Shell::new(&mut Command::new("csh")));

fuzz_target!(|data: &[u8]| {
    // Can't pass null bytes
    let data = data.split(|b| *b == 0).next().unwrap();
    let text = OsStr::from_bytes(data);
    let quote = text.quote().to_string();
    let maybe_quote = text.maybe_quote().to_string();

    // Loop unrolled to easily see which line panics
    assert_eq!(BASH.send(&quote), data, "{:?}", text);
    assert_eq!(BASH.send(&maybe_quote), data, "{:?}", text);
    assert_eq!(ZSH.send(&quote), data, "{:?}", text);
    assert_eq!(ZSH.send(&maybe_quote), data, "{:?}", text);
    assert_eq!(KSH.send(&quote), data, "{:?}", text);
    assert_eq!(KSH.send(&maybe_quote), data, "{:?}", text);
    assert_eq!(MKSH.send(&quote), data, "{:?}", text);
    assert_eq!(MKSH.send(&maybe_quote), data, "{:?}", text);
    assert_eq!(BUSYBOX.send(&quote), data, "{:?}", text);
    assert_eq!(BUSYBOX.send(&maybe_quote), data, "{:?}", text);

    if !quote.starts_with('$') {
        assert_eq!(DASH.send(&quote), data, "{:?}", text);
        assert_eq!(DASH.send(&maybe_quote), data, "{:?}", text);
        assert_eq!(POSH.send(&quote), data, "{:?}", text);
        assert_eq!(POSH.send(&maybe_quote), data, "{:?}", text);
        assert_eq!(YASH.send(&quote), data, "{:?}", text);
        assert_eq!(YASH.send(&maybe_quote), data, "{:?}", text);
    }

    // Limited testing of fish and csh, to at least pin down what
    // the incompatibilities are.
    // $'' is not supported.
    // Backslashes have a meaning inside single quotes.
    if !quote.starts_with('$') && !quote.contains('\\') {
        // Fish does something buggy with the private use area.
        // One instance of this is `echo \uF661`, which outputs `a`.
        // wontfix: https://github.com/fish-shell/fish-shell/issues/8316
        if !quote
            .chars()
            .any(|ch| (ch as u32) >= 0xF600 && (ch as u32) <= 0xF6FF)
        {
            assert_eq!(FISH.send(&quote), data, "{:?}", text);
            assert_eq!(FISH.send(&maybe_quote), data, "{:?}", text);
        }

        // csh doesn't like a # in the middle of an argument in
        // non-interactive mode. (But in interactive mode it's ok,
        // so that doesn't really matter.)
        // It also doesn't like a ! in the middle.
        if !quote.contains(&['#', '!'][..]) {
            assert_eq!(TCSH.send(&quote), data, "{:?}", text);
            assert_eq!(TCSH.send(&maybe_quote), data, "{:?}", text);
            // assert_eq!(CSH.send(&quote), data, "{:?}", text);
            // assert_eq!(CSH.send(&maybe_quote), data, "{:?}", text);
        }
    }
});
