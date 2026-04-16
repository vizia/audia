use vizia::prelude::*;

use crate::{
    messages::{AlbumResult, ArtistResult, Track},
    ui::events::{PlaybackUiEvent, SearchAppEvent, SearchUiEvent},
    worker,
};

pub struct SearchState {
    pub backend: crate::worker::SharedBackend,
    pub status: Signal<String>,
    pub search_input: Signal<String>,
    pub search_result_rows: Signal<Vec<Track>>,
    pub search_artist_rows: Signal<Vec<ArtistResult>>,
    pub search_album_rows: Signal<Vec<AlbumResult>>,
    pub selected_index: Signal<usize>,
    pub selected_summary: Signal<String>,
    pub showing_playlist: Signal<bool>,
    pub showing_album: Signal<bool>,
}

impl SearchState {
    pub(crate) fn refresh_selected_summary(&mut self) {
        let results = self.search_result_rows.get();
        let summary = if results.is_empty() {
            "Selected: none".to_string()
        } else {
            let idx = self
                .selected_index
                .get()
                .min(results.len().saturating_sub(1));
            self.selected_index.set(idx);
            let track = &results[idx];
            format!("Selected #{}: {} - {}", idx + 1, track.name, track.artist)
        };

        self.selected_summary.set(summary);
    }

    pub(crate) fn set_search_results(
        &mut self,
        tracks: Vec<Track>,
        artists: Vec<ArtistResult>,
        albums: Vec<AlbumResult>,
    ) {
        self.search_result_rows.set(tracks);
        self.search_artist_rows.set(artists);
        self.search_album_rows.set(albums);
    }

    pub(crate) fn refresh_result_selection(&mut self) {
        let results = self.search_result_rows.get();
        let idx = self
            .selected_index
            .get()
            .min(results.len().saturating_sub(1));
        self.selected_index.set(idx);
    }
}

impl Model for SearchState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|search_event, _: &mut _| match search_event {
            SearchAppEvent::Results(results) => {
                let previous_track_ids = self
                    .search_result_rows
                    .get()
                    .iter()
                    .map(|track| track.id.clone())
                    .collect::<Vec<_>>();
                let incoming_track_ids = results
                    .tracks
                    .iter()
                    .map(|track| track.id.clone())
                    .collect::<Vec<_>>();
                let same_track_set = previous_track_ids == incoming_track_ids;

                self.set_search_results(
                    results.tracks.clone(),
                    results.artists.clone(),
                    results.albums.clone(),
                );
                if !same_track_set {
                    self.selected_index.set(0);
                }
                self.refresh_selected_summary();
                self.refresh_result_selection();
            }
            SearchAppEvent::AlbumTracks { .. } => {}
        });

        event.map(|search_ui_event, _: &mut _| match search_ui_event {
            SearchUiEvent::SelectResult(index) => {
                let search_results = self.search_result_rows.get();
                if *index >= search_results.len() {
                    self.status
                        .set("Selected search result is unavailable.".to_string());
                    return;
                }

                let selected_track = search_results[*index].clone();
                cx.emit(PlaybackUiEvent::AddToQueue(vec![selected_track]));
            }
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
            SearchUiEvent::SetInput(value) => {
                self.search_input.set(value.clone());
            }
            SearchUiEvent::SubmitQuery(query) => {
                let query = query.trim().to_string();
                if query.is_empty() {
                    self.status.set("Enter a search query first.".to_string());
                    return;
                }

                self.showing_playlist.set(false);
                self.showing_album.set(false);
                self.status.set(format!("Searching for '{query}'..."));
                worker::search_tracks(self.backend.clone(), query, cx.get_proxy());
            }
        });
    }
}
