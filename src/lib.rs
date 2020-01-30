pub mod api;

use api::{Likes, Playlists};
use api::likes::LikesRaw;
use api::me::Me;
use api::playlists::PlaylistsRaw;
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
    HttpError(String),
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

/// The `Zester` provides the functionality to "zest" SoundCloud for data once
/// constructed.
/// 
/// Documentation on methods that mentions "the user" is referring to the user
/// whose credentials are provided when creating the struct.
pub struct Zester {
    oauth_token: String,
    client_id: String,
    user_id: Option<i64>
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
            return Ok(resp.into_string()?)
        } else {
            return Err(Error::HttpError(resp.status_line().into()));
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
            user_id: None
        };

        zester.user_id = Some(zester.me()?.id.unwrap());
        Ok(zester)
    }

    /// Get information about the user.
    pub fn me(&self) -> Result<Me, Error> {
        let json_string = self.api_req("me", &[])?;
        Ok(serde_json::from_str(&json_string)?)
    }

    /// Get all of the user's liked tracks.
    // TODO: clean up types. collection applies to more than likes
    pub fn likes(&self) -> Result<Likes, Error> {
        let mut collections = vec![];

        let json_string = self.api_req(
            &format!("users/{}/track_likes", self.user_id.unwrap()),
            &[
                ("limit", "500"),
                ("offset", "0"),
                ("linked_partitioning", "1")
            ]
        )?;

        let mut likes_raw: LikesRaw = serde_json::from_str(&json_string)?;
        collections.extend(likes_raw.collection.unwrap().into_iter());

        // continually grab lists of likes until there are none left
        while let Some(ref next_href) = likes_raw.next_href {
            // sending requests too close together eventually results in 500s
            // being returned
            thread::sleep(Duration::from_millis(2));

            let json_string = self.api_req_full(next_href, &[], true)?;
            likes_raw = serde_json::from_str(&json_string)?;

            collections.extend(likes_raw.collection.unwrap().into_iter());
        }

        Ok(Likes { collections })
    }

    /// Get all of the user's liked and created playlists.
    pub fn playlists(&self) -> Result<Playlists, Error> {
        let mut playlists_info = vec![];
        let mut playlists = vec![];

        let json_string = self.api_req(
            &format!("users/{}/playlists/liked_and_owned", self.user_id.unwrap()),
            &[
                ("limit", "50"),
                ("offset", "0"),
                ("linked_partitioning", "1")
            ]
        )?;

        let mut playlists_raw: PlaylistsRaw = serde_json::from_str(&json_string)?;
        playlists_info.extend(playlists_raw.collection.unwrap().into_iter());

        // continually grab lists of playlists until there are none left
        while let Some(ref next_href) = playlists_raw.next_href {
            // sending requests too close together eventually results in 500s
            // being returned
            thread::sleep(Duration::from_secs(2));

            let json_string = self.api_req_full(next_href, &[], true)?;
            playlists_raw = serde_json::from_str(&json_string)?;

            playlists_info.extend(playlists_raw.collection.unwrap().into_iter());
        }

        // now we need to get the full information about all the playlists, which
        // is what we're actually returning

        for collection in playlists_info {
            // sending requests too close together eventually results in 500s
            // being returned
            // TODO: instead of waiting every time, only start waiting after
            // a 500 occurs
            thread::sleep(Duration::from_secs(2));

            let pinfo = collection.playlist.unwrap();

            // TODO: don't unwrap
            let uri = &pinfo.uri.unwrap();
            let json_string = self.api_req_full(uri, &[("representation", "full")], true)?;
            playlists.push(serde_json::from_str(&json_string)?);
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
