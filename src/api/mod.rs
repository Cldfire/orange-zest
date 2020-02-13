pub mod common;
pub mod likes;
pub mod me;
pub mod playlists;

use serde_derive::{Serialize, Deserialize};
use common::{Track, Quality, Protocol};
use playlists::Playlist;
use likes::LikesCollection;
use me::Me;
use crate::{Error, Zester};
use std::io::prelude::*;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

// TODO: fix naming discrepancies between fields of structs
#[derive(Debug, Serialize, Deserialize)]
pub struct Likes {
    pub collections: Vec<LikesCollection>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Playlists {
    pub playlists: Vec<Playlist>,
}

impl Track {
    /// Download the track's associated audio file and return a `Read` instance
    /// providing the data.
    pub fn download(&self, zester: &Zester) -> Result<impl Read, Error> {
        // first we need to determine what we're downloading
        let info_url;
        if let Some(media) = &self.media {
            if let Some(transcodings) = &media.transcodings {
                    // TODO: make selection more robust
                    // right now we just look for the first progressive stream that's
                    // also high-quality and bail out if we don't find one

                    // TODO: also going to have to support HLS
                    // some tracks only have HLS streams available for download
                    if let Some(transcoding) = transcodings
                        .iter()
                        .find(|t|
                            t.quality == Quality::Hq &&
                            t.format.protocol == Protocol::Progressive
                        ) {
                        info_url = &transcoding.url;
                    } else {
                        return Err(Error::DataNotPresent("desired transcoding".into()))
                    }
            } else {
                return Err(Error::DataNotPresent("transcodings information".into()))
            }
        } else {
            return Err(Error::DataNotPresent("media information".into()))
        }

        // now we use the URL we got to get the actual URL to the media file
        let info_json: serde_json::Value = serde_json::from_str(&zester.api_req_full(info_url, &[], false)?)?;
        if let Some(url) = info_json.get("url") {
            Ok(ureq::get(url.as_str().unwrap()).call().into_reader())
        } else {
            Err(Error::DataNotPresent("media file url in info json".into()))
        }
    }
}

impl Me {
    pub fn total_playlist_count(&self) -> i64 {
        self.playlist_count.unwrap() +
            self.playlist_likes_count.unwrap() +
            self.private_playlists_count.unwrap()
    }
}

impl Playlist {
    /// Make sure all info is present for all tracks in this playlist.
    /// 
    /// I noticed during the implementation of downloading the audio for all of
    /// a playlist's tracks that often the track data returned by the playlist
    /// API is not complete (and is notably lacking the media URLs which we of
    /// course need).
    /// 
    /// This method fixes that by making some batch requests for track info
    // TODO: add event hooks
    pub fn complete_tracks_info(&mut self, zester: &Zester) -> Result<(), Error> {
        let mut track_ids_to_complete = vec![];
        let mut info_map = HashMap::new();
        let pause_secs = 2;

        let tracks = if let Some(tracks) = &self.tracks {
            tracks
        } else {
            return Ok(());
        };

        for track in tracks.iter() {
            if track.media.is_none() {
                track_ids_to_complete.push(track.id.unwrap() as u64);
            }
        }

        let mut chunks_iter = track_ids_to_complete.chunks(10);
        let mut maybe_chunk = chunks_iter.next();
        while let Some(ids) = maybe_chunk {
            for track in match zester.tracks_info(ids) {
                Ok(t) => t,
                Err(Error::HttpError(code)) if code >= 500 && code < 600 => {
                    // the server responded with an error. waiting a couple of seconds
                    // and then trying again seems to resolve this, so that's
                    // what we'll do
                    thread::sleep(Duration::from_secs(pause_secs));
                    continue;
                },
                Err(e) => return Err(e)
            } {
                info_map.insert(track.id.unwrap(), track);
            }

            maybe_chunk = chunks_iter.next();
        }

        // Replace info in this playlist with the info we obtained
        for track in self.tracks.as_mut().unwrap().iter_mut() {
            if let Some(updated_track) = info_map.remove(track.id.as_ref().unwrap()) {
                *track = updated_track;
            }
        }

        Ok(())
    }
}
