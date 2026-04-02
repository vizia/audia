use crate::ui::events::SystemAppEvent;
use crate::ui::model_data::UiModel;
use vizia::prelude::*;

impl Model for UiModel {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        self.oauth_state.event(cx, event);
        self.preferences_data.event(cx, event);
        self.playback_state.event(cx, event);
        self.search_state.event(cx, event);
        self.playlists_state.event(cx, event);
        event.map(|system_event: &SystemAppEvent, _| match system_event {
            SystemAppEvent::Ready => {
                if !self.oauth_state.auth_valid.get() {
                    self.status
                        .set("Not logged in. Click Login with Spotify to continue.".to_string());
                }
            }
            SystemAppEvent::StatusMessage(message) => {
                self.status.set(message.clone());
            }
            SystemAppEvent::Error(error) => {
                self.status.set(format!("Error: {error}"));
            }
        });
    }
}
