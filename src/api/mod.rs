pub mod likes;
pub mod me;

use serde_derive::{Serialize, Deserialize};
use likes::{Track, Quality, Protocol, Collection};
use crate::{Error, Zester};
use std::io::prelude::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct Likes {
    pub collections: Vec<Collection>,
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
