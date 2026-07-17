use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use vizia::prelude::*;

use crate::{
    messages::{PlaylistEntry, Track},
    ui::{
        events::{CenterPanelEvent, PlaybackEvent, PlaylistsEvent},
        model_data::CenterPage,
    },
    worker,
};

#[derive(Clone)]
pub struct PlaylistsState {
    pub backend: crate::worker::SharedBackend,
    pub status: Signal<String>,
    pub show_create_playlist_modal: Signal<bool>,
    pub create_playlist_name: Signal<String>,
    pub is_creating_playlist: Signal<bool>,
    pub show_rename_playlist_modal: Signal<bool>,
    pub rename_playlist_id: Signal<String>,
    pub rename_playlist_name: Signal<String>,
    pub is_renaming_playlist: Signal<bool>,
    pub playlist_rows: Signal<Vec<PlaylistEntry>>,
    pub playlist_tracks: Signal<Vec<Track>>,
    pub filtered_playlist_tracks: Signal<Vec<Track>>,
    pub filtered_track_indices: Signal<Vec<usize>>,
    pub track_filter_input: Signal<String>,
    pub active_playlist_id: Signal<Option<String>>,
    pub active_playlist_name: Signal<String>,
    pub active_playlist_track_count: Signal<usize>,
    pub active_playlist_duration_ms: Signal<u64>,
    pub active_playlist_image_key: Signal<Option<String>>,
    pub playlist_selected_index: Signal<usize>,
    pub shuffle_mode: Signal<bool>,
    pub current_playlist_request_id: u64,
    pub playlist_track_filter_input: Signal<String>,
    pub active_playlist_task: Option<TaskHandle>,
}

impl PlaylistsState {
    pub fn new(backend: crate::worker::SharedBackend, status: Signal<String>) -> Self {
        Self {
            backend,
            status,
            show_create_playlist_modal: Signal::new(false),
            create_playlist_name: Signal::new(String::new()),
            is_creating_playlist: Signal::new(false),
            show_rename_playlist_modal: Signal::new(false),
            rename_playlist_id: Signal::new(String::new()),
            rename_playlist_name: Signal::new(String::new()),
            is_renaming_playlist: Signal::new(false),
            playlist_rows: Signal::new(Vec::new()),
            playlist_tracks: Signal::new(Vec::new()),
            filtered_playlist_tracks: Signal::new(Vec::new()),
            filtered_track_indices: Signal::new(Vec::new()),
            track_filter_input: Signal::new(String::new()),
            active_playlist_id: Signal::new(None),
            active_playlist_name: Signal::new(String::new()),
            active_playlist_track_count: Signal::new(0),
            active_playlist_duration_ms: Signal::new(0),
            active_playlist_image_key: Signal::new(None),
            playlist_selected_index: Signal::new(0),
            shuffle_mode: Signal::new(false),
            current_playlist_request_id: 0,
            playlist_track_filter_input: Signal::new(String::new()),
            active_playlist_task: None,
        }
    }

