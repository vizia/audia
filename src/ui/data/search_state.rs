use vizia::prelude::*;

use crate::{
    messages::{Album, Artist, Track},
    ui::{
        events::{CenterPanelEvent, PlaybackEvent, SearchEvent},
        model_data::CenterPage,
    },
    worker,
};

#[derive(Clone)]
pub struct SearchState {
    pub backend: crate::worker::SharedBackend,
    pub search_tabs: Signal<Vec<&'static str>>,
    pub selected_search_tab: Signal<usize>,
    pub status: Signal<String>,
    pub search_input: Signal<String>,
    pub search_track_rows: Signal<Vec<Track>>,
    pub search_artist_rows: Signal<Vec<Artist>>,
    pub search_album_rows: Signal<Vec<Album>>,
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
            search_track_rows: Signal::new(Vec::new()),
            search_artist_rows: Signal::new(Vec::new()),
            search_album_rows: Signal::new(Vec::new()),
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

    pub(crate) fn set_search_results(
        &mut self,
        tracks: Vec<Track>,
        artists: Vec<Artist>,
        albums: Vec<Album>,
    ) {
        self.search_track_rows.set(tracks);
        self.search_artist_rows.set(artists);
        self.search_album_rows.set(albums);
    }
}

impl Model for SearchState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|search_event, _| match search_event {
            SearchEvent::Results(results) => {
                self.set_search_results(
                    results.tracks.clone(),
                    results.artists.clone(),
                    results.albums.clone(),
                );
            }
            SearchEvent::HydrateArtwork(results) => {
                Self::cancel_task(&mut self.active_search_task);
                self.active_search_task = Some(worker::hydrate_search_artwork(results.clone(), cx));
            }
            SearchEvent::LoadAlbumTracks(album) => {
                self.status
                    .set(format!("Loading tracks for '{}'...", album.name));
                Self::cancel_task(&mut self.active_album_task);
                self.active_album_task = Some(worker::fetch_album_tracks(
                    self.backend.clone(),
                    album.clone(),
                    cx,
                ));
            }
            SearchEvent::HydrateAlbumArtwork(data) => {
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
            SearchEvent::HydrateArtistArtwork {
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
            SearchEvent::SelectTab(index) => {
                self.selected_search_tab.set(*index);
            }
            SearchEvent::SelectTrack(index) => {
                let search_results = self.search_track_rows.get();
                if *index >= search_results.len() {
                    self.status
                        .set("Selected search result is unavailable.".to_string());
                    return;
                }

                let selected_track = search_results[*index].clone();
                cx.emit(PlaybackEvent::AddToQueue(vec![selected_track]));
            }
            SearchEvent::SelectArtist(index) => {
                let artists = self.search_artist_rows.get();
                if *index >= artists.len() {
                    self.status
                        .set("Selected artist is unavailable.".to_string());
                    return;
                }

                let artist = artists[*index].clone();

                self.status
                    .set(format!("Loading albums for '{}'...", artist.name));
                cx.emit(CenterPanelEvent::NavigateTo(CenterPage::Artist));
                Self::cancel_task(&mut self.active_artist_task);
                self.active_artist_task = Some(worker::fetch_artist_view(
                    self.backend.clone(),
                    artist.id,
                    cx,
                ));
            }
            SearchEvent::SelectAlbum(index) => {
                let albums = self.search_album_rows.get();
                if *index >= albums.len() {
                    self.status
                        .set("Selected album is unavailable.".to_string());
                    return;
                }

                let album = albums[*index].clone();
                self.status
                    .set(format!("Loading tracks for '{}'...", album.name));
                cx.emit(CenterPanelEvent::NavigateTo(CenterPage::AlbumTracks));
                Self::cancel_task(&mut self.active_album_task);
                self.active_album_task =
                    Some(worker::fetch_album_tracks(self.backend.clone(), album, cx));
            }
            SearchEvent::OpenAlbumFromTrack(track_id) => {
                self.status.set("Loading album...".to_string());
                cx.emit(CenterPanelEvent::NavigateTo(CenterPage::AlbumTracks));
                Self::cancel_task(&mut self.active_album_task);
                self.active_album_task = Some(worker::fetch_album_from_track(
                    self.backend.clone(),
                    track_id.clone(),
                    cx,
                ));
            }
            SearchEvent::OpenArtistFromTrack(track_id) => {
                self.status
                    .set("Loading artist from current track...".to_string());
                cx.emit(CenterPanelEvent::NavigateTo(CenterPage::Artist));
                Self::cancel_task(&mut self.active_artist_task);
                self.active_artist_task = Some(worker::fetch_artist_view_from_track(
                    self.backend.clone(),
                    track_id.clone(),
                    cx,
                ));
            }

            SearchEvent::SetInput(value) => {
                self.search_input.set(value.clone());
            }
            SearchEvent::SubmitQuery(query) => {
                let query = query.trim().to_string();
                if query.is_empty() {
                    self.status.set("Enter a search query first.".to_string());
                    return;
                }

                cx.emit(CenterPanelEvent::NavigateTo(CenterPage::Search));
                self.status.set(format!("Searching for '{query}'..."));
                Self::cancel_task(&mut self.active_search_task);
                self.active_search_task =
                    Some(worker::search_tracks(self.backend.clone(), query, cx));
            }
        });
    }
}
