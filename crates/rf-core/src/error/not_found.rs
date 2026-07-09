//! Ergonomic `Option`/`Result` -> `404` bridge for the terse handler path.
//!
//! The `find!(Model, id)` macro expands to `Model::find(id).await`, whose type
//! is `Result<Option<Value>, String>`: a *missing* row is a legitimate `Ok(None)`,
//! not an error, so a bare `?` cannot turn it into a `404`. Idiomatic Rust would
//! force a hand-written `match`/`ok_or_else(..)` at every lookup — exactly the
//! boilerplate that makes a handler 3-4x longer than its Laravel equivalent.
//!
//! [`OrNotFound`] closes that gap **without hiding `Result`/`Option`** (which is a
//! language ceiling and stays honest): it adds `.or_404()` / `.or_not_found()`
//! extension methods that map an absent value to [`AppError::NotFound`], so the
//! whole lookup collapses to one first-class `?` expression:
//!
//! ```rust,ignore
//! let row = find!(Task, id).or_404()?;      // Result<Option<Value>, String> -> Value
//! ```
//!
//! The trait is implemented for both a plain `Option<T>` and a
//! `Result<Option<T>, E>` (where `E: Into<AppError>`, e.g. the ORM's `String`
//! error), so it works whether or not the caller has already unwrapped the
//! surrounding `Result`.

use crate::error::AppError;

/// Extension trait turning "value might be absent" into an `AppError::NotFound`
/// (`404`) so it can be propagated with `?`.
///
/// See the [module docs](self) for the motivation.
pub trait OrNotFound<T> {
    /// Map an absent value to a `404` with a generic `"Resource"` label.
    ///
    /// For a plain `Option`, `Some(v)` becomes `Ok(v)` and `None` becomes
    /// `Err(AppError::NotFound { .. })`. For a `Result<Option<T>, E>`, any
    /// underlying `Err(e)` is preserved (converted via `E: Into<AppError>`),
    /// `Ok(None)` becomes the `404`, and `Ok(Some(v))` becomes `Ok(v)`.
    fn or_not_found(self) -> Result<T, AppError>;

    /// Alias of [`or_not_found`](Self::or_not_found) reading closer to the HTTP
    /// status the app author is thinking in (`find!(Model, id).or_404()?`).
    fn or_404(self) -> Result<T, AppError>
    where
        Self: Sized,
    {
        self.or_not_found()
    }

    /// Like [`or_404`](Self::or_404) but with a caller-supplied resource label
    /// that appears in the `404` problem-details body
    /// (`find!(Task, id).or_404_with("Task")?`).
    fn or_404_with(self, resource: impl Into<String>) -> Result<T, AppError>;
}

impl<T> OrNotFound<T> for Option<T> {
    fn or_not_found(self) -> Result<T, AppError> {
        self.or_404_with("Resource")
    }

    fn or_404_with(self, resource: impl Into<String>) -> Result<T, AppError> {
        self.ok_or_else(|| AppError::NotFound {
            resource: resource.into(),
        })
    }
}

impl<T, E> OrNotFound<T> for Result<Option<T>, E>
where
    E: Into<AppError>,
{
    fn or_not_found(self) -> Result<T, AppError> {
        self.or_404_with("Resource")
    }

    fn or_404_with(self, resource: impl Into<String>) -> Result<T, AppError> {
        match self {
            Ok(Some(value)) => Ok(value),
            Ok(None) => Err(AppError::NotFound {
                resource: resource.into(),
            }),
            Err(err) => Err(err.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn option_some_is_ok() {
        let got: Option<i32> = Some(7);
        assert_eq!(got.or_404().unwrap(), 7);
    }

    #[test]
    fn option_none_is_404() {
        let got: Option<i32> = None;
        let err = got.or_not_found().unwrap_err();
        assert_eq!(err.status_code(), 404);
    }

    #[test]
    fn result_ok_some_is_ok() {
        // Shape returned by `find!(Model, id)`.
        let got: Result<Option<i32>, String> = Ok(Some(42));
        assert_eq!(got.or_404().unwrap(), 42);
    }

    #[test]
    fn result_ok_none_is_404() {
        let got: Result<Option<i32>, String> = Ok(None);
        let err = got.or_404_with("Task").unwrap_err();
        assert_eq!(err.status_code(), 404);
        match err {
            AppError::NotFound { resource } => assert_eq!(resource, "Task"),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn result_err_preserves_underlying_error() {
        // An actual DB failure must stay a 500, not be masked as a 404.
        let got: Result<Option<i32>, String> = Err("db down".to_string());
        let err = got.or_404().unwrap_err();
        assert_eq!(err.status_code(), 500);
    }

    #[test]
    fn propagates_with_question_mark() {
        fn handler() -> crate::AppResult<i32> {
            let found: Result<Option<i32>, String> = Ok(None);
            let value = found.or_404()?;
            Ok(value)
        }
        assert_eq!(handler().unwrap_err().status_code(), 404);
    }
}
