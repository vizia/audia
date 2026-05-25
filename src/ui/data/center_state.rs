use vizia::prelude::*;

use crate::ui::{events::CenterEvents, model_data::CenterPage};

#[derive(Clone)]
pub struct CenterState {
    pub current_page: Signal<CenterPage>,
    pub page_history: Signal<Vec<CenterPage>>,
    pub page_history_index: Signal<usize>,
    pub can_go_back: Signal<bool>,
    pub can_go_forward: Signal<bool>,
}

impl CenterState {
    pub fn new() -> Self {
        Self {
            current_page: Signal::new(CenterPage::Search),
            page_history: Signal::new(vec![CenterPage::Search]),
            page_history_index: Signal::new(0),
            can_go_back: Signal::new(false),
            can_go_forward: Signal::new(false),
        }
    }
}

impl CenterState {
    fn sync_navigation_flags(&self) {
        let index = self.page_history_index.get();
        let len = self.page_history.get().len();

        self.can_go_back.set(index > 0);
        self.can_go_forward.set(index + 1 < len);
    }

    fn navigate_to(&mut self, page: CenterPage) {
        let mut history = self.page_history.get();
        let mut index = self.page_history_index.get();

        if history.get(index).copied() == Some(page) {
            self.current_page.set(page);
            self.sync_navigation_flags();
            return;
        }

        history.truncate(index + 1);
        history.push(page);
        index = history.len().saturating_sub(1);

        self.page_history.set(history);
        self.page_history_index.set(index);
        self.current_page.set(page);
        self.sync_navigation_flags();
    }

    fn navigate_back(&mut self) {
        let index = self.page_history_index.get();
        if index == 0 {
            self.sync_navigation_flags();
            return;
        }

        let next_index = index - 1;
        let history = self.page_history.get();
        if let Some(page) = history.get(next_index).copied() {
            self.page_history_index.set(next_index);
            self.current_page.set(page);
        }

        self.sync_navigation_flags();
    }

    fn navigate_forward(&mut self) {
        let history = self.page_history.get();
        let index = self.page_history_index.get();
        if index + 1 >= history.len() {
            self.sync_navigation_flags();
            return;
        }

        let next_index = index + 1;
        if let Some(page) = history.get(next_index).copied() {
            self.page_history_index.set(next_index);
            self.current_page.set(page);
        }

        self.sync_navigation_flags();
    }
}

impl Model for CenterState {
    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|ui_event, _: &mut _| match ui_event {
            CenterEvents::NavigateTo(page) => {
                self.navigate_to(*page);
            }
            CenterEvents::NavigateBack => {
                self.navigate_back();
            }
            CenterEvents::NavigateForward => {
                self.navigate_forward();
            }
        });
    }
}
