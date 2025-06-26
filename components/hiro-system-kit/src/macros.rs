#[macro_export]
macro_rules! green {
    ($i:ident)  => ({
        use colored::{ColoredString, Colorize};
        ColoredString::from($i).green().bold().to_string()
    });
    ($($arg:tt)*) => ({
        let s = format!($($arg)*);
        green!(s)
    });
}

#[macro_export]
macro_rules! red {
    ($i:ident)  => ({
        use colored::Colorize;
        $i.red().bold().to_string()
    });
    ($($arg:tt)*) => ({
        let s = format!($($arg)*);
        red!(s)
    });
}

#[macro_export]
macro_rules! yellow {
    ($i:ident)  => ({
        use colored::Colorize;
        $i.yellow().bold().to_string()
    });
    ($($arg:tt)*) => ({
        let s = format!($($arg)*);
        yellow!(s)
    });
}

#[macro_export]
macro_rules! blue {
    ($i:ident)  => ({
        use colored::Colorize;
        $i.blue().bold().to_string()
    });
    ($($arg:tt)*) => ({
        let s = format!($($arg)*);
        blue!(s)
    });
}

#[macro_export]
macro_rules! purple {
    ($i:ident)  => ({
        use colored::Colorize;
        $i.purple().bold().to_string()
    });
    ($($arg:tt)*) => ({
        let s = format!($($arg)*);
        purple!(s)
    });
}

#[macro_export]
macro_rules! black {
    ($i:ident)  => ({
        use colored::Colorize;
        $i.black().bold().to_string()
    });
    ($($arg:tt)*) => ({
        let s = format!($($arg)*);
        black!(s)
    });
}

#[macro_export]
macro_rules! pluralize {
    ($value:expr, $word:expr) => {
        if $value > 1 {
            format!("{} {}s", $value, $word)
        } else {
            format!("{} {}", $value, $word)
        }
    };
}

#[macro_export]
macro_rules! format_err {
    ($($arg:tt)*) => (
        {
            format!("{} {}", red!("error:"), $($arg)*)
        }
    )
}

#[macro_export]
macro_rules! format_warn {
    ($($arg:tt)*) => (
        {
            format!("{} {}", yellow!("warn:"), $($arg)*)
        }
    )
}

#[macro_export]
macro_rules! format_note {
    ($($arg:tt)*) => (
        {
            format!("{} {}", blue!("note:"), $($arg)*)
        }
    )
}
