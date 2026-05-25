use vizia::prelude::*;

use crate::{
    messages::Track,
    ui::events::{AlbumEvent, PlaybackEvent},
};

#[derive(Clone)]
pub struct AlbumState {
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

impl AlbumState {
    pub fn new() -> Self {
        Self {
            album_tracks: Signal::new(Vec::new()),
            album_name: Signal::new(String::new()),
            album_artist: Signal::new(String::new()),
            album_release_year: Signal::new(None),
            album_track_count: Signal::new(0),
            album_total_duration_ms: Signal::new(0),
            album_image_key: Signal::new(None),
            album_selected_index: Signal::new(0),
            album_shuffle_mode: Signal::new(false),
        }
    }
}

impl Model for AlbumState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|album_event, _: &mut _| match album_event {
            AlbumEvent::AlbumTracks(data) => {
                let _ = &data.id;
                self.album_name.set(data.name.clone());
                self.album_artist.set(data.artist.clone());
                self.album_release_year.set(data.release_year);
                self.album_track_count.set(data.track_count);
                self.album_total_duration_ms.set(data.total_duration_ms);
                self.album_image_key.set(data.image_key.clone());
                self.album_tracks.set(data.tracks.clone());
                self.album_selected_index.set(0);
            }
            AlbumEvent::AlbumTrackSelected(index) => {
                let tracks_len = self.album_tracks.with(|tracks| tracks.len());
                if *index >= tracks_len {
                    return;
                }
                let track = self.album_tracks.with(|tracks| tracks[*index].clone());
                cx.emit(PlaybackEvent::AddToQueue(vec![track]));
            }
            AlbumEvent::PlayAlbum => {
                if self.album_tracks.with(|tracks| tracks.is_empty()) {
                    return;
                }

                let tracks = self.album_tracks.get();
                cx.emit(PlaybackEvent::ClearQueue);
                cx.emit(PlaybackEvent::AddToQueue(tracks));
                if self.album_shuffle_mode.get() {
                    cx.emit(PlaybackEvent::ShuffleQueue);
                }
            }
            AlbumEvent::ToggleShuffleAlbum => {
                self.album_shuffle_mode.update(|value| *value = !*value);
            }
        });
    }
}
