use crate::api::common::Track;
use crate::api::playlists::{PlaylistMeta, Playlist};
use std::io::Read;
use crate::Error;

/// Events that can occur while zesting likes
#[derive(Debug)]
pub enum LikesZestingEvent {
    /// Finished downloading more data about likes.
    ///
    /// This event can occur multiple times.
    MoreLikesInfoDownloaded {
        /// The number of additional likes that info was downloaded for
        count: i64
    },

    /// The server returned an error response and we are waiting for the given
    /// amount of seconds before retrying the request.
    /// 
    /// This event can occur multiple times.
    PausedAfterServerError {
        time_secs: u64
    }
}

/// Events that can occur while zesting track audio.
pub enum TracksAudioZestingEvent<'a> {
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
    /// 
    /// This event can occur multiple times.
    FinishTrackDownload {
        track_info: &'a Track,
        // TODO: replace with impl Read when stable
        track_data: Box<dyn Read>
    },

    /// An error occured while trying to download a track.
    /// 
    /// This event can occur multiple times.
    TrackDownloadError {
        track_info: &'a Track,
        err: Error
    },

    /// The server returned an error response and we are waiting for the given
    /// amount of seconds before retrying the request.
    /// 
    /// This event can occur multiple times.
    PausedAfterServerError {
        time_secs: u64
    }
}

/// Events that can occur while zesting playlists
#[derive(Debug)]
pub enum PlaylistsZestingEvent<'a> {
    /// Finished downloading "meta"-data about `count` more playlists.
    ///
    /// This event can occur multiple times.
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
    /// This event can occur multiple times.
    StartPlaylistInfoDownload {
        playlist_meta: &'a PlaylistMeta
    },

    /// End of downloading full information for another playlist.
    ///
    /// This event can occur multiple times.
    FinishPlaylistInfoDownload {
        playlist_meta: &'a PlaylistMeta
    },

    /// An error occured while downloading playlist info.
    /// 
    /// This event can occur multiple times.
    PlaylistInfoDownloadError {
        playlist_meta: &'a PlaylistMeta,
        err: Error
    },

    /// An error occured while attempting to complete downloaded playlist information.
    /// 
    /// The information will still be returned, but it may not be complete.
    /// 
    /// This event can occur multiple times.
    PlaylistInfoCompletionError {
        playlist_meta: &'a PlaylistMeta,
        err: Error
    },

    /// The server returned an error response and we are waiting for the given
    /// amount of seconds before retrying the request.
    /// 
    /// This event can occur multiple times.
    PausedAfterServerError {
        time_secs: u64
    }
}

/// Events that can occur while zesting audio for playlists
pub enum PlaylistsAudioZestingEvent<'a> {
    /// The number of playlists and tracks that are going to be downloaded.
    ///
    /// This event occurs only once.
    NumItemsToDownload {
        playlists_num: u64,
        tracks_num: u64
    },

    /// Start of downloading a playlist.
    ///
    /// This event can occur multiple times.
    StartPlaylistDownload {
        playlist_info: &'a Playlist
    },

    /// Events related to the downloading of individual tracks.
    TrackEvent(TracksAudioZestingEvent<'a>, &'a Playlist),

    /// Finished downloading a playlist.
    /// 
    /// This event can occur multiple times.
    FinishPlaylistDownload {
        playlist_info: &'a Playlist
    }
}
