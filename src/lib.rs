pub mod api;
pub mod events;

use api::{Likes, Playlists};
use api::likes::LikesRaw;
use api::me::Me;
use api::common::Track;
use api::playlists::{Playlist, PlaylistsRaw};
use events::*;
use std::thread;
use std::time::Duration;
use std::path::Path;
use std::fs::File;
use std::cmp::min;
use std::io::prelude::*;
use serde::Serialize;
use serde::de::DeserializeOwned;

const API_BASE: &str = "https://api-v2.soundcloud.com/";
/// Amount of time to pause after a 500 is returned from the server
const PAUSE_SECS: u64 = 2;

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

/// An enum of values to return from the body of a `retry_loop`
enum LoopControl {
    /// Continue on to the next iteration
    Next,
    /// Retry the current iteration
    Retry
}


/// Loop through the given iterator with the ability to "retry" any iteration
/// from the top of the loop body.
fn retry_loop<'a, T, I, F>(mut iter: I, mut loop_body: F)
    where
        T: Copy,
        I: Iterator<Item = T>,
        F: FnMut(T) -> LoopControl
{
    let mut maybe_val = iter.next();

    while let Some(val) = maybe_val {
        match loop_body(val) {
            LoopControl::Next => maybe_val = iter.next(),
            LoopControl::Retry => {},
        }
    }
}

