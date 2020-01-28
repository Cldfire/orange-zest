mod api;

use api::likes::{Likes, Collection};
use api::me::Me;
use std::thread;
use std::time::Duration;

const API_BASE: &str = "https://api-v2.soundcloud.com/";

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    JsonDecodeError(serde_json::Error),
    HttpError(String)
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
    fn api_req_full(&self, path: &str, query_params: &[(&str, &str)]) -> Result<String, Error> {
        let mut r = ureq::get(path);
    
        for param in query_params {
            r.query(param.0, param.1);
        }
    
        r.query("client_id", &self.client_id);
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
        self.api_req_full(&format!("{}{}", API_BASE, path), query_params)
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
    pub fn likes(&self) -> Result<Vec<Collection>, Error> {
        let mut collections = vec![];

        let json_string = self.api_req(
            &format!("users/{}/track_likes", self.user_id.unwrap()),
            &[
                ("limit", "500"),
                ("offset", "0"),
                ("linked_partitioning", "1")
            ]
        )?;

        let mut likes: Likes = serde_json::from_str(&json_string)?;
        collections.extend(likes.collection.unwrap().into_iter());

        // continually grab lists of likes until there are none left
        while let Some(ref next_href) = likes.next_href {
            // sending requests too close together eventually results in 500s
            // being returned
            thread::sleep(Duration::from_millis(2_000));

            let json_string = self.api_req_full(next_href, &[])?;
            likes = serde_json::from_str(&json_string)?;

            collections.extend(likes.collection.unwrap().into_iter());
        }

        Ok(collections)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    // TODO: temporary test, remove or improve
    #[test]
    fn likes() -> Result<(), Error> {
        let zester = Zester::new("".into(), "".into())?;
        let likes = zester.likes()?;

        println!("{:#?}", likes);

        Ok(())
    }
}