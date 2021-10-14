use os_display::Quotable;

fn main() {
    for arg in std::env::args_os().skip(1) {
        println!("Native: {}", arg.maybe_quote());
        #[cfg(any(feature = "windows", feature = "unix"))]
        if let Some(arg) = arg.to_str() {
            use os_display::Quoted;
            #[cfg(feature = "unix")]
            #[cfg(windows)]
            println!("Unix: {}", Quoted::unix(arg).force(false));
            #[cfg(feature = "windows")]
            #[cfg(not(windows))]
            println!("Windows: {}", Quoted::windows(arg).force(false));
        }
    }
}
