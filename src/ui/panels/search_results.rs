use crate::messages::{Album, Artist, PlaylistEntry, Track};
use crate::ui::events::{PlaylistsUiEvent, SearchUiEvent};
use vizia::icons::ICON_DOTS;
use vizia::prelude::*;

pub fn search_results_panel(
    cx: &mut Context,
    search_result_rows: Signal<Vec<Track>>,
    search_artist_rows: Signal<Vec<Artist>>,
    search_album_rows: Signal<Vec<Album>>,
    search_tabs: Signal<Vec<&'static str>>,
    selected_search_tab: Signal<usize>,
    playlist_rows: Signal<Vec<PlaylistEntry>>,
) {
    fn format_time(ms: u32) -> String {
        let total_seconds = ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{minutes}:{seconds:02}")
    }

    VStack::new(cx, move |cx| {
        Label::new(cx, Localized::new("search_results")).class("panel-title");

        TabView::new(cx, search_tabs, move |_, index, item| match index {
            0 => TabPair::new(
                move |cx| {
                    Label::new(cx, item).hoverable(false);
                    Element::new(cx).class("indicator");
                },
                move |cx| {
                    List::new(cx, search_result_rows, move |cx, _index, item| {
                        HStack::new(cx, |cx| {
                            let image_key = item.map(|track| track.album_image_key.clone());
                            let track_id = item.map(|track| track.id.clone());

                            Binding::new(cx, image_key, move |cx| {
                                let tid = track_id.get();
                                if let Some(key) = image_key.get() {
                                    Image::new(cx, key)
                                        .size(Pixels(48.0))
                                        .class("album-art")
                                        .pointer_events(PointerEvents::Auto)
                                        .on_press(move |cx| {
                                            cx.emit(SearchUiEvent::OpenAlbumFromTrack(tid.clone()))
                                        });
                                } else {
                                    Label::new(cx, "♪")
                                        .size(Pixels(48.0))
                                        .class("search-result-fallback")
                                        .pointer_events(PointerEvents::Auto)
                                        .on_press(move |cx| {
                                            cx.emit(SearchUiEvent::OpenAlbumFromTrack(tid.clone()))
                                        });
                                }
                            });

                            VStack::new(cx, |cx| {
                                Label::new(cx, item.map(|track| track.name.clone()))
                                    .text_wrap(false)
                                    .class("track-title");
                                Label::new(cx, item.map(|track| track.artist.clone()))
                                    .text_wrap(false)
                                    .class("track-artist");
                            })
                            .width(Stretch(1.0))
                            .height(Auto)
                            .gap(Pixels(2.0));

                            Label::new(cx, item.map(|track| format_time(track.duration_ms)))
                                .class("track-duration");

                            // Add to Playlist menu
                            let track_id_for_menu = item.map(|track| track.id.clone());
                            let track_id_copy = track_id_for_menu.get();
                            let playlists_copy = playlist_rows.get();

                            Submenu::new(
                                cx,
                                |cx| Svg::new(cx, ICON_DOTS),
                                move |cx| {
                                    for playlist in &playlists_copy {
                                        let pid = playlist.id.clone();
                                        let tid = track_id_copy.clone();
                                        let pname = playlist.name.clone();

                                        MenuButton::new(
                                            cx,
                                            move |cx| {
                                                if !pid.is_empty() && !tid.is_empty() {
                                                    cx.emit(PlaylistsUiEvent::AddTrackToPlaylist {
                                                        track_id: tid.clone(),
                                                        playlist_id: pid.clone(),
                                                    });
                                                }
                                            },
                                            move |cx| Label::new(cx, format!("{}", pname)),
                                        );
                                    }
                                },
                            )
                            .pointer_events(PointerEvents::Auto)
                            .class("track-menu");
                        })
                        .pointer_events(PointerEvents::None)
                        .class("result-row");
                    })
                    .selectable(Selectable::Single)
                    .selection_follows_focus(true)
                    .on_select(|cx, idx| cx.emit(SearchUiEvent::SelectResult(idx)))
                    .width(Stretch(1.0))
                    .height(Stretch(1.0));
                },
            ),
            1 => TabPair::new(
                move |cx| {
                    Label::new(cx, item).hoverable(false);
                    Element::new(cx).class("indicator");
                },
                move |cx| {
                    List::new(cx, search_artist_rows, |cx, _index, item| {
                        HStack::new(cx, |cx| {
                            let image_key = item.map(|artist| artist.image_key.clone());

                            Binding::new(cx, image_key, move |cx| {
                                if let Some(key) = image_key.get() {
                                    Image::new(cx, key).size(Pixels(40.0)).class("album-art");
                                } else {
                                    Label::new(cx, "◉")
                                        .size(Pixels(40.0))
                                        .class("search-result-fallback");
                                }
                            });

                            Label::new(cx, item.map(|artist| artist.name.clone()))
                                .text_wrap(false)
                                .class("search-result-title")
                                .width(Stretch(1.0));
                        })
                        .hoverable(false)
                        .class("result-row");
                    })
                    .selectable(Selectable::Single)
                    .selection_follows_focus(true)
                    .on_select(|cx, idx| cx.emit(SearchUiEvent::SelectArtist(idx)))
                    .width(Stretch(1.0))
                    .height(Stretch(1.0));
                },
            ),
            2 => TabPair::new(
                move |cx| {
                    Label::new(cx, item).hoverable(false);
                    Element::new(cx).class("indicator");
                },
                move |cx| {
                    List::new(cx, search_album_rows, |cx, _index, item| {
                        HStack::new(cx, |cx| {
                            let image_key = item.map(|album| album.image_key.clone());

                            Binding::new(cx, image_key, move |cx| {
                                if let Some(key) = image_key.get() {
                                    Image::new(cx, key).size(Pixels(40.0)).class("album-art");
                                } else {
                                    Label::new(cx, "◫")
                                        .size(Pixels(40.0))
                                        .class("search-result-fallback");
                                }
                            });

                            VStack::new(cx, |cx| {
                                Label::new(cx, item.map(|album| album.name.clone()))
                                    .text_wrap(false)
                                    .width(Stretch(1.0))
                                    .class("search-result-title");
                                Label::new(cx, item.map(|album| album.artist.clone()))
                                    .text_wrap(false)
                                    .width(Stretch(1.0))
                                    .class("search-result-artist");
                            })
                            .width(Stretch(1.0))
                            .height(Auto)
                            .gap(Pixels(2.0));
                        })
                        .hoverable(false)
                        .class("result-row");
                    })
                    .selectable(Selectable::Single)
                    .selection_follows_focus(true)
                    .on_select(|cx, idx| cx.emit(SearchUiEvent::SelectAlbum(idx)))
                    .width(Stretch(1.0))
                    .height(Stretch(1.0));
                },
            ),
            _ => unreachable!(),
        })
        .with_selected(selected_search_tab)
        .on_select(|cx, index| cx.emit(SearchUiEvent::SelectTab(index)))
        .class("search-tabs")
        .width(Stretch(1.0))
        .height(Stretch(1.0));
    })
    .class("panel")
    .class("search-results-panel")
    .width(Stretch(1.0))
    .height(Stretch(1.0))
    .padding(Pixels(8.0))
    .gap(Pixels(4.0));
}
