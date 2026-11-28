use serde::{Serialize, Serializer};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the forum's security check has not been passed yet")]
    SecurityCheck,

    #[error("the forum window is not open")]
    NoForumWindow,

    #[error("the forum answered with HTTP {0}")]
    Status(u16),

    #[error("Steam is rate limiting requests; try again in a minute")]
    SteamRateLimited,

    #[error("Steam: {0}")]
    Steam(String),

    #[error("{0}")]
    ForumMessage(String),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Tauri(#[from] tauri::Error),

    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Extract(#[from] crate::extract::ExtractError),

    #[error(transparent)]
    Download(#[from] crate::download::DownloadError),

    #[error(transparent)]
    JDownloader(#[from] crate::jdownloader::JdError),
}

impl Error {
    fn kind(&self) -> &'static str {
        match self {
            Error::SecurityCheck => "security_check",
            Error::NoForumWindow => "no_forum_window",
            Error::Status(_) => "status",
            Error::SteamRateLimited => "steam_rate_limited",
            Error::Steam(_) => "steam",
            Error::ForumMessage(_) => "forum_message",
            Error::Http(_) => "http",
            Error::Tauri(_) => "tauri",
            Error::Url(_) => "url",
            Error::Io(_) => "io",
            Error::Extract(e) => e.kind(),
            Error::Download(e) => e.kind(),
            Error::JDownloader(e) => e.kind(),
        }
    }
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("Error", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}
