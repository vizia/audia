use vizia::prelude::*;

use crate::{
    messages::{AlbumResult, Track},
    ui::events::{PlaybackUiEvent, SearchAppEvent, SearchUiEvent},
    worker,
};

pub struct AlbumState {
    pub backend: crate::worker::SharedBackend,
    pub status: Signal<String>,
    pub showing_playlist: Signal<bool>,
    pub showing_album: Signal<bool>,
    pub search_album_rows: Signal<Vec<AlbumResult>>,
    pub album_tracks: Signal<Vec<Track>>,
    pub album_name: Signal<String>,
    pub album_artist: Signal<String>,
    pub album_image_key: Signal<Option<String>>,
    pub album_selected_index: Signal<usize>,
}

impl Model for AlbumState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _: &mut _| match app_event {
            SearchAppEvent::AlbumTracks {
                id: _,
                name,
                artist,
                image_key,
                tracks,
            } => {
                self.album_name.set(name.clone());
                self.album_artist.set(artist.clone());
                self.album_image_key.set(image_key.clone());
                self.album_tracks.set(tracks.clone());
                self.album_selected_index.set(0);
            }
            SearchAppEvent::Results(_) => {}
        });

        event.map(|ui_event, _: &mut _| match ui_event {
            SearchUiEvent::SelectAlbum(index) => {
                let albums = self.search_album_rows.get();
                if *index >= albums.len() {
                    self.status
                        .set("Selected album is unavailable.".to_string());
                    return;
                }

                let album = albums[*index].clone();
                self.status
                    .set(format!("Loading tracks for '{}'...", album.name));
                self.showing_playlist.set(false);
                self.showing_album.set(true);
                worker::fetch_album_tracks(self.backend.clone(), album, cx.get_proxy());
            }
            SearchUiEvent::OpenAlbumFromPlayback {
                track_id,
                image_key,
                image_url,
            } => {
                if let Some(track_id) = track_id {
                    self.status.set("Loading album from current track...".to_string());
                    self.showing_playlist.set(false);
                    self.showing_album.set(true);
                    worker::fetch_album_from_track(self.backend.clone(), track_id.clone(), cx.get_proxy());
                    return;
                }

                let current_album_key = self.album_image_key.get();
                if image_key.is_some()
                    && *image_key == current_album_key
                    && !self.album_tracks.get().is_empty()
                {
                    self.showing_playlist.set(false);
                    self.showing_album.set(true);
                    return;
                }

                let albums = self.search_album_rows.get();
                let by_key = image_key.as_ref().and_then(|key| {
                    albums
                        .iter()
                        .find(|album| album.image_key.as_ref() == Some(key))
                        .cloned()
                });
                let by_url = image_url.as_ref().and_then(|url| {
                    albums
                        .iter()
                        .find(|album| album.image_url.as_ref() == Some(url))
                        .cloned()
                });

                if let Some(album) = by_key.or(by_url) {
                    self.status
                        .set(format!("Loading tracks for '{}'...", album.name));
                    self.showing_playlist.set(false);
                    self.showing_album.set(true);
                    worker::fetch_album_tracks(self.backend.clone(), album, cx.get_proxy());
                } else {
                    self.status
                        .set("Album not found in current search results.".to_string());
                }
            }
            SearchUiEvent::BackFromAlbum => {
                self.showing_album.set(false);
            }
            SearchUiEvent::AlbumTrackSelected(index) => {
                let tracks = self.album_tracks.get();
                if *index >= tracks.len() {
                    return;
                }
                let track = tracks[*index].clone();
                cx.emit(PlaybackUiEvent::AddToQueue(vec![track]));
            }
            SearchUiEvent::SubmitQuery(_) => {
                self.showing_album.set(false);
            }
            SearchUiEvent::SelectResult(_) | SearchUiEvent::SetInput(_) => {}
        });
    }
}