    fn cancel_task(handle: &mut Option<TaskHandle>) {
        if let Some(existing) = handle.take() {
            existing.cancel();
        }
    }

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
            PlaylistsEvent::Playlists(playlists) => {
                self.playlist_rows.set(playlists.clone());
            }
            PlaylistsEvent::RefreshUserPlaylists => {
                worker::refresh_user_playlists(self.backend.clone(), cx);
            }
            PlaylistsEvent::RefreshPlaylistTracks {
                request_id,
                id,
                name,
            } => {
                self.current_playlist_request_id = *request_id;
                Self::cancel_task(&mut self.active_playlist_task);
                self.active_playlist_task = Some(worker::fetch_playlist_tracks(
                    self.backend.clone(),
                    id.clone(),
                    name.clone(),
                    *request_id,
                    cx,
                ));
            }
            PlaylistsEvent::PlaylistCreated { id, name } => {
                self.is_creating_playlist.set(false);
                self.show_create_playlist_modal.set(false);
                self.create_playlist_name.set(String::new());
                self.status.set(format!("Created playlist '{name}'."));
                self.active_playlist_id.set(Some(id.clone()));
                self.active_playlist_name.set(name.clone());
            }
            PlaylistsEvent::PlaylistCreateFailed(message) => {
                self.is_creating_playlist.set(false);
                self.status.set(message.clone());
            }
            PlaylistsEvent::PlaylistRenamed { id, name } => {
                self.is_renaming_playlist.set(false);
                self.show_rename_playlist_modal.set(false);
                self.rename_playlist_name.set(String::new());
                self.status.set(format!("Renamed playlist to '{name}'."));
                if self.active_playlist_id.get().as_deref() == Some(id.as_str()) {
                    self.active_playlist_name.set(name.clone());
                }
            }
            PlaylistsEvent::PlaylistRenameFailed(message) => {
                self.is_renaming_playlist.set(false);
                self.status.set(message.clone());
            }
            PlaylistsEvent::PlaylistDeleted(id) => {
                if self.active_playlist_id.get().as_deref() == Some(id.as_str()) {
                    self.active_playlist_id.set(None);
                    self.active_playlist_name.set(String::new());
                    self.playlist_tracks.set(Vec::new());
                    self.filtered_playlist_tracks.set(Vec::new());
                }
                self.status.set("Playlist removed.".to_string());
            }
            PlaylistsEvent::PlaylistDeleteFailed(message) => {
                self.status.set(message.clone());
            }
            PlaylistsEvent::PlaylistTracks {
                request_id,
                id,
                name,
                tracks,
                track_count,
                total_duration_ms,
            } => {
                if *request_id != self.current_playlist_request_id {
                    return;
                }

                self.active_playlist_id.set(Some(id.clone()));
                self.active_playlist_name.set(name.clone());
                self.active_playlist_track_count.set(*track_count);
                self.active_playlist_duration_ms.set(*total_duration_ms);
                self.playlist_tracks.set(tracks.clone());
                self.track_filter_input.set(String::new());
                self.apply_track_filter();
                self.playlist_selected_index.set(0);
                cx.emit(CenterPanelEvent::NavigateTo(CenterPage::PlaylistTracks));

                let mut playlist_rows = self.playlist_rows.get();
                if let Some(row) = playlist_rows.iter_mut().find(|row| row.id == *id) {
                    row.total_duration_ms = *total_duration_ms;
                    row.track_count = *track_count;
                    self.active_playlist_image_key.set(row.image_key.clone());
                }
                self.playlist_rows.set(playlist_rows);
            }
            PlaylistsEvent::OpenCreatePlaylistModal => {
                self.show_create_playlist_modal.set(true);
                self.create_playlist_name.set(String::new());
            }
            PlaylistsEvent::CloseCreatePlaylistModal => {
                self.show_create_playlist_modal.set(false);
                self.create_playlist_name.set(String::new());
                self.is_creating_playlist.set(false);
            }
            PlaylistsEvent::SetCreatePlaylistName(value) => {
                self.create_playlist_name.set(value.clone());
            }
            PlaylistsEvent::SubmitCreatePlaylist => {
                if self.is_creating_playlist.get() {
                    return;
                }

                let playlist_name = self.create_playlist_name.get();
                let trimmed_name = playlist_name.trim();
                if trimmed_name.is_empty() {
                    self.status
                        .set("Please enter a playlist name before creating it.".to_string());
                    return;
                }

                self.is_creating_playlist.set(true);
                self.status
                    .set(format!("Creating playlist '{}'...", trimmed_name));
                worker::create_playlist(self.backend.clone(), trimmed_name.to_string(), cx);
            }
            PlaylistsEvent::OpenRenamePlaylistModal { id, name } => {
                self.rename_playlist_id.set(id.clone());
                self.rename_playlist_name.set(name.clone());
                self.show_rename_playlist_modal.set(true);
            }
            PlaylistsEvent::CloseRenamePlaylistModal => {
                self.show_rename_playlist_modal.set(false);
                self.rename_playlist_name.set(String::new());
                self.is_renaming_playlist.set(false);
            }
            PlaylistsEvent::SetRenamePlaylistName(value) => {
                self.rename_playlist_name.set(value.clone());
            }
            PlaylistsEvent::SubmitRenamePlaylist => {
                if self.is_renaming_playlist.get() {
                    return;
                }

                let new_name = self.rename_playlist_name.get();
                let trimmed = new_name.trim();
                if trimmed.is_empty() {
                    self.status.set("Please enter a playlist name.".to_string());
                    return;
                }

                self.is_renaming_playlist.set(true);
                self.status
                    .set(format!("Renaming playlist to '{trimmed}'..."));
                worker::rename_playlist(
                    self.backend.clone(),
                    self.rename_playlist_id.get(),
                    trimmed.to_string(),
                    cx,
                );
            }
            PlaylistsEvent::DeletePlaylist(playlist_id) => {
                self.status.set("Removing playlist...".to_string());
                worker::delete_playlist(self.backend.clone(), playlist_id.clone(), cx);
            }
            PlaylistsEvent::ShufflePlaylist => {
                let current = self.shuffle_mode.get();
                self.shuffle_mode.set(!current);
            }
            PlaylistsEvent::SetTrackFilter(value) => {
                self.track_filter_input.set(value.clone());
                self.apply_track_filter();
                self.playlist_selected_index.set(0);
            }
            PlaylistsEvent::SelectPlaylist(index) => {
                let playlists = self.playlist_rows.get();
                if *index >= playlists.len() {
                    self.status
                        .set("Selected playlist is unavailable.".to_string());
                    return;
                }

                let playlist = playlists[*index].clone();
                if self.active_playlist_id.get().as_deref() == Some(playlist.id.as_str())
                    && !self.playlist_tracks.get().is_empty()
                {
                    self.status
                        .set(format!("Showing cached playlist '{}'...", playlist.name));
                    cx.emit(CenterPanelEvent::NavigateTo(CenterPage::PlaylistTracks));
                    return;
                }

                self.status
                    .set(format!("Loading playlist '{}'...", playlist.name));
                self.active_playlist_track_count.set(playlist.track_count);
                self.active_playlist_duration_ms
                    .set(playlist.total_duration_ms);
                self.active_playlist_image_key
                    .set(playlist.image_key.clone());

                self.current_playlist_request_id =
                    self.current_playlist_request_id.saturating_add(1);

                Self::cancel_task(&mut self.active_playlist_task);
                self.active_playlist_task = Some(worker::fetch_playlist_tracks(
                    self.backend.clone(),
                    playlist.id,
                    playlist.name,
                    self.current_playlist_request_id,
                    cx,
                ));
            }
            PlaylistsEvent::PlayPlaylist => {
                cx.emit(PlaybackEvent::ClearQueue);
                cx.emit(PlaylistsEvent::AddPlaylistToQueue);
            }
            PlaylistsEvent::AddPlaylistToQueue => {
                let tracks = self.playlist_tracks.get();
                if tracks.is_empty() {
                    self.status
                        .set("Playlist has no tracks to add.".to_string());
                    return;
                }

                cx.emit(PlaybackEvent::AddToQueue(tracks));
                if self.shuffle_mode.get() {
                    cx.emit(PlaybackEvent::ShuffleQueue);
                }
            }
            PlaylistsEvent::PlaylistTrackSelected(index) => {
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

                cx.emit(PlaybackEvent::AddToQueue(vec![track]));
            }
            PlaylistsEvent::AddTrackToPlaylist {
                track_id,
                playlist_id,
            } => {
                self.status.set("Adding track to playlist...".to_string());
                worker::add_track_to_playlist(
                    self.backend.clone(),
                    track_id.clone(),
                    playlist_id.clone(),
                    cx,
                );
            }
            PlaylistsEvent::RemoveTrackFromPlaylist {
                track_id,
                playlist_id,
            } => {
                self.status
                    .set("Removing track from playlist...".to_string());
                self.current_playlist_request_id =
                    self.current_playlist_request_id.saturating_add(1);
                Self::cancel_task(&mut self.active_playlist_task);
                worker::remove_track_from_playlist(
                    self.backend.clone(),
                    track_id.clone(),
                    playlist_id.clone(),
                    self.active_playlist_name.get(),
                    self.current_playlist_request_id,
                    cx,
                );
            }
        });
    }
}
