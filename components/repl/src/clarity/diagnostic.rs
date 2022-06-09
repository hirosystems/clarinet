use super::representations::Span;
use std::fmt;

/// In a near future, we can go further in our static analysis and provide different levels
/// of diagnostics, such as warnings, hints, best practices, etc.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Level {
    Note,
    Warning,
    Error,
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Level::Note => write!(f, "{}", blue!("note")),
            Level::Warning => write!(f, "{}", yellow!("warning")),
            Level::Error => write!(f, "{}", red!("error")),
        }
    }
}

pub trait DiagnosableError {
    fn message(&self) -> String;
    fn suggestion(&self) -> Option<String>;
    fn level(&self) -> Level {
        Level::Error
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Diagnostic {
    pub level: Level,
    pub message: String,
    pub spans: Vec<Span>,
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn err(error: &dyn DiagnosableError) -> Diagnostic {
        Diagnostic {
            spans: vec![],
            level: Level::Error,
            message: error.message(),
            suggestion: error.suggestion(),
        }
    }

    pub fn add_span(&mut self, start_line: u32, start_column: u32, end_line: u32, end_column: u32) {
        self.spans.push(Span {
            start_line,
            start_column,
            end_line,
            end_column,
        });
    }

    // Generate the formatted output for this diagnostic, given the source code.
    // TODO: Preferably a filename would be saved in the Span, but for now, pass a name here.
    pub fn output(&self, name: &str, lines: &Vec<String>) -> Vec<String> {
        let mut output = Vec::new();
        if self.spans.len() > 0 {
            output.push(format!(
                "{}:{}:{}: {}: {}",
                name, // self.spans[0].filename,
                self.spans[0].start_line,
                self.spans[0].start_column,
                self.level,
                self.message,
            ));
        } else {
            output.push(format!("{}: {}", self.level, self.message,));
        }
        output.append(&mut self.output_code(lines));
        output
    }

    pub fn output_code(&self, lines: &Vec<String>) -> Vec<String> {
        let mut output = Vec::new();
        if self.spans.is_empty() {
            return output;
        }
        let span = &self.spans[0];
        let first_line = span.start_line.saturating_sub(1) as usize;
        let last_line = span.end_line.saturating_sub(1) as usize;

        output.push(lines[first_line].clone());
        let mut pointer = format!(
            "{: <1$}^",
            "",
            (span.start_column.saturating_sub(1)) as usize
        );
        if span.start_line == span.end_line {
            pointer = format!(
                "{}{:~<2$}",
                pointer,
                "",
                (span.end_column - span.start_column) as usize
            );
        }
        pointer = format!("{}", pointer);
        output.push(pointer);

        for span in self.spans.iter().skip(1) {
            let first_line = span.start_line.saturating_sub(1) as usize;
            let last_line = span.end_line.saturating_sub(1) as usize;

            output.push(lines[first_line].clone());
            let mut pointer = format!(
                "{: <1$}^",
                "",
                (span.start_column.saturating_sub(1)) as usize
            );
            if span.start_line == span.end_line {
                pointer = format!(
                    "{}{:~<2$}",
                    pointer,
                    "",
                    (span.end_column - span.start_column) as usize
                );
            } else {
                for line_num in (first_line + 1)..last_line {
                    output.push(lines[line_num].clone());
                }
            }
            output.push(pointer);
        }
        output
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.level)?;
        if self.spans.len() == 1 {
            write!(
                f,
                " (line {}, column {})",
                self.spans[0].start_line, self.spans[0].start_column
            )?;
        } else if self.spans.len() > 1 {
            let lines: Vec<String> = self
                .spans
                .iter()
                .map(|s| format!("line: {}", s.start_line))
                .collect();
            write!(f, " ({})", lines.join(", "))?;
        }
        write!(f, ": {}.", &self.message)?;
        if let Some(suggestion) = &self.suggestion {
            write!(f, "\n{}", suggestion)?;
        }
        write!(f, "\n")
    }
}
