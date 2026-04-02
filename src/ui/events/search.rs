use crate::messages::SearchResultsData;

#[derive(Clone, Debug)]
pub enum SearchUiEvent {
    SelectResult(usize),
    SetInput(String),
    SubmitQuery(String),
}

#[derive(Clone, Debug)]
pub enum SearchAppEvent {
    Results(SearchResultsData),
}
