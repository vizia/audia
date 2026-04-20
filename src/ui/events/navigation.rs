use crate::ui::model_data::CenterPage;

#[derive(Clone, Debug)]
pub enum CenterUiEvent {
    NavigateTo(CenterPage),
    NavigateBack,
    NavigateForward,
}
