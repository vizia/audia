use crate::messages::Album;

#[derive(Clone, Debug)]
pub enum ArtistEvent {
    ArtistView {
        id: String,
        name: String,
        image_key: Option<String>,
        albums: Vec<Album>,
    },
    ArtistAlbumSelected(usize),
}
