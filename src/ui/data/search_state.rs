use vizia::prelude::*;

use crate::{
    messages::{AlbumResult, ArtistResult, Track},
    ui::{
        events::{CenterUiEvent, PlaybackUiEvent, SearchAppEvent, SearchUiEvent},
        model_data::CenterPage,
    },
    worker,
};

pub struct SearchState {
    pub backend: crate::worker::SharedBackend,
    pub selected_search_tab: Signal<usize>,
    pub status: Signal<String>,
    pub search_input: Signal<String>,
    pub search_result_rows: Signal<Vec<Track>>,
    pub search_artist_rows: Signal<Vec<ArtistResult>>,
    pub search_album_rows: Signal<Vec<AlbumResult>>,
    pub current_artist_id: Signal<Option<String>>,
    pub current_artist_albums: Signal<Vec<AlbumResult>>,
    pub selected_index: Signal<usize>,
    pub selected_summary: Signal<String>,
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
            SearchAppEvent::AlbumTracks { .. } | SearchAppEvent::ArtistView { .. } => {}
        });

        event.map(|search_ui_event, _: &mut _| match search_ui_event {
            SearchUiEvent::SelectTab(index) => {
                self.selected_search_tab.set(*index);
            }
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
            SearchUiEvent::SelectArtist(index) => {
                let artists = self.search_artist_rows.get();
                if *index >= artists.len() {
                    self.status
                        .set("Selected artist is unavailable.".to_string());
                    return;
                }

                let artist = artists[*index].clone();
                if self.current_artist_id.get().as_deref() == Some(artist.id.as_str())
                    && !self.current_artist_albums.get().is_empty()
                {
                    self.status
                        .set(format!("Showing cached albums for '{}'", artist.name));
                    cx.emit(CenterUiEvent::NavigateTo(CenterPage::Artist));
                    return;
                }

                self.status
                    .set(format!("Loading albums for '{}'...", artist.name));
                cx.emit(CenterUiEvent::NavigateTo(CenterPage::Artist));
                worker::fetch_artist_view(self.backend.clone(), artist.id, cx.get_proxy());
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
                cx.emit(CenterUiEvent::NavigateTo(CenterPage::AlbumTracks));
                worker::fetch_album_tracks(self.backend.clone(), album, cx.get_proxy());
            }
            SearchUiEvent::OpenAlbumFromTrack(track_id) => {
                self.status.set("Loading album...".to_string());
                cx.emit(CenterUiEvent::NavigateTo(CenterPage::AlbumTracks));
                worker::fetch_album_from_track(
                    self.backend.clone(),
                    track_id.clone(),
                    cx.get_proxy(),
                );
            }
            SearchUiEvent::OpenArtistFromTrack(track_id) => {
                self.status
                    .set("Loading artist from current track...".to_string());
                cx.emit(CenterUiEvent::NavigateTo(CenterPage::Artist));
                worker::fetch_artist_view_from_track(
                    self.backend.clone(),
                    track_id.clone(),
                    cx.get_proxy(),
                );
            }
            SearchUiEvent::OpenArtistByName(artist_name) => {
                let artist_name = artist_name.trim().to_string();
                if artist_name.is_empty() {
                    self.status
                        .set("No artist name available from playback.".to_string());
                    return;
                }

                self.status
                    .set(format!("Loading albums for '{}'...", artist_name));
                cx.emit(CenterUiEvent::NavigateTo(CenterPage::Artist));
                worker::fetch_artist_view_by_name(
                    self.backend.clone(),
                    artist_name,
                    cx.get_proxy(),
                );
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

                cx.emit(CenterUiEvent::NavigateTo(CenterPage::Search));
                self.status.set(format!("Searching for '{query}'..."));
                worker::search_tracks(self.backend.clone(), query, cx.get_proxy());
            }
        });
    }
}
