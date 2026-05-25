use vizia::prelude::*;

use crate::{
    messages::{Album, Artist, Track},
    ui::{
        events::{CenterUiEvent, PlaybackUiEvent, SearchAppEvent, SearchUiEvent},
        model_data::CenterPage,
    },
    worker,
};

#[derive(Clone)]
pub struct SearchState {
    pub backend: crate::worker::SharedBackend,
    pub selected_search_tab: Signal<usize>,
    pub status: Signal<String>,
    pub search_input: Signal<String>,
    pub search_result_rows: Signal<Vec<Track>>,
    pub search_artist_rows: Signal<Vec<Artist>>,
    pub search_album_rows: Signal<Vec<Album>>,
    pub current_artist_id: Signal<Option<String>>,
    pub current_artist_albums: Signal<Vec<Album>>,
    pub selected_index: Signal<usize>,
    pub selected_summary: Signal<String>,
    pub search_tabs: Signal<Vec<&'static str>>,
    pub active_search_task: Option<TaskHandle>,
    pub active_artist_task: Option<TaskHandle>,
    pub active_album_task: Option<TaskHandle>,
}

impl SearchState {
    pub fn new(backend: crate::worker::SharedBackend, status: Signal<String>) -> Self {
        Self {
            backend,
            selected_search_tab: Signal::new(0),
            status,
            search_input: Signal::new(String::new()),
            search_result_rows: Signal::new(Vec::new()),
            search_artist_rows: Signal::new(Vec::new()),
            search_album_rows: Signal::new(Vec::new()),
            current_artist_id: Signal::new(None),
            current_artist_albums: Signal::new(Vec::new()),
            selected_index: Signal::new(0),
            selected_summary: Signal::new("Selected: none".to_string()),
            search_tabs: Signal::new(vec!["Songs", "Artists", "Albums"]),
            active_search_task: None,
            active_artist_task: None,
            active_album_task: None,
        }
    }

    fn cancel_task(handle: &mut Option<TaskHandle>) {
        if let Some(existing) = handle.take() {
            existing.cancel();
        }
    }
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
        artists: Vec<Artist>,
        albums: Vec<Album>,
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
            SearchAppEvent::HydrateArtwork(results) => {
                Self::cancel_task(&mut self.active_search_task);
                self.active_search_task = Some(worker::hydrate_search_artwork(results.clone(), cx));
            }
            SearchAppEvent::LoadAlbumTracks(album) => {
                self.status
                    .set(format!("Loading tracks for '{}'...", album.name));
                Self::cancel_task(&mut self.active_album_task);
                self.active_album_task = Some(worker::fetch_album_tracks(
                    self.backend.clone(),
                    album.clone(),
                    cx,
                ));
            }
            SearchAppEvent::HydrateAlbumArtwork(data) => {
                Self::cancel_task(&mut self.active_album_task);
                self.active_album_task = Some(worker::hydrate_album_artwork(
                    data.id.clone(),
                    data.name.clone(),
                    data.artist.clone(),
                    data.image_url.clone(),
                    data.tracks.clone(),
                    data.release_year,
                    data.track_count,
                    data.total_duration_ms,
                    cx,
                ));
            }
            SearchAppEvent::HydrateArtistArtwork {
                id,
                name,
                image_url,
                albums,
            } => {
                Self::cancel_task(&mut self.active_artist_task);
                self.active_artist_task = Some(worker::hydrate_artist_artwork(
                    id.clone(),
                    name.clone(),
                    image_url.clone(),
                    albums.clone(),
                    cx,
                ));
            }
            SearchAppEvent::AlbumTracks(_) | SearchAppEvent::ArtistView { .. } => {}
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
                Self::cancel_task(&mut self.active_artist_task);
                self.active_artist_task = Some(worker::fetch_artist_view(
                    self.backend.clone(),
                    artist.id,
                    cx,
                ));
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
                Self::cancel_task(&mut self.active_album_task);
                self.active_album_task =
                    Some(worker::fetch_album_tracks(self.backend.clone(), album, cx));
            }
            SearchUiEvent::OpenAlbumFromTrack(track_id) => {
                self.status.set("Loading album...".to_string());
                cx.emit(CenterUiEvent::NavigateTo(CenterPage::AlbumTracks));
                Self::cancel_task(&mut self.active_album_task);
                self.active_album_task = Some(worker::fetch_album_from_track(
                    self.backend.clone(),
                    track_id.clone(),
                    cx,
                ));
            }
            SearchUiEvent::OpenArtistFromTrack(track_id) => {
                self.status
                    .set("Loading artist from current track...".to_string());
                cx.emit(CenterUiEvent::NavigateTo(CenterPage::Artist));
                Self::cancel_task(&mut self.active_artist_task);
                self.active_artist_task = Some(worker::fetch_artist_view_from_track(
                    self.backend.clone(),
                    track_id.clone(),
                    cx,
                ));
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
                Self::cancel_task(&mut self.active_search_task);
                self.active_search_task =
                    Some(worker::search_tracks(self.backend.clone(), query, cx));
            }
        });
    }
}
