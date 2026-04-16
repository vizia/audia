use crate::ui::events::PlaybackUiEvent;
use vizia::icons::{
    ICON_PLAYER_PAUSE_FILLED, ICON_PLAYER_PLAY_FILLED, ICON_PLAYER_SKIP_BACK_FILLED,
    ICON_PLAYER_SKIP_FORWARD_FILLED, ICON_VOLUME, ICON_VOLUME_OFF,
};
use vizia::prelude::*;

pub fn playback_controls_panel(
    cx: &mut Context,
    playback_device_options: Signal<Vec<String>>,
    selected_playback_device_index: Signal<Option<usize>>,
    playback_is_playing: Signal<bool>,
    playback_volume: Signal<f32>,
    playback_is_muted: Signal<bool>,
    playback_scrub_percent: Signal<f32>,
    playback_duration_ms: Signal<u32>,
    playback_track_name: Signal<String>,
    playback_track_artist: Signal<String>,
    playback_track_image_key: Signal<Option<String>>,
    playback_track_id: Signal<Option<String>>,
    playback_track_image_url: Signal<Option<String>>,
) {
    fn format_time(ms: u32) -> String {
        let total_seconds = ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{minutes}:{seconds:02}")
    }

    let duration_played = playback_scrub_percent.map(move |percent| {
        let played_ms = (percent / 100.0) * (playback_duration_ms.get() as f32);
        format_time(played_ms.round() as u32)
    });

    let duration_remaining = playback_scrub_percent.map(move |percent| {
        let remaining_ms = ((100.0 - percent) / 100.0) * (playback_duration_ms.get() as f32);
        format_time(remaining_ms.round() as u32)
    });

    HStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Binding::new(cx, playback_track_image_key, move |cx| {
                if let Some(image_key) = playback_track_image_key.get() {
                    let image_key_for_view = image_key.clone();
                    let image_key_for_event = image_key.clone();
                    let image_url = playback_track_image_url;
                    Button::new(cx, move |cx| {
                        Image::new(cx, image_key_for_view.clone()).class("album-art")
                    })
                    .variant(ButtonVariant::Text)
                    .class("playback-album-button")
                    .on_press(move |cx| {
                        cx.emit(PlaybackUiEvent::OpenAlbumFromPlayback {
                            track_id: playback_track_id.get(),
                            image_key: Some(image_key_for_event.clone()),
                            image_url: image_url.get(),
                        });
                    });
                } else {
                    Label::new(cx, "♪").size(Pixels(60.0));
                }
            });

            VStack::new(cx, |cx| {
                Label::new(cx, playback_track_name)
                    .text_wrap(false)
                    .class("now-playing-title");
                Label::new(cx, playback_track_artist)
                    .text_wrap(false)
                    .class("now-playing-artist");
            })
            .height(Auto)
            .width(Stretch(1.0))
            .gap(Pixels(2.0));
        })
        .alignment(Alignment::Left)
        .height(Auto)
        .width(Stretch(1.0))
        .gap(Pixels(8.0));

        VStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                Button::new(cx, |cx| Svg::new(cx, ICON_PLAYER_SKIP_BACK_FILLED))
                    .class("playback-skip-back")
                    .name("Skip Back")
                    .on_press(|cx| cx.emit(PlaybackUiEvent::Previous));

                ToggleButton::with_contents(
                    cx,
                    playback_is_playing,
                    |cx| Svg::new(cx, ICON_PLAYER_PLAY_FILLED),
                    |cx| Svg::new(cx, ICON_PLAYER_PAUSE_FILLED),
                )
                .class("playback-toggle")
                .on_toggle(|cx| cx.emit(PlaybackUiEvent::Toggle));

                Button::new(cx, |cx| Svg::new(cx, ICON_PLAYER_SKIP_FORWARD_FILLED))
                    .class("playback-skip-forward")
                    .name("Skip Forward")
                    .on_press(|cx| cx.emit(PlaybackUiEvent::Next));
            })
            .class("transport")
            .height(Auto)
            .width(Auto)
            .alignment(Alignment::Center)
            .gap(Pixels(8.0));

            HStack::new(cx, |cx| {
                Label::new(cx, duration_played)
                    .alignment(Alignment::Right)
                    .width(Pixels(40.0));
                Slider::new(cx, playback_scrub_percent)
                    .range(0.0..100.0)
                    .on_change(|cx, val| cx.emit(PlaybackUiEvent::SetScrub(val)))
                    .width(Stretch(1.0));
                Label::new(
                    cx,
                    duration_remaining.map(|remaining| format!("-{remaining}")),
                )
                .width(Pixels(40.0));
            })
            .height(Auto)
            .width(Stretch(1.0))
            .alignment(Alignment::Center)
            .gap(Pixels(4.0));
        })
        .height(Auto)
        .width(Stretch(1.0))
        .alignment(Alignment::Center)
        .gap(Pixels(4.0));

        HStack::new(cx, |cx| {
            HStack::new(cx, |cx| {
                ToggleButton::with_contents(
                    cx,
                    playback_is_muted,
                    |cx| Svg::new(cx, ICON_VOLUME),
                    |cx| Svg::new(cx, ICON_VOLUME_OFF),
                )
                .class("playback-volume-toggle")
                .on_toggle(|cx| cx.emit(PlaybackUiEvent::ToggleMute));
                Slider::new(cx, playback_volume)
                    .range(0.0..100.0)
                    .on_change(|cx, val| cx.emit(PlaybackUiEvent::SetVolume(val)))
                    .width(Pixels(110.0));
                // Label::new(
                //     cx,
                //     playback_volume.map(|v| format!("{}%", v.round() as i32)),
                // )
                // .width(Pixels(44.0));
            })
            .height(Auto)
            .width(Auto)
            .alignment(Alignment::Center)
            .gap(Pixels(8.0));

            // PickList::new(
            //     cx,
            //     playback_device_options,
            //     selected_playback_device_index,
            //     true,
            // )
            // .width(Pixels(220.0))
            // .placeholder("Choose a playback device")
            // .on_select(|cx, index| cx.emit(PlaybackUiEvent::SelectPlaybackDevice(index)));
        })
        .alignment(Alignment::Right)
        .height(Auto);
    })
    .alignment(Alignment::Center)
    .class("panel")
    .class("playback-panel")
    .width(Stretch(1.0))
    .height(Auto)
    .padding(Pixels(8.0))
    .gap(Pixels(8.0));
}