/// Returns true if the given HTTP status code is a 500
fn is_500(code: u16) -> bool {
    code >= 500 && code < 600
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
    /// The callback you provide will be called when various events occur,
    /// allowing you to handle them as you please.
    pub fn likes<F: Fn(LikesZestingEvent)>(&self, cb: F) -> Result<Likes, Error> {
        use LikesZestingEvent::*;

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

        cb(MoreLikesInfoDownloaded { count: likes_count as i64 });

        // continually grab lists of likes until there are none left
        while let Some(ref next_href) = likes_raw.next_href {
            let json_string = match self.api_req_full(next_href, &[], true) {
                Ok(s) => s,
                Err(Error::HttpError(code)) if is_500(code) => {
                    // the server responded with an error. waiting a couple of seconds
                    // and then trying again seems to resolve this, so that's
                    // what we'll do
                    // TODO: completely bail out if max retry count reached?

                    cb(PausedAfterServerError { time_secs: PAUSE_SECS });
                    thread::sleep(Duration::from_secs(PAUSE_SECS));
                    continue;
                },
                Err(e) => return Err(e)
            };

            likes_raw = serde_json::from_str(&json_string)?;
            let likes_count = likes_raw.collection.as_ref().unwrap().len();

            collections.extend(likes_raw.collection.unwrap().into_iter());
            cb(MoreLikesInfoDownloaded { count: likes_count as i64 });
        }

        Ok(Likes { collections })
    }

    /// Download the audio files for the given `Likes`.
    ///
    /// The provided callback will be called when various events occur,
    /// allowing you to handle them as you please.
    ///
    /// Of particular note, one of the events the callback will hand you gives
    /// you access to the downloaded audio data for you to use however works
    /// best for your use-case.
    ///
    /// `num_recent` specifies the number of recent likes to download.
    pub fn likes_audio<F: Fn(TracksAudioZestingEvent)>(
        &self,
        likes: &Likes,
        num_recent: u64,
        cb: F
    ) -> Result<(), Error> {
        use TracksAudioZestingEvent::*;

        let download_num = min(num_recent as usize, likes.collections.len());
        cb(NumTracksToDownload { num: download_num as u64 });

        self.tracks_audio(
            likes.collections.iter().map(|c| &c.track).take(download_num),
            |e| cb(e)
        )?;

        Ok(())
    }

    /// Get all of the user's liked and created playlists.
    ///
    /// The callback you provide will be called when various events occur,
    /// allowing you to handle them as you please.
    pub fn playlists<F: Fn(PlaylistsZestingEvent)>(&self, cb: F) -> Result<Playlists, Error> {
        use PlaylistsZestingEvent::*;

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

        cb(MorePlaylistMetaInfoDownloaded { count: playlists_count as i64});

        // continually grab lists of playlists until there are none left
        while let Some(ref next_href) = playlists_raw.next_href {
            let json_string = match self.api_req_full(next_href, &[], true) {
                Ok(s) => s,
                Err(Error::HttpError(code)) if code >= 500 && code < 600 => {
                    // the server responded with an error. waiting a couple of seconds
                    // and then trying again seems to resolve this, so that's
                    // what we'll do
                    cb(PausedAfterServerError { time_secs: PAUSE_SECS });
                    thread::sleep(Duration::from_secs(PAUSE_SECS));

                    continue;
                },
                Err(e) => return Err(e)
            };

            playlists_raw = serde_json::from_str(&json_string)?;

            playlists_count = playlists_raw.collection.as_ref().unwrap().len();
            playlists_info.extend(playlists_raw.collection.unwrap().into_iter());

            cb(MorePlaylistMetaInfoDownloaded { count: playlists_count as i64 });
        }

        cb(FinishPlaylistMetaInfoDownloading);
        
        // now we need to get the full information about all the playlists, which
        // is what we're actually returning
        retry_loop(playlists_info.iter(), |c| {
            let pmeta = c.playlist.as_ref().unwrap();
            cb(StartPlaylistInfoDownload { playlist_meta: &pmeta });

            // TODO: don't unwrap
            let uri = pmeta.uri.as_ref().unwrap();
            match self.api_req_full(&uri.replace("api.", "api-v2."), &[("representation", "full")], true) {
                Ok(s) => {
                    let mut playlist: Playlist = match serde_json::from_str(&s) {
                        Ok(p) => p,
                        Err(e) => {
                            cb(PlaylistInfoDownloadError { playlist_meta: &pmeta, err: Error::from(e) });
                            return LoopControl::Next;
                        }
                    };

                    // Make sure the track information is complete
                    if let Err(e) = playlist.complete_tracks_info(self) {
                        cb(PlaylistInfoCompletionError { playlist_meta: &pmeta, err: e });
                    };
                    playlists.push(playlist);
        
                    cb(FinishPlaylistInfoDownload { playlist_meta: &pmeta });
                    LoopControl::Next
                },
                Err(Error::HttpError(code)) if is_500(code) => {
                    // the server responded with an error. waiting a couple of seconds
                    // and then trying again seems to resolve this, so that's
                    // what we'll do

                    cb(PausedAfterServerError { time_secs: PAUSE_SECS });
                    thread::sleep(Duration::from_secs(PAUSE_SECS));
                    LoopControl::Retry
                },
                Err(e) => {
                    cb(PlaylistInfoDownloadError { playlist_meta: &pmeta, err: e });
                    LoopControl::Next
                }
            }
        });

        Ok(Playlists { playlists })
    }

    /// Download the the audio files for all of the user's playlists.
    ///
    /// The optionally-provided callback will be called when various events occur,
    /// allowing you to handle them as you please.
    ///
    /// Of particular note, one of the events the callback will hand you gives
    /// you access to the downloaded audio data for you to use however works
    /// best for your use-case.
    pub fn playlists_audio<'a, I, F>(
        &self,
        playlists: I,
        cb: F
    ) -> Result<(), Error> where
        I: Iterator<Item = &'a Playlist>,
        F: Fn(PlaylistsAudioZestingEvent)
    {
        use PlaylistsAudioZestingEvent::*;
        
        let playlist_refs: Vec<_> = playlists.collect();
        let tracks_num = playlist_refs.iter().map(|p| p.tracks.as_ref().unwrap().len() as u64).sum();
        cb(NumItemsToDownload { playlists_num: playlist_refs.len() as u64, tracks_num });
    
        let mut playlists_iter = playlist_refs.into_iter();
        let mut maybe_playlist = playlists_iter.next();

        while let Some(playlist_info) = maybe_playlist.as_ref() {
            cb(StartPlaylistDownload { playlist_info });

            self.tracks_audio(
                playlist_info.tracks.as_ref().unwrap().iter(),
                |e| cb(TrackEvent(e, playlist_info))
            )?;

            cb(FinishPlaylistDownload { playlist_info });
            maybe_playlist = playlists_iter.next();
        }
    
        Ok(())
    }

    /// Download the audio files for each track in the given iterator.
    ///
    /// The provided callback will be called when various events occur, allowing
    /// you to handle them as you please.
    ///
    /// Of particular note, one of the events the callback will hand you gives
    /// you access to the downloaded audio data for you to use however works
    /// best for your use-case.
    pub fn tracks_audio<'a, I: Iterator<Item = &'a Track>, F: Fn(TracksAudioZestingEvent)>(
        &self,
        tracks: I,
        cb: F
    ) -> Result<(), Error> {
        use TracksAudioZestingEvent::*;

        let track_refs: Vec<_> = tracks.collect();
        cb(NumTracksToDownload { num: track_refs.len() as u64 });

        retry_loop(track_refs.into_iter(), |track| {
            cb(StartTrackDownload { track_info: &track });

            match track.download(self) {
                Ok(r) => {
                    cb(FinishTrackDownload { track_info: track, track_data: Box::new(r) });
                    LoopControl::Next
                },
                Err(Error::HttpError(code)) if is_500(code) => {
                    // the server responded with an error. waiting a couple of seconds
                    // and then trying again seems to resolve this, so that's
                    // what we'll do

                    cb(PausedAfterServerError { time_secs: PAUSE_SECS });
                    thread::sleep(Duration::from_secs(PAUSE_SECS));
                    LoopControl::Retry
                },
                Err(e) => {
                    cb(TrackDownloadError { track_info: track, err: e });
                    LoopControl::Next
                }
            }
        });

        Ok(())
    }

    /// Get information for the specified track IDs.
    pub fn tracks_info<A: AsRef<[u64]>>(&self, ids: A) -> Result<Vec<Track>, Error> {
        let mut ids_string = String::new();

        for id in ids.as_ref() {
            ids_string.push_str(&id.to_string());
            ids_string.push(',');
        }
        ids_string.pop();

        Ok(serde_json::from_str(&self.api_req(
            "tracks",
            &[("ids", &ids_string)]
        )?)?)
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
