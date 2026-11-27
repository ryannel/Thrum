use std::fmt;

/// The reason a game description, rule or test operation was refused.
/// ```
/// let error = thrum::Error::invalid("health", "must be positive");
/// assert!(error.to_string().contains("health"));
/// ```
#[derive(Debug)]
pub enum Error {
    /// A game value failed the rule's validation.
    /// ```
    /// assert!(matches!(thrum::Error::invalid("speed", "too high"), thrum::Error::Invalid { .. }));
    /// ```
    Invalid {
        /// The invalid value.
        /// ```
        /// if let thrum::Error::Invalid { object, .. } = thrum::Error::invalid("speed", "too high") {
        ///     assert_eq!(object, "speed");
        /// }
        /// ```
        object: String,
        /// The reason the value was refused.
        /// ```
        /// if let thrum::Error::Invalid { cause, .. } = thrum::Error::invalid("speed", "too high") {
        ///     assert_eq!(cause, "too high");
        /// }
        /// ```
        cause: String,
    },
    /// A game description or operation must be corrected before retrying.
    /// ```
    /// let error = thrum::Error::invalid("health", "must be positive");
    /// assert_eq!(error.code(), "T0401");
    /// ```
    Refused(
        /// The message, including the stable code and remedy.
        /// ```
        /// assert!(thrum::Error::invalid("health", "must be positive").to_string().contains("T0401"));
        /// ```
        Diagnostic,
    ),
}

/// A named game error and its remedy.
/// ```
/// let error = thrum::Error::invalid("health", "must be positive");
/// assert!(error.to_string().contains("health"));
/// ```
#[derive(Debug)]
pub struct Diagnostic {
    code: &'static str,
    object: String,
    cause: String,
    remedy: &'static str,
}

/// The outcome of a game operation.
/// ```
/// let result: thrum::Result<i32> = Ok(3);
/// assert_eq!(result.unwrap(), 3);
/// ```
pub type Result<T = ()> = std::result::Result<T, Error>;

impl Error {
    /// Refuse an invalid game value from a rule.
    /// ```
    /// assert!(thrum::Error::invalid("score", "overflow").to_string().contains("overflow"));
    /// ```
    pub fn invalid(object: impl Into<String>, cause: impl Into<String>) -> Self {
        Self::Invalid {
            object: object.into(),
            cause: cause.into(),
        }
    }

    /// A stable identifier for the mistake and its remedy.
    /// ```
    /// assert_eq!(thrum::Error::invalid("score", "overflow").code(), "T0401");
    /// ```
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid { .. } => "T0401",
            Self::Refused(diagnostic) => diagnostic.code,
        }
    }

    pub(crate) fn refused(
        code: &'static str,
        object: impl Into<String>,
        cause: impl Into<String>,
        remedy: &'static str,
    ) -> Self {
        Self::Refused(Diagnostic {
            code,
            object: object.into(),
            cause: cause.into(),
            remedy,
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid { object, cause } => {
                write!(
                    output,
                    "{}: {object}: {cause}. Correct the value and retry.",
                    self.code()
                )
            }
            Self::Refused(Diagnostic {
                code,
                object,
                cause,
                remedy,
            }) => {
                write!(output, "{code}: {object}: {cause}. {remedy}")
            }
        }
    }
}

impl std::error::Error for Error {}
