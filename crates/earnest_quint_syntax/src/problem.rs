use std::fmt;

/// Why Quint refuses a file, at the place Quint names.
///
/// It is the first error `quint parse` reports for the file, with its code. A syntax error's
/// message is earnest's own wording: Quint's lists the tokens its parser expected, and those
/// are not reproduced. Every other message is Quint's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    /// Quint's error code, such as `QNT000` for a syntax error or `QNT013` for an import that
    /// can't be loaded.
    pub code: String,
    /// What is wrong.
    pub message: String,
    /// The file, as Quint names it in its source map.
    pub source: String,
    /// Counted from 1.
    pub line: usize,
    /// Counted from 1 along the line, in characters.
    pub column: usize,
}

impl Problem {
    pub(crate) fn syntax(message: String, line: usize, column: usize) -> Self {
        Self::coded("QNT000", message, line, column)
    }

    pub(crate) fn coded(code: &str, message: String, line: usize, column: usize) -> Self {
        Self {
            code: code.to_owned(),
            message,
            source: String::new(),
            line: line + 1,
            column: column + 1,
        }
    }

    pub(crate) fn in_source(mut self, source: &str) -> Self {
        source.clone_into(&mut self.source);
        self
    }
}

impl fmt::Display for Problem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}:{}:{} [{}] {}",
            self.source, self.line, self.column, self.code, self.message
        )
    }
}
