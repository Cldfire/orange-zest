// Generated by https://app.quicktype.io/ with a few hand edits
//
// Turn on derive debug impl and make all properties optional

// TODO: a lot of stuff that could use enums to strongly type certain properties
// can't right now because serde doesn't support deserializing to an enum with
// a non-unit variant fallback to capture the value as a string if it doesn't
// match one of the existing unit variants
//
// see https://github.com/serde-rs/serde/pull/1382#issuecomment-424706998

use serde_derive::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LikesRaw {
    pub collection: Option<Vec<Collection>>,
    pub next_href: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Collection {
    pub created_at: Option<String>,
    // Made this non-optional since it will always be present here
    pub track: Track,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Track {
    pub comment_count: Option<i64>,
    pub full_duration: Option<i64>,
    pub downloadable: Option<bool>,
    pub created_at: Option<String>,
    pub description: Option<String>,
    pub media: Option<Media>,
    pub title: Option<String>,
    pub publisher_metadata: Option<PublisherMetadata>,
    pub duration: Option<i64>,
    pub has_downloads_left: Option<bool>,
    pub artwork_url: Option<String>,
    pub public: Option<bool>,
    pub streamable: Option<bool>,
    pub tag_list: Option<String>,
    pub download_url: Option<String>,
    pub genre: Option<String>,
    pub id: Option<i64>,
    pub reposts_count: Option<i64>,
    pub state: Option<String>,
    pub label_name: Option<String>,
    pub last_modified: Option<String>,
    pub commentable: Option<bool>,
    pub policy: Option<String>,
    pub visuals: Option<Visuals>,
    pub kind: Option<String>,
    pub purchase_url: Option<String>,
    pub sharing: Option<String>,
    pub uri: Option<String>,
    pub download_count: Option<i64>,
    pub likes_count: Option<i64>,
    pub urn: Option<String>,
    pub license: Option<String>,
    pub purchase_title: Option<String>,
    pub display_date: Option<String>,
    pub embeddable_by: Option<String>,
    pub release_date: Option<String>,
    pub user_id: Option<i64>,
    pub monetization_model: Option<String>,
    pub waveform_url: Option<String>,
    pub permalink: Option<String>,
    pub permalink_url: Option<String>,
    pub user: Option<User>,
    pub playback_count: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Media {
    pub transcodings: Option<Vec<Transcoding>>,
}

// As far as I can tell none of these fields need to be optional
#[derive(Debug, Serialize, Deserialize)]
pub struct Transcoding {
    pub url: String,
    pub preset: String,
    pub duration: i64,
    pub snipped: bool,
    pub format: Format,
    pub quality: Quality,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Format {
    pub protocol: Protocol,
    pub mime_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublisherMetadata {
    pub urn: Option<String>,
    pub contains_music: Option<bool>,
    pub id: Option<i64>,
    pub artist: Option<String>,
    pub writer_composer: Option<String>,
    pub publisher: Option<String>,
    pub isrc: Option<String>,
    pub album_title: Option<String>,
    pub release_title: Option<String>,
    pub p_line_for_display: Option<String>,
    pub p_line: Option<String>,
    pub explicit: Option<bool>,
    pub upc_or_ean: Option<String>,
    pub c_line: Option<String>,
    pub c_line_for_display: Option<String>,
    pub iswc: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub avatar_url: Option<String>,
    pub first_name: Option<String>,
    pub full_name: Option<String>,
    pub id: Option<i64>,
    pub kind: Option<String>,
    pub last_modified: Option<String>,
    pub last_name: Option<String>,
    pub permalink: Option<String>,
    pub permalink_url: Option<String>,
    pub uri: Option<String>,
    pub urn: Option<String>,
    pub username: Option<String>,
    pub verified: Option<bool>,
    pub city: Option<String>,
    pub country_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Visuals {
    pub urn: Option<String>,
    pub enabled: Option<bool>,
    pub visuals: Option<Vec<Visual>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Visual {
    pub urn: Option<String>,
    pub entry_time: Option<i64>,
    pub visual_url: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Protocol {
    #[serde(rename = "hls")]
    Hls,
    #[serde(rename = "progressive")]
    Progressive,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum Quality {
    #[serde(rename = "hq")]
    Hq,
    #[serde(rename = "sq")]
    Sq,
}
