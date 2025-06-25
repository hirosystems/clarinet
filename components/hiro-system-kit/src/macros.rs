#[macro_export]
macro_rules! green {
    ($($arg:tt)*) => ({
        use colored::Colorize;
        format!($($arg)*).green().bold().to_string()
    })
}

#[macro_export]
macro_rules! red {
    ($($arg:tt)*) => ({
        use colored::Colorize;
        format!($($arg)*).red().bold().to_string()
    })
}

#[macro_export]
macro_rules! yellow {
    ($($arg:tt)*) => ({
        use colored::Colorize;
        format!($($arg)*).yellow().bold().to_string()
    })
}

#[macro_export]
macro_rules! blue {
    ($($arg:tt)*) => ({
        use colored::Colorize;
        format!($($arg)*).blue().bold().to_string()
    })
}

#[macro_export]
macro_rules! purple {
    ($($arg:tt)*) => ({
        use colored::Colorize;
        format!($($arg)*).purple().bold().to_string()
    })
}

#[macro_export]
macro_rules! black {
    ($($arg:tt)*) => ({
        use colored::Colorize;
        format!($($arg)*).black().bold().to_string()
    })
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
