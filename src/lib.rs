pub mod api;

use api::{Likes, Playlists};
use api::likes::LikesRaw;
use api::me::Me;
use api::playlists::{PlaylistsRaw, PlaylistMeta};
use std::thread;
use std::time::Duration;
use std::path::Path;
use std::fs::File;
use std::io::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

const API_BASE: &str = "https://api-v2.soundcloud.com/";

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    JsonDecodeError(serde_json::Error),
    /// Contains the response status code
    HttpError(u16),
    /// Something we needed wasn't present in the JSON
    ///
    /// (The "something" will be described by the string.)
    DataNotPresent(String)
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonDecodeError(err)
    }
}

/// Load an object from a JSON file at the given path.
pub fn load_json<P: AsRef<Path>, O: DeserializeOwned>(path: P) -> Result<O, Error> {
    let mut file = File::open(path)?;
    let mut string = String::new();
    file.read_to_string(&mut string)?;

    Ok(serde_json::from_str(&string)?)
}

/// Write an object to a JSON file at the given path.
pub fn write_json<P: AsRef<Path>, O: Serialize>(object: &O, path: P, pretty_print: bool) -> Result<(), Error> {
    let bytes = if pretty_print {
        serde_json::to_string_pretty(object)?.into_bytes()
    } else {
        serde_json::to_string(object)?.into_bytes()
    };

    let mut file = File::create(path)?;
    file.write_all(&bytes)?;
    Ok(())
}

/// Events that can occur while zesting playlists
#[derive(Debug)]
pub enum LikesZestingEvent {
    /// Finished downloading more data about likes.
    ///
    /// This event can occur more than once.
    MoreLikesInfoDownloaded {
        /// The number of additional likes that info was downloaded for
        count: i64
    },

    /// The server returned an error response and we are waiting for the given
    /// amount of seconds before retrying the request.
    PausedAfterServerError {
        time_secs: u64
    }
}

/// Events that can occur while zesting playlists
#[derive(Debug)]
pub enum PlaylistsZestingEvent<'a> {
    /// Finished downloading "meta"-data about `count` more playlists.
    ///
    /// This event can occur more than once.
    MorePlaylistMetaInfoDownloaded {
        /// The number of additional playlists that info was downloaded for
        count: i64
    },

    /// Finished downloading "meta"-data for all playlists.
    ///
    /// This event occurs only once.
    FinishPlaylistMetaInfoDownloading,

    /// Start of downloading full information for another playlist.
    ///
    /// This event can occur more than once.
    StartPlaylistInfoDownload {
        /// The name of the playlist info is being downloaded for
        playlist_meta: &'a PlaylistMeta
    },

    /// End of downloading full information for another playlist.
    ///
    /// This event can occur more than once.
    FinishPlaylistInfoDownload,

    /// The server returned an error response and we are waiting for the given
    /// amount of seconds before retrying the request.
    PausedAfterServerError {
        time_secs: u64
    }
}

/// The `Zester` provides the functionality to "zest" SoundCloud for data once
/// constructed.
/// 
/// Documentation on methods that mentions "the user" is referring to the user
/// whose credentials are provided when creating the struct.
pub struct Zester {
    oauth_token: String,
    client_id: String,
    pub me: Option<Me>
}

impl Zester {
    // An internal function that streamlines the process of making an API request
    // TODO: support compression when ureq does
    fn api_req_full(
        &self,
        path: &str,
        query_params: &[(&str, &str)],
        add_client_id: bool
    ) -> Result<String, Error> {
        let mut r = ureq::get(path);

        for param in query_params {
            r.query(param.0, param.1);
        }

        if add_client_id {
            r.query("client_id", &self.client_id);
        }
        r.set("Authorization", &format!("OAuth {}", &self.oauth_token));
        r.timeout_connect(10_000); // 10 second timeout

        let resp = r.call();

        if resp.ok() {
            Ok(resp.into_string()?)
        } else {
            Err(Error::HttpError(resp.status()))
        }
    }

    // Calls the above but concats with the base URL inside the fn to avoid verbosity
    fn api_req(&self, path: &str, query_params: &[(&str, &str)]) -> Result<String, Error> {
        self.api_req_full(&format!("{}{}", API_BASE, path), query_params, true)
    }

    /// Construct a new `Zester` with the given credentials.
    /// 
    /// This will send a request to the "/me" api route to determine the id of
    /// the user whose credentials you provided.
    // TODO: docs on how to get credentials
    pub fn new(oauth_token: String, client_id: String) -> Result<Self, Error> {
        let mut zester = Self {
            oauth_token,
            client_id,
            me: None
        };

        zester.me = Some(zester.me()?);
        Ok(zester)
    }

    /// Get information about the user.
    pub fn me(&self) -> Result<Me, Error> {
        let json_string = self.api_req("me", &[])?;
        Ok(serde_json::from_str(&json_string)?)
    }

