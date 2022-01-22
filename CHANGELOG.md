## v0.1.3 (2021-01-22)
- Add `Quoted::external()` to escape double quotes for native commands on Windows.
- Quote `U+2800 BRAILLE PATTERN BLANK` for clarity.

## v0.1.2 (2021-11-08)
- Escape dangerous control codes for bidirectional text. See also: [CVE-2021-42574](https://blog.rust-lang.org/2021/11/01/cve-2021-42574.html).

## v0.1.1 (2021-10-14)
- Escape unicode control characters like `U+0085 NEXT LINE` and `U+2028 LINE SEPARATOR`.

## v0.1.0 (2021-10-13)
- Initial release.
