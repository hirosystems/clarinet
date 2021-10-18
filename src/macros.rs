#[allow(unused_macros)]
macro_rules! green {
    ($($arg:tt)*) => (
        {
            use atty::Stream;
            use ansi_term::Colour;
            if atty::is(Stream::Stdout) {
                let colour = Colour::Green.bold();
                format!(
                    "{}",
                    colour.paint($($arg)*)
                )
            } else {
                format!(
                    "{}",
                    $($arg)*
                )
            }
        }
    )
}

#[allow(unused_macros)]
macro_rules! red {
    ($($arg:tt)*) => (
        {
            use atty::Stream;
            use ansi_term::Colour;
            if atty::is(Stream::Stdout) {
                let colour = Colour::Red.bold();
                format!(
                    "{}",
                    colour.paint($($arg)*)
                )
            } else {
                format!(
                    "{}",
                    $($arg)*
                )
            }
        }
    )
}

#[allow(unused_macros, unused_must_use)]
macro_rules! yellow {
    ($($arg:tt)*) => (
        {
            use atty::Stream;
            use ansi_term::Colour;
            if atty::is(Stream::Stdout) {
                let colour = Colour::Yellow.bold();
                format!(
                    "{}",
                    colour.paint($($arg)*)
                )
            } else {
                format!(
                    "{}",
                    $($arg)*
                )
            }
        }
    )
}

#[allow(unused_macros)]
macro_rules! blue {
    ($($arg:tt)*) => (
        {
            use atty::Stream;
            use ansi_term::Colour;
            if atty::is(Stream::Stdout) {
                let colour = Colour::Cyan.bold();
                format!(
                    "{}",
                    colour.paint($($arg)*)
                )
            } else {
                format!(
                    "{}",
                    $($arg)*
                )
            }
        }
    )
}
