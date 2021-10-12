use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

pub(crate) struct Shell {
    pub(crate) child: Mutex<Child>,
    pub(crate) raw: bool,
}

impl Shell {
    pub(crate) fn new(cmd: &mut Command) -> Self {
        Shell {
            child: Mutex::new(
                cmd.stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .spawn()
                    .unwrap(),
            ),
            raw: false,
        }
    }

    #[allow(unused)]
    pub(crate) fn raw(cmd: &mut Command) -> Self {
        let mut proc = Self::new(cmd);
        proc.raw = true;
        proc
    }

    pub(crate) fn send(&self, input: impl std::fmt::Display) -> Vec<u8> {
        let mut child = self.child.lock().unwrap();
        if self.raw {
            writeln!(child.stdin.as_mut().unwrap(), "{}", input).unwrap();
        } else {
            // \0 as an unambiguous separator, \n in case there's line buffering
            writeln!(child.stdin.as_mut().unwrap(), "printf '%s\\0\\n' {}", input).unwrap();
        }
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
