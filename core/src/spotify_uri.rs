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
pub enum SpotifyResourceName {
    Id(SpotifyId),
    NamedId(String, SpotifyId),
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

        let name = parts.next().ok_or(SpotifyUriError::InvalidFormat)?;

        if scheme != "spotify" {
            return Err(SpotifyUriError::InvalidRoot.into());
        }

        let item_type = item_type.into();

        match item_type {
            SpotifyItemType::Local => Ok(Self {
                item_type,
                name: SpotifyResourceName::String("not implemented".to_owned()),
            }),
            _ => Ok(Self {
                item_type,
                name: SpotifyResourceName::Id(SpotifyId::from_base62(name)?),
            }),
        }
    }

    pub fn from_raw(src: &[u8]) -> SpotifyUriResult {
        Ok(Self {
            item_type: SpotifyItemType::Unknown,
            name: SpotifyResourceName::Id(SpotifyId::from_raw(src)?),
        })
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
        let name = self.to_name()?;

        match self.name {
            SpotifyResourceName::NamedId(ref username, _) => {
                Ok(format!("spotify:user:{username}:{item_type}:{name}"))
            }
            _ => Ok(format!("spotify:{item_type}:{name}")),
        }
    }

    #[deprecated(since = "0.7.0", note = "use to_name instead")]
    pub fn to_base62(&self) -> Result<String, Error> {
        self.to_name()
    }

    pub fn to_name(&self) -> Result<String, Error> {
        match self.name {
            SpotifyResourceName::Id(id) | SpotifyResourceName::NamedId(_, id) => {
                Ok(id.to_base62()?)
            }
            SpotifyResourceName::String(ref s) => Ok(s.clone()),
        }
    }
}

impl From<SpotifyId> for SpotifyUri {
    fn from(id: SpotifyId) -> Self {
        Self {
            item_type: SpotifyItemType::Unknown,
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

impl TryFrom<&protocol::metadata::Album> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(album: &protocol::metadata::Album) -> Result<Self, Self::Error> {
        Ok(Self {
            item_type: SpotifyItemType::Album,
            name: SpotifyResourceName::Id(SpotifyId::from_raw(album.gid())?),
        })
    }
}

impl TryFrom<&protocol::metadata::Artist> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(artist: &protocol::metadata::Artist) -> Result<Self, Self::Error> {
        Ok(Self {
            item_type: SpotifyItemType::Artist,
            name: SpotifyResourceName::Id(SpotifyId::from_raw(artist.gid())?),
        })
    }
}

impl TryFrom<&protocol::metadata::Episode> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(episode: &protocol::metadata::Episode) -> Result<Self, Self::Error> {
        Ok(Self {
            item_type: SpotifyItemType::Episode,
            name: SpotifyResourceName::Id(SpotifyId::from_raw(episode.gid())?),
        })
    }
}

impl TryFrom<&protocol::metadata::Track> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(track: &protocol::metadata::Track) -> Result<Self, Self::Error> {
        Ok(Self {
            item_type: SpotifyItemType::Track,
            name: SpotifyResourceName::Id(SpotifyId::from_raw(track.gid())?),
        })
    }
}

impl TryFrom<&protocol::metadata::Show> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(show: &protocol::metadata::Show) -> Result<Self, Self::Error> {
        Ok(Self {
            item_type: SpotifyItemType::Show,
            name: SpotifyResourceName::Id(SpotifyId::from_raw(show.gid())?),
        })
    }
}

impl TryFrom<&protocol::metadata::ArtistWithRole> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(artist: &protocol::metadata::ArtistWithRole) -> Result<Self, Self::Error> {
        Ok(Self {
            item_type: SpotifyItemType::Artist,
            name: SpotifyResourceName::Id(SpotifyId::from_raw(artist.artist_gid())?),
        })
    }
}

impl TryFrom<&protocol::playlist4_external::Item> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(item: &protocol::playlist4_external::Item) -> Result<Self, Self::Error> {
        Ok(Self {
            item_type: SpotifyItemType::Track,
            ..Self::from_uri(item.uri())?
        })
    }
}

// Note that this is the unique revision of an item's metadata on a playlist,
// not the ID of that item or playlist.
impl TryFrom<&protocol::playlist4_external::MetaItem> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(item: &protocol::playlist4_external::MetaItem) -> Result<Self, Self::Error> {
        Ok(Self {
            item_type: SpotifyItemType::Unknown,
            name: SpotifyResourceName::Id(SpotifyId::try_from(item.revision())?),
        })
    }
}

// Note that this is the unique revision of a playlist, not the ID of that playlist.
impl TryFrom<&protocol::playlist4_external::SelectedListContent> for SpotifyUri {
    type Error = crate::Error;
    fn try_from(
        playlist: &protocol::playlist4_external::SelectedListContent,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            item_type: SpotifyItemType::Unknown,
            name: SpotifyResourceName::Id(SpotifyId::try_from(playlist.revision())?),
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
        Ok(Self {
            item_type: SpotifyItemType::Unknown,
            name: SpotifyResourceName::Id(SpotifyId::from_base62(picture.uri())?),
        })
    }
}
