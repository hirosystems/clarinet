#[allow(unused_macros)]
macro_rules! green {
    ($($arg:tt)*) => (
        {
            use atty::Stream;
            use ansi_term::Colour;
            if atty::is(Stream::Stdout) {
                let colour = Colour::Green;
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
            use std::io::Write;
            use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
            let mut stdout = StandardStream::stdout(ColorChoice::Always);
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
            writeln!(&mut stdout, $($arg)*)
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
                let colour = Colour::Yellow;
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
                let colour = Colour::Cyan;
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
