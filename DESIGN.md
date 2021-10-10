Some loose thoughts that don't belong in the README or the source code.

# Generality

You can't use another platform's quoting style. You also can't use this library with `no_std`.

A simple platform-independent implementation of PowerShell's quoting would operate on e.g. `impl IntoIterator<Item = u16>`, but that means a ton of transcoding in the common case. A more advanced implementation could do something tricky with a trait to get a `Result<Cow<str>, Cow<[u16]>>` out of various types but I kind of hate that.

A platform-independent implementation of Unix quoting would be more reasonable, but it would still grow the API, so I don't want to be hasty with it.

(If something like this would be useful to you please open an issue.)

# Speed

Strings are iterated over with `.chars()` to check if they contain unicode whitespace. This check isn't vital on Unix, most shells only care about ASCII whitespace. It may be a bit faster to use `.as_bytes().iter()`.

There's a UTF-8 validation step that's unnecessary for types like `&str` that are already known to be UTF-8. I don't think this is optimized away (but haven't checked). It doesn't seem fixable without making the `Quotable` impl more complicated.

The performance seems fine but if it's ever not fine this could be a place to start.
