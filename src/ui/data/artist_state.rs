use vizia::prelude::*;

use crate::{
    messages::Album,
    ui::{
        events::{ArtistUiEvent, CenterUiEvent, SearchAppEvent},
        model_data::CenterPage,
    },
    worker,
};

#[derive(Clone)]
pub struct ArtistState {
    pub backend: crate::worker::SharedBackend,
    pub status: Signal<String>,
    pub artist_id: Signal<Option<String>>,
    pub artist_name: Signal<String>,
    pub artist_image_key: Signal<Option<String>>,
    pub artist_albums: Signal<Vec<Album>>,
}

impl ArtistState {
    pub fn new(backend: crate::worker::SharedBackend, status: Signal<String>) -> Self {
        Self {
            backend,
            status,
            artist_id: Signal::new(None),
            artist_name: Signal::new(String::new()),
            artist_image_key: Signal::new(None),
            artist_albums: Signal::new(Vec::new()),
        }
    }
}

impl Model for ArtistState {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _: &mut _| match app_event {
            SearchAppEvent::ArtistView {
                id,
                name,
                image_key,
                albums,
            } => {
                self.artist_id.set(Some(id.clone()));
                self.artist_name.set(name.clone());
                self.artist_image_key.set(image_key.clone());
                self.artist_albums.set(albums.clone());
            }
            SearchAppEvent::Results(_) | SearchAppEvent::AlbumTracks { .. } => {}
        });

        event.map(|ui_event, _: &mut _| match ui_event {
            ArtistUiEvent::ArtistAlbumSelected(index) => {
                let albums = self.artist_albums.get();
                if *index >= albums.len() {
                    self.status
                        .set("Selected artist album is unavailable.".to_string());
                    return;
                }

                let album = albums[*index].clone();
                self.status
                    .set(format!("Loading tracks for '{}'...", album.name));
                cx.emit(CenterUiEvent::NavigateTo(CenterPage::AlbumTracks));
                worker::fetch_album_tracks(self.backend.clone(), album, cx.get_proxy());
            }
        });
    }
}
