use std::fmt;

use miette::{Context, IntoDiagnostic};

pub trait Describe<T, E>: miette::IntoDiagnostic<T, E> {
    fn describe<S: fmt::Display + Send + Sync + 'static>(self, s: S) -> miette::Result<T>;
    fn with_describe<F: FnOnce() -> S, S: fmt::Display + Send + Sync + 'static>(
        self,
        f: F,
    ) -> miette::Result<T>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> Describe<T, E> for Result<T, E> {
    fn describe<S: fmt::Display + Send + Sync + 'static>(self, s: S) -> miette::Result<T> {
        self.into_diagnostic().context(s)
    }

    fn with_describe<F: FnOnce() -> S, S: fmt::Display + Send + Sync + 'static>(
        self,
        f: F,
    ) -> miette::Result<T> {
        match &self {
            Ok(_) => self.into_diagnostic(),
            Err(_) => self.into_diagnostic().context(f()),
        }
    }
}
