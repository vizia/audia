use vizia::prelude::*;

use crate::ui::{events::RightPanelEvents, model_data::RightPanelPage};

#[derive(Clone, Copy)]
pub struct RightPanelState {
    pub current_page: Signal<RightPanelPage>,
}

impl RightPanelState {
    pub fn new() -> Self {
        Self {
            current_page: Signal::new(RightPanelPage::Queue),
        }
    }
}

impl Model for RightPanelState {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|ui_event, _: &mut _| match ui_event {
            RightPanelEvents::NavigateTo(page) => {
                self.current_page.set(*page);
            }
        });
    }
}
