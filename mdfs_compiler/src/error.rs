use mdf_schema::Microseconds;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileErrorKind {
    Parse,
    Semantic,
    IO,
    TimeMap,
    Validation,
}

impl CompileErrorKind {
    pub(crate) fn from_code(code: &'static str) -> Self {
        // Spec: docs/MDFS_DSL-and-Compiler_Spec.md#6.2
        match code {
            // Parse
            "E1001" | "E1002" | "E1003" | "E1004" | "E1005" | "E1006" | "E1101" | "E3201" | "E3202"
            | "E3203" | "E3204" => Self::Parse,

            // IO
            "E2001" | "E2002" | "E2003" | "E2004" => Self::IO,

            // Semantic
            "E2101" | "E4201" => Self::Semantic,

            // TimeMap
            "E3001" | "E3002" | "E3003" | "E3004" | "E3005" => Self::TimeMap,

            // Validation
            "E4001" | "E4002" | "E4003" | "E4004" | "E4101" | "E4102" => Self::Validation,

            // MVP default: treat unknown codes as Parse.
            _ => Self::Parse,
        }
    }
}

#[derive(Debug, Error, Clone)]
#[error("{code}: {message} (line {line})")]
pub struct CompileError {
    pub code: &'static str,
    pub kind: CompileErrorKind,
    pub message: String,
    pub line: usize,

    // --- Structured fields (MVP: optional, message stays source-of-truth) ---
    pub file: Option<String>,
    pub column: Option<usize>,
    pub step_index: Option<usize>,
    pub lane: Option<u8>,
    pub time_us: Option<Microseconds>,
    pub context: Option<String>,
}

impl CompileError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>, line: usize) -> Self {
        Self {
            code,
            kind: CompileErrorKind::from_code(code),
            message: message.into(),
            line,

            file: None,
            column: None,
            step_index: None,
            lane: None,
            time_us: None,
            context: None,
        }
    }

    pub fn with_file(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    pub fn with_step_index(mut self, step_index: usize) -> Self {
        self.step_index = Some(step_index);
        self
    }

    pub fn with_lane(mut self, lane: u8) -> Self {
        self.lane = Some(lane);
        self
    }

    pub fn with_time_us(mut self, time_us: Microseconds) -> Self {
        self.time_us = Some(time_us);
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}
