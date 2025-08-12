use crate::spotify_id::SpotifyItemType;
use crate::{Error, SpotifyId};
use std::fmt;
use thiserror::Error;

use librespot_protocol as protocol;

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
pub enum SpotifyUri {
    Album {
        id: SpotifyId,
    },
    Artist {
        id: SpotifyId,
    },
    Episode {
        id: SpotifyId,
    },
    Playlist {
        user: Option<String>,
        id: SpotifyId,
    },
    Show {
        id: SpotifyId,
    },
    Track {
        id: SpotifyId,
    },
    Local {
        artist: String,
        album_title: String,
        track_title: String,
        duration: std::time::Duration,
    },
    Unknown {
        id: SpotifyId,
    },
}

impl SpotifyUri {
    /// Returns whether this `SpotifyUri` is for a playable audio item, if known.
    pub fn is_playable(&self) -> bool {
        matches!(self, SpotifyUri::Episode { .. } | SpotifyUri::Track { .. })
    }

    pub fn item_type(&self) -> SpotifyItemType {
        match &self {
            SpotifyUri::Album { .. } => SpotifyItemType::Album,
            SpotifyUri::Artist { .. } => SpotifyItemType::Artist,
            SpotifyUri::Episode { .. } => SpotifyItemType::Episode,
            SpotifyUri::Playlist { .. } => SpotifyItemType::Playlist,
            SpotifyUri::Show { .. } => SpotifyItemType::Show,
            SpotifyUri::Track { .. } => SpotifyItemType::Track,
            SpotifyUri::Local { .. } => SpotifyItemType::Local,
            SpotifyUri::Unknown { .. } => SpotifyItemType::Unknown,
        }
    }

    pub fn to_name(&self) -> Result<String, Error> {
        match &self {
            SpotifyUri::Album { id }
            | SpotifyUri::Artist { id }
            | SpotifyUri::Episode { id }
            | SpotifyUri::Playlist { id, user: None }
            | SpotifyUri::Show { id }
            | SpotifyUri::Track { id }
            | SpotifyUri::Unknown { id } => id.to_base62(),
            SpotifyUri::Local {
                artist,
                album_title,
                track_title,
                duration,
            } => {
                let duration_secs = duration.as_secs();
                Ok(format!(
                    "{artist}:{album_title}:{track_title}:{duration_secs}"
                ))
            }
            SpotifyUri::Playlist {
                id,
                user: Some(user),
            } => Ok(format!("{user}:{id}")),
        }
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
        let mut username: Option<String> = None;

        let item_type = {
            let next = parts.next().ok_or(SpotifyUriError::InvalidFormat)?;
            if next == "user" {
                username.replace(
                    parts
                        .next()
                        .ok_or(SpotifyUriError::InvalidFormat)?
                        .to_owned(),
                );
                parts.next().ok_or(SpotifyUriError::InvalidFormat)?
            } else {
                next
            }
        };

        let name = parts.next().ok_or(SpotifyUriError::InvalidFormat)?;

        if scheme != "spotify" {
            return Err(SpotifyUriError::InvalidRoot.into());
        }

        let item_type = item_type.into();

        match item_type {
            SpotifyItemType::Album => Ok(Self::Album {
                id: SpotifyId::from_base62(name)?,
            }),
            SpotifyItemType::Artist => Ok(Self::Artist {
                id: SpotifyId::from_base62(name)?,
            }),
            SpotifyItemType::Episode => Ok(Self::Episode {
                id: SpotifyId::from_base62(name)?,
            }),
            SpotifyItemType::Playlist => Ok(Self::Playlist {
                id: SpotifyId::from_base62(name)?,
                user: username,
            }),
            SpotifyItemType::Show => Ok(Self::Show {
                id: SpotifyId::from_base62(name)?,
            }),
            SpotifyItemType::Track => Ok(Self::Track {
                id: SpotifyId::from_base62(name)?,
            }),
            SpotifyItemType::Local => Ok(Self::Local {
                artist: "unimplemented".to_owned(),
                album_title: "unimplemented".to_owned(),
                track_title: "unimplemented".to_owned(),
                duration: Default::default(),
            }),
            SpotifyItemType::Unknown => Ok(Self::Unknown {
                id: SpotifyId::from_base62(name)?,
            }),
        }
    }

