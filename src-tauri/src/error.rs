use serde::{Serialize, Serializer};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The forum answered with its JavaScript security-check page. The forum
    /// window has been told to reload so it can pass the check; retry once it
    /// has finished loading.
    #[error("the forum's security check has not been passed yet")]
    SecurityCheck,

    #[error("the forum window is not open")]
    NoForumWindow,

    #[error("the forum answered with HTTP {0}")]
    Status(u16),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Tauri(#[from] tauri::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl Error {
    fn kind(&self) -> &'static str {
        match self {
            Error::SecurityCheck => "security_check",
            Error::NoForumWindow => "no_forum_window",
            Error::Status(_) => "status",
            Error::Http(_) => "http",
            Error::Tauri(_) => "tauri",
            Error::Url(_) => "url",
            Error::Io(_) => "io",
        }
    }
}

/// Commands return errors to the frontend as `{ kind, message }`.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("Error", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}
