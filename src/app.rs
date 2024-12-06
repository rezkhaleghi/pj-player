use std::error::Error;
use std::process::Child;
use std::sync::{ Arc, Mutex };
use crate::search::{ search_youtube, search_archive };
// use crossterm::event::{ KeyEvent, KeyCode };

#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    YouTube,
    InternetArchive,
}

#[derive(PartialEq)]
pub enum Mode {
    Stream,
    Download,
}

#[derive(PartialEq)]
pub enum View {
    SearchInput,
    SearchResults,
    InitialSelection,
    SourceSelection,
    Streaming,
    Downloading,
}
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub identifier: String,
    pub title: String,
    pub source: Source,
}

pub struct AppUi {
    pub search_input: String,
    pub search_results: Vec<SearchResult>,
    pub selected_result_index: Option<usize>,
    pub selected_source_index: usize,
    pub source: Source,
    pub current_view: View,
    pub visualization_data: Arc<Mutex<Vec<u8>>>,
    pub ffplay_process: Option<Child>,
    pub mode: Option<Mode>,
    pub current_equalizer: usize,
    pub download_status: Arc<Mutex<Option<String>>>,
}

impl AppUi {
    pub fn new() -> Self {
        AppUi {
            search_input: String::new(),
            search_results: Vec::new(),
            selected_result_index: Some(0),
            selected_source_index: 0,
            source: Source::YouTube,
            current_view: View::SearchInput,
            visualization_data: Arc::new(Mutex::new(vec![0; 10])),
            ffplay_process: None,
            current_equalizer: 0,
            mode: None,
            download_status: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn search(&mut self) -> Result<(), Box<dyn Error>> {
        self.search_results = match self.source {
            Source::YouTube => search_youtube(&self.search_input).await?,
            Source::InternetArchive => search_archive(&self.search_input).await?,
        };
        self.current_view = View::SearchResults;
        self.selected_result_index = Some(0);
        Ok(())
    }

    pub fn stop_streaming(&mut self) {
        if let Some(mut process) = self.ffplay_process.take() {
            let _ = process.kill();
        }
    }

    // pub fn handle_key_press(&mut self, key: KeyEvent) {
    //     match key {
    //         KeyEvent { code: KeyCode::Char('e'), .. } => {
    //             self.current_equalizer = (self.current_equalizer + 1) % 5;
    //         }
    //         // Handle other keys...
    //         _ => {}
    //     }
    // }
}