    pub fn from_raw(src: &[u8], item_type: SpotifyItemType) -> SpotifyUriResult {
        let id = SpotifyId::from_raw(src)?;

        match item_type {
            SpotifyItemType::Album => Ok(Self::Album { id }),
            SpotifyItemType::Artist => Ok(Self::Artist { id }),
            SpotifyItemType::Episode => Ok(Self::Episode { id }),
            SpotifyItemType::Playlist => Ok(Self::Playlist { id, user: None }),
            SpotifyItemType::Show => Ok(Self::Show { id }),
            SpotifyItemType::Track => Ok(Self::Track { id }),
            SpotifyItemType::Unknown => Ok(Self::Unknown { id }),
            SpotifyItemType::Local => Err(SpotifyUriError::InvalidFormat.into()),
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
        let item_type: &str = self.item_type().into();
        let name = self.to_name()?;

        Ok(format!("spotify:{item_type}:{name}"))
    }

    #[deprecated(since = "0.7.0", note = "use to_name instead")]
    pub fn to_base62(&self) -> Result<String, Error> {
        self.to_name()
    }
}

impl From<SpotifyId> for SpotifyUri {
    fn from(id: SpotifyId) -> Self {
        Self::Unknown { id }
    }
}

impl fmt::Debug for SpotifyUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SpotifyUri")
            .field(&self.to_uri().unwrap_or_else(|_| "invalid uri".into()))
            .finish()
    }
}

impl TryFrom<&protocol::metadata::Album> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(album: &protocol::metadata::Album) -> Result<Self, Self::Error> {
        Ok(Self::Album {
            id: SpotifyId::from_raw(album.gid())?,
        })
    }
}

impl TryFrom<&protocol::metadata::Artist> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(artist: &protocol::metadata::Artist) -> Result<Self, Self::Error> {
        Ok(Self::Artist {
            id: SpotifyId::from_raw(artist.gid())?,
        })
    }
}

impl TryFrom<&protocol::metadata::Episode> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(episode: &protocol::metadata::Episode) -> Result<Self, Self::Error> {
        Ok(Self::Episode {
            id: SpotifyId::from_raw(episode.gid())?,
        })
    }
}

impl TryFrom<&protocol::metadata::Track> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(track: &protocol::metadata::Track) -> Result<Self, Self::Error> {
        Ok(Self::Track {
            id: SpotifyId::from_raw(track.gid())?,
        })
    }
}

impl TryFrom<&protocol::metadata::Show> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(show: &protocol::metadata::Show) -> Result<Self, Self::Error> {
        Ok(Self::Show {
            id: SpotifyId::from_raw(show.gid())?,
        })
    }
}

impl TryFrom<&protocol::metadata::ArtistWithRole> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(artist: &protocol::metadata::ArtistWithRole) -> Result<Self, Self::Error> {
        Ok(Self::Artist {
            id: SpotifyId::from_raw(artist.artist_gid())?,
        })
    }
}

impl TryFrom<&protocol::playlist4_external::Item> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(item: &protocol::playlist4_external::Item) -> Result<Self, Self::Error> {
        Self::from_uri(item.uri())
    }
}

// Note that this is the unique revision of an item's metadata on a playlist,
// not the ID of that item or playlist.
impl TryFrom<&protocol::playlist4_external::MetaItem> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(item: &protocol::playlist4_external::MetaItem) -> Result<Self, Self::Error> {
        Ok(Self::Unknown {
            id: SpotifyId::try_from(item.revision())?,
        })
    }
}

// Note that this is the unique revision of a playlist, not the ID of that playlist.
impl TryFrom<&protocol::playlist4_external::SelectedListContent> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(
        playlist: &protocol::playlist4_external::SelectedListContent,
    ) -> Result<Self, Self::Error> {
        Ok(Self::Unknown {
            id: SpotifyId::try_from(playlist.revision())?,
        })
    }
}

// TODO: check meaning and format of this field in the wild. This might be a FileId,
// which is why we now don't create a separate `Playlist` enum value yet and choose
// to discard any item type.
impl TryFrom<&protocol::playlist_annotate3::TranscodedPicture> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(
        picture: &protocol::playlist_annotate3::TranscodedPicture,
    ) -> Result<Self, Self::Error> {
        Ok(Self::Unknown {
            id: SpotifyId::from_base62(picture.uri())?,
        })
    }
}
