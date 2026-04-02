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
    pub playlist_selected_index: Signal<usize>,
    pub showing_playlist: Signal<bool>,
}

impl Model for PlaylistsState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|playlists_event, _: &mut _| match playlists_event {
            PlaylistsAppEvent::Playlists(playlists) => {
                self.playlist_rows.set(playlists.clone());
            }
            PlaylistsAppEvent::PlaylistTracks { name, tracks } => {
                self.active_playlist_name.set(name.clone());
                self.playlist_tracks.set(tracks.clone());
                self.playlist_selected_index.set(0);
                self.showing_playlist.set(true);
            }
        });

        event.map(|app_event, _| match app_event {
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
                worker::fetch_playlist_tracks(
                    self.backend.clone(),
                    playlist.id,
                    playlist.name,
                    cx.get_proxy(),
                );
            }
            PlaylistsUiEvent::BackToSearch => {
                self.showing_playlist.set(false);
            }
            PlaylistsUiEvent::AddPlaylistToQueue => {
                let tracks = self.playlist_tracks.get();
                if tracks.is_empty() {
                    self.status
                        .set("Playlist has no tracks to add.".to_string());
                    return;
                }

                cx.emit(PlaybackUiEvent::AddToQueue(tracks));
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