    /// Get all of the user's liked tracks.
    ///
    /// The optionally-provided callback will be called when various events occur,
    /// allowing you to handle them as you please.
    pub fn likes<F: Fn(LikesZestingEvent)>(&self, cb: Option<F>) -> Result<Likes, Error> {
        use LikesZestingEvent::*;
        let pause_secs = 2;

        let mut collections = vec![];

        let json_string = self.api_req(
            &format!("users/{}/track_likes", self.me.as_ref().unwrap().id.unwrap()),
            &[
                ("limit", "500"),
                ("offset", "0"),
                ("linked_partitioning", "1")
            ]
        )?;

        let mut likes_raw: LikesRaw = serde_json::from_str(&json_string)?;
        let likes_count = likes_raw.collection.as_ref().unwrap().len();
        collections.extend(likes_raw.collection.unwrap().into_iter());
        if let Some(cb) = cb.as_ref() {
            cb(MoreLikesInfoDownloaded { count: likes_count as i64 });
        }

        // continually grab lists of likes until there are none left
        while let Some(ref next_href) = likes_raw.next_href {
            let json_string = match self.api_req_full(next_href, &[], true) {
                Ok(s) => s,
                Err(Error::HttpError(code)) if code >= 500 && code < 600 => {
                    // the server responded with an error. waiting a couple of seconds
                    // and then trying again seems to resolve this, so that's
                    // what we'll do
                    // TODO: completely bail out if max retry count reached?

                    if let Some(cb) = cb.as_ref() {
                        cb(PausedAfterServerError { time_secs: pause_secs });
                    }
                    thread::sleep(Duration::from_secs(pause_secs));
                    continue;
                },
                Err(e) => return Err(e)
            };
            likes_raw = serde_json::from_str(&json_string)?;

            let likes_count = likes_raw.collection.as_ref().unwrap().len();
            collections.extend(likes_raw.collection.unwrap().into_iter());
            if let Some(cb) = cb.as_ref() {
                cb(MoreLikesInfoDownloaded { count: likes_count as i64 });
            }
        }

        Ok(Likes { collections })
    }

    /// Get all of the user's liked and created playlists.
    ///
    /// The optionally-provided callback will be called when various events occur,
    /// allowing you to handle them as you please.
    pub fn playlists<F: Fn(PlaylistsZestingEvent)>(&self, cb: Option<F>) -> Result<Playlists, Error> {
        use PlaylistsZestingEvent::*;
        let pause_secs = 2;

        let mut playlists_info = vec![];
        let mut playlists = vec![];

        let json_string = self.api_req(
            &format!("users/{}/playlists/liked_and_owned", self.me.as_ref().unwrap().id.unwrap()),
            &[
                ("limit", "50"),
                ("offset", "0"),
                ("linked_partitioning", "1")
            ]
        )?;

        let mut playlists_raw: PlaylistsRaw = serde_json::from_str(&json_string)?;
        let mut playlists_count = playlists_raw.collection.as_ref().unwrap().len();
        playlists_info.extend(playlists_raw.collection.unwrap().into_iter());
        if let Some(cb) = cb.as_ref() {
            cb(MorePlaylistMetaInfoDownloaded { count: playlists_count as i64});
        }

        // continually grab lists of playlists until there are none left
        while let Some(ref next_href) = playlists_raw.next_href {
            let json_string = match self.api_req_full(next_href, &[], true) {
                Ok(s) => s,
                Err(Error::HttpError(code)) if code >= 500 && code < 600 => {
                    // the server responded with an error. waiting a couple of seconds
                    // and then trying again seems to resolve this, so that's
                    // what we'll do

                    if let Some(cb) = cb.as_ref() {
                        cb(PausedAfterServerError { time_secs: pause_secs });
                    }
                    thread::sleep(Duration::from_secs(pause_secs));
                    continue;
                },
                Err(e) => return Err(e)
            };

            playlists_raw = serde_json::from_str(&json_string)?;

            playlists_count = playlists_raw.collection.as_ref().unwrap().len();
            playlists_info.extend(playlists_raw.collection.unwrap().into_iter());
            if let Some(cb) = cb.as_ref() {
                cb(MorePlaylistMetaInfoDownloaded { count: playlists_count as i64 });
            }
        }

        if let Some(cb) = cb.as_ref() {
            cb(FinishPlaylistMetaInfoDownloading);
        }

        let mut playlists_info_iter = playlists_info.into_iter();
        let mut collection = playlists_info_iter.next();

        // now we need to get the full information about all the playlists, which
        // is what we're actually returning
        while let Some(c) = collection.as_ref() {
            let pmeta = c.playlist.as_ref().unwrap();
            if let Some(cb) = cb.as_ref() {
                cb(StartPlaylistInfoDownload { playlist_meta: &pmeta });
            }

            // TODO: don't unwrap
            let uri = pmeta.uri.as_ref().unwrap();
            let json_string = match self.api_req_full(uri, &[("representation", "full")], true) {
                Ok(s) => s,
                Err(Error::HttpError(code)) if code >= 500 && code < 600 => {
                    // the server responded with an error. waiting a couple of seconds
                    // and then trying again seems to resolve this, so that's
                    // what we'll do

                    if let Some(cb) = cb.as_ref() {
                        cb(PausedAfterServerError { time_secs: pause_secs });
                    }
                    thread::sleep(Duration::from_secs(pause_secs));
                    continue;
                },
                Err(e) => return Err(e)
            };
            playlists.push(serde_json::from_str(&json_string)?);
            if let Some(cb) = cb.as_ref() {
                cb(FinishPlaylistInfoDownload);
            }

            collection = playlists_info_iter.next();
        }

        Ok(Playlists { playlists })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // TODO: temporary test, remove or improve
    #[test]
    fn likes() -> Result<(), Error> {
        let _zester = Zester::new("".into(), "".into())?;

        Ok(())
    }
}
