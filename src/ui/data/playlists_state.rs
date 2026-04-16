use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use vizia::prelude::*;

use crate::{
    messages::{PlaylistEntry, Track},
    ui::events::{PlaybackUiEvent, PlaylistsAppEvent, PlaylistsUiEvent},
    worker,
};

pub struct PlaylistsState {
    pub backend: crate::worker::SharedBackend,
    pub status: Signal<String>,
    pub playlist_rows: Signal<Vec<PlaylistEntry>>,
    pub playlist_tracks: Signal<Vec<Track>>,
    pub filtered_playlist_tracks: Signal<Vec<Track>>,
    pub filtered_track_indices: Signal<Vec<usize>>,
    pub track_filter_input: Signal<String>,
    pub active_playlist_name: Signal<String>,
    pub active_playlist_track_count: Signal<usize>,
    pub active_playlist_duration_ms: Signal<u64>,
    pub active_playlist_image_key: Signal<Option<String>>,
    pub playlist_selected_index: Signal<usize>,
    pub showing_playlist: Signal<bool>,
    pub shuffle_mode: Signal<bool>,
}

impl PlaylistsState {
    fn apply_track_filter(&mut self) {
        let query = self.track_filter_input.get();
        let tracks = self.playlist_tracks.get();
        let trimmed_query = query.trim();

        if trimmed_query.is_empty() {
            self.filtered_playlist_tracks.set(tracks.clone());
            self.filtered_track_indices
                .set((0..tracks.len()).collect::<Vec<_>>());
            return;
        }

        let matcher = SkimMatcherV2::default();
        let mut matches = tracks
            .iter()
            .enumerate()
            .filter_map(|(index, track)| {
                let haystack = format!("{} {}", track.name, track.artist);
                matcher
                    .fuzzy_match(&haystack, trimmed_query)
                    .map(|score| (index, score, track.clone()))
            })
            .collect::<Vec<_>>();

        matches.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        self.filtered_track_indices
            .set(matches.iter().map(|(index, _, _)| *index).collect());
        self.filtered_playlist_tracks
            .set(matches.into_iter().map(|(_, _, track)| track).collect());
    }
}

impl Model for PlaylistsState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|playlists_event, _: &mut _| match playlists_event {
            PlaylistsAppEvent::Playlists(playlists) => {
                self.playlist_rows.set(playlists.clone());
            }
            PlaylistsAppEvent::PlaylistTracks {
                id,
                name,
                tracks,
                track_count,
                total_duration_ms,
            } => {
                self.active_playlist_name.set(name.clone());
                self.active_playlist_track_count.set(*track_count);
                self.active_playlist_duration_ms.set(*total_duration_ms);
                self.playlist_tracks.set(tracks.clone());
                self.track_filter_input.set(String::new());
                self.apply_track_filter();
                self.playlist_selected_index.set(0);
                self.showing_playlist.set(true);

                let mut playlist_rows = self.playlist_rows.get();
                if let Some(row) = playlist_rows.iter_mut().find(|row| row.id == *id) {
                    row.total_duration_ms = *total_duration_ms;
                    row.track_count = *track_count;
                    self.active_playlist_image_key.set(row.image_key.clone());
                }
                self.playlist_rows.set(playlist_rows);
            }
        });

        event.map(|app_event, _| match app_event {
            PlaylistsUiEvent::ShufflePlaylist => {
                let current = self.shuffle_mode.get();
                self.shuffle_mode.set(!current);
            }
            PlaylistsUiEvent::SetTrackFilter(value) => {
                self.track_filter_input.set(value.clone());
                self.apply_track_filter();
                self.playlist_selected_index.set(0);
            }
            PlaylistsUiEvent::SelectPlaylist(index) => {
                let playlists = self.playlist_rows.get();
                if *index >= playlists.len() {
                    self.status
                        .set("Selected playlist is unavailable.".to_string());
                    return;
                }

                let playlist = playlists[*index].clone();
                self.status
                    .set(format!("Loading playlist '{}'...", playlist.name));
                self.active_playlist_track_count.set(playlist.track_count);
                self.active_playlist_duration_ms
                    .set(playlist.total_duration_ms);
                self.active_playlist_image_key
                    .set(playlist.image_key.clone());
                worker::fetch_playlist_tracks(
                    self.backend.clone(),
                    playlist.id,
                    playlist.name,
                    cx.get_proxy(),
                );
            }
            PlaylistsUiEvent::PlayPlaylist => {
                cx.emit(PlaybackUiEvent::ClearQueue);
                cx.emit(PlaylistsUiEvent::AddPlaylistToQueue);
            }
            PlaylistsUiEvent::AddPlaylistToQueue => {
                let tracks = self.playlist_tracks.get();
                if tracks.is_empty() {
                    self.status
                        .set("Playlist has no tracks to add.".to_string());
                    return;
                }

                cx.emit(PlaybackUiEvent::AddToQueue(tracks));
                if self.shuffle_mode.get() {
                    cx.emit(PlaybackUiEvent::ShuffleQueue);
                }
            }
            PlaylistsUiEvent::PlaylistTrackSelected(index) => {
                let filtered_indices = self.filtered_track_indices.get();
                if *index >= filtered_indices.len() {
                    self.status
                        .set("Selected playlist track is unavailable.".to_string());
                    return;
                }

                self.playlist_selected_index.set(*index);
                let tracks = self.playlist_tracks.get();
                let source_index = filtered_indices[*index];
                let track = tracks[source_index].clone();

                cx.emit(PlaybackUiEvent::AddToQueue(vec![track]));
            }
        });
    }
}
