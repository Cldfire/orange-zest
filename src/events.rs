use crate::api::common::Track;
use crate::api::playlists::PlaylistMeta;
use std::io::Read;

/// Events that can occur while zesting likes
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

/// Events that can occur while zesting audio for likes
pub enum LikesAudioZestingEvent<'a> {
    /// The number of tracks that are going to be downloaded.
    ///
    /// This event occurs only once.
    NumTracksToDownload {
        num: u64
    },

    /// Start of downloading a track.
    ///
    /// This event can occur multiple times.
    StartTrackDownload {
        track_info: &'a Track
    },

    /// Finished downloading a track.
    ///
    /// `track_data` is a `Read` instance that you can use to access the data.
    FinishTrackDownload {
        track_info: &'a Track,
        // TODO: replace with impl Read when stable
        track_data: Box<dyn Read>
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
