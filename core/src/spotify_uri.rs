use crate::spotify_id::SpotifyItemType;
use crate::{Error, SpotifyId};
use std::fmt;
use thiserror::Error;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum SpotifyUriError {
    #[error("not a valid Spotify URI")]
    InvalidFormat,
    #[error("URI does not belong to Spotify")]
    InvalidRoot,
}

impl From<SpotifyUriError> for Error {
    fn from(err: SpotifyUriError) -> Self {
        Error::invalid_argument(err)
    }
}

pub type SpotifyUriResult = Result<SpotifyUri, Error>;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum SpotifyResourceName {
    Id(SpotifyId),
    String(String),
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SpotifyUri {
    pub item_type: SpotifyItemType,
    pub name: SpotifyResourceName,
}

impl SpotifyUri {
    /// Returns whether this `SpotifyUri` is for a playable audio item, if known.
    pub fn is_playable(&self) -> bool {
        matches!(
            self.item_type,
            SpotifyItemType::Episode | SpotifyItemType::Track
        )
    }

    /// Parses a [Spotify URI] into a `SpotifyUri`.
    ///
    /// `uri` is expected to be in the canonical form `spotify:{type}:{id}`, where `{type}`
    /// can be arbitrary while `{id}` is a 22-character long, base62 encoded Spotify ID.
    ///
    /// Note that this should not be used for playlists, which have the form of
    /// `spotify:playlist:{id}`.
    ///
    /// [Spotify URI]: https://developer.spotify.com/documentation/web-api/concepts/spotify-uris-ids
    pub fn from_uri(src: &str) -> SpotifyUriResult {
        // Basic: `spotify:{type}:{id}`
        // Named: `spotify:user:{user}:{type}:{id}`
        // Local: `spotify:local:{artist}:{album_title}:{track_title}:{duration_in_seconds}`
        let mut parts = src.split(':');

        let scheme = parts.next().ok_or(SpotifyUriError::InvalidFormat)?;

        let item_type = {
            let next = parts.next().ok_or(SpotifyUriError::InvalidFormat)?;
            if next == "user" {
                let _username = parts.next().ok_or(SpotifyUriError::InvalidFormat)?;
                parts.next().ok_or(SpotifyUriError::InvalidFormat)?
            } else {
                next
            }
        };

        let id = parts.next().ok_or(SpotifyUriError::InvalidFormat)?;

        if scheme != "spotify" {
            return Err(SpotifyUriError::InvalidRoot.into());
        }

        let item_type = item_type.into();

        match item_type {
            SpotifyItemType::Local => Ok(Self {
                item_type,
                name: SpotifyResourceName::String("not implemented".to_owned()),
            }),
            _ => {
                let mut id = SpotifyId::from_base62(id)?;
                id.item_type = item_type;

                Ok(Self {
                    item_type,
                    name: SpotifyResourceName::Id(id),
                })
            }
        }
    }

    /// Returns the `SpotifyUri` as a [Spotify URI] in the canonical form `spotify:{type}:{id}`,
    /// where `{type}` is an arbitrary string and `{id}` is a 22-character long, base62 encoded
    /// Spotify ID.
    ///
    /// If the `SpotifyUri` has an associated type unrecognized by the library, `{type}` will
    /// be encoded as `unknown`.
    ///
    /// [Spotify URI]: https://developer.spotify.com/documentation/web-api/concepts/spotify-uris-ids
    pub fn to_uri(&self) -> Result<String, Error> {
        let item_type: &str = self.item_type.into();

        match self.name {
            SpotifyResourceName::Id(id) => {
                // 8 chars for the "spotify:" prefix + 1 colon + 22 chars base62 encoded ID  = 31
                // + unknown size item_type.
                let mut dst = String::with_capacity(31 + item_type.len());
                dst.push_str("spotify:");
                dst.push_str(item_type);
                dst.push(':');
                let base_62 = id.to_base62()?;
                dst.push_str(&base_62);
                Ok(dst)
            }
            SpotifyResourceName::String(ref s) => Ok(format!("spotify:{item_type}:{s}")),
        }
    }

    #[deprecated(since = "0.7.0", note = "use to_name instead")]
    pub fn to_base62(&self) -> Result<String, Error> {
        self.to_name()
    }

    pub fn to_name(&self) -> Result<String, Error> {
        match self.name {
            SpotifyResourceName::Id(id) => {
                let base_62 = id.to_base62()?;
                Ok(base_62)
            }
            SpotifyResourceName::String(ref s) => Ok(s.clone()),
        }
    }
}

impl From<SpotifyId> for SpotifyUri {
    fn from(id: SpotifyId) -> Self {
        Self {
            item_type: id.item_type,
            name: SpotifyResourceName::Id(id),
        }
    }
}

impl fmt::Debug for SpotifyUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SpotifyUri")
            .field(&self.to_uri().unwrap_or_else(|_| "invalid uri".into()))
            .finish()
    }
}
