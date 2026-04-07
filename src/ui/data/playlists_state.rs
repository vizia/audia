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
    pub active_playlist_name: Signal<String>,
    pub active_playlist_meta: Signal<String>,
    pub playlist_selected_index: Signal<usize>,
    pub showing_playlist: Signal<bool>,
    pub shuffle_mode: Signal<bool>,
}

impl Model for PlaylistsState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        fn format_playlist_meta(track_count: usize, total_duration_ms: u64) -> String {
            let total_seconds = total_duration_ms / 1000;
            let hours = total_seconds / 3600;
            let minutes = (total_seconds % 3600) / 60;
            let seconds = total_seconds % 60;

            let song_label = if track_count == 1 { "song" } else { "songs" };
            let duration = if hours > 0 {
                format!("{hours}:{minutes:02}:{seconds:02}")
            } else {
                format!("{minutes}:{seconds:02}")
            };

            format!("{track_count} {song_label} • {duration}")
        }

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
                self.active_playlist_meta
                    .set(format_playlist_meta(*track_count, *total_duration_ms));
                self.playlist_tracks.set(tracks.clone());
                self.playlist_selected_index.set(0);
                self.showing_playlist.set(true);

                let mut playlist_rows = self.playlist_rows.get();
                if let Some(row) = playlist_rows.iter_mut().find(|row| row.id == *id) {
                    row.total_duration_ms = *total_duration_ms;
                    row.track_count = *track_count;
                }
                self.playlist_rows.set(playlist_rows);
            }
        });

        event.map(|app_event, _| match app_event {
            PlaylistsUiEvent::ShufflePlaylist => {
                let current = self.shuffle_mode.get();
                self.shuffle_mode.set(!current);
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
                self.active_playlist_meta.set(format_playlist_meta(
                    playlist.track_count,
                    playlist.total_duration_ms,
                ));
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
                let tracks = self.playlist_tracks.get();
                if *index >= tracks.len() {
                    self.status
                        .set("Selected playlist track is unavailable.".to_string());
                    return;
                }

                self.playlist_selected_index.set(*index);
                let track = tracks[*index].clone();

                cx.emit(PlaybackUiEvent::AddToQueue(vec![track]));
            }
        });
    }
}
