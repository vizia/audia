use vizia::prelude::*;

use crate::{
    messages::Track,
    ui::events::{AlbumUiEvent, PlaybackUiEvent, SearchAppEvent, SearchUiEvent},
};

pub struct AlbumState {
    pub showing_album: Signal<bool>,
    pub album_tracks: Signal<Vec<Track>>,
    pub album_name: Signal<String>,
    pub album_artist: Signal<String>,
    pub album_release_year: Signal<Option<u32>>,
    pub album_track_count: Signal<usize>,
    pub album_total_duration_ms: Signal<u64>,
    pub album_image_key: Signal<Option<String>>,
    pub album_selected_index: Signal<usize>,
    pub album_shuffle_mode: Signal<bool>,
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
                release_year,
                track_count,
                total_duration_ms,
            } => {
                self.album_name.set(name.clone());
                self.album_artist.set(artist.clone());
                self.album_release_year.set(*release_year);
                self.album_track_count.set(*track_count);
                self.album_total_duration_ms.set(*total_duration_ms);
                self.album_image_key.set(image_key.clone());
                self.album_tracks.set(tracks.clone());
                self.album_selected_index.set(0);
            }
            SearchAppEvent::Results(_) => {}
        });

        event.map(|ui_event, _: &mut _| match ui_event {
            AlbumUiEvent::BackFromAlbum => {
                self.showing_album.set(false);
            }
            AlbumUiEvent::AlbumTrackSelected(index) => {
                let tracks = self.album_tracks.get();
                if *index >= tracks.len() {
                    return;
                }
                let track = tracks[*index].clone();
                cx.emit(PlaybackUiEvent::AddToQueue(vec![track]));
            }
            AlbumUiEvent::PlayAlbumTrack(index) => {
                let tracks = self.album_tracks.get();
                if *index >= tracks.len() {
                    return;
                }
                let track = tracks[*index].clone();
                cx.emit(PlaybackUiEvent::AddToQueue(vec![track]));
            }
            AlbumUiEvent::PlayAlbum => {
                let tracks = self.album_tracks.get();
                if tracks.is_empty() {
                    return;
                }

                cx.emit(PlaybackUiEvent::AddToQueue(tracks));
                if self.album_shuffle_mode.get() {
                    cx.emit(PlaybackUiEvent::ShuffleQueue);
                }
            }
            AlbumUiEvent::ShuffleAlbum => {
                let current = self.album_shuffle_mode.get();
                self.album_shuffle_mode.set(!current);
            }
        });

        event.map(|ui_event, _: &mut _| match ui_event {
            SearchUiEvent::SubmitQuery(_) => {
                self.showing_album.set(false);
            }
            SearchUiEvent::SelectResult(_)
            | SearchUiEvent::SelectAlbum(_)
            | SearchUiEvent::SetInput(_) => {}
        });
    }
}
