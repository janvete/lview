use std::collections::VecDeque;

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::config::Config;
use crate::discovery::{discover_logs, DiscoveryResult, LogSource};
use crate::log_stream::start_stream;
use crate::search::SearchState;
use crate::ssh::SshSession;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Picker,
    Viewer,
    Search,
    PickerSearch,
}

#[derive(Debug)]
pub struct LogViewer {
    pub source: LogSource,
    pub buffer: VecDeque<String>,
    pub scroll: usize,
    pub live: bool,
    pub receiver: UnboundedReceiver<String>,
    pub max_lines: usize,
    pub search: SearchState,
    pub search_cursor: Option<usize>,
    pub error: Option<String>,
}

impl LogViewer {
    pub fn new(source: LogSource, receiver: UnboundedReceiver<String>, max_lines: usize) -> Self {
        Self {
            source,
            buffer: VecDeque::with_capacity(max_lines),
            scroll: 0,
            live: true,
            receiver,
            max_lines,
            search: SearchState::default(),
            search_cursor: None,
            error: None,
        }
    }

    /// Returns visible lines together with their original buffer index.
    pub fn visible_lines(&self) -> Vec<(usize, &String)> {
        self.buffer
            .iter()
            .enumerate()
            .filter(|(_, l)| self.search.matches(l))
            .collect()
    }

    pub fn push_line(&mut self, line: String) {
        if let Some(err) = line.strip_prefix("STDERR:") {
            self.error = Some(err.to_string());
            return;
        }
        if self.buffer.len() >= self.max_lines {
            self.buffer.pop_front();
            if self.scroll > 0 {
                self.scroll = self.scroll.saturating_sub(1);
            }
        }
        self.buffer.push_back(line);
        if self.live {
            self.scroll_to_bottom();
        }
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = self.buffer.len().saturating_sub(1);
        self.scroll = (self.scroll + n).min(max);
        self.live = false;
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
        if self.scroll < self.buffer.len().saturating_sub(1) {
            self.live = false;
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll = self.buffer.len().saturating_sub(1);
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
        self.live = false;
    }

    pub fn toggle_live(&mut self) {
        self.live = !self.live;
        if self.live {
            self.scroll_to_bottom();
        }
    }

    pub fn set_search(&mut self, query: String) {
        self.search.query = query;
        self.search_cursor = None;
        if !self.search.query.is_empty()
            && let Some(idx) = self.search.next_match(self.buffer.make_contiguous(), self.scroll)
        {
            self.scroll = idx;
            self.search_cursor = Some(idx);
            self.live = false;
        }
    }

    pub fn next_search_result(&mut self) {
        let lines: Vec<String> = self.buffer.iter().cloned().collect();
        if let Some(idx) = self.search.next_match(&lines, self.scroll) {
            self.scroll = idx;
            self.search_cursor = Some(idx);
            self.live = false;
        }
    }

    pub fn prev_search_result(&mut self) {
        let lines: Vec<String> = self.buffer.iter().cloned().collect();
        if let Some(idx) = self.search.prev_match(&lines, self.scroll) {
            self.scroll = idx;
            self.search_cursor = Some(idx);
            self.live = false;
        }
    }
}

pub struct App {
    pub session: SshSession,
    pub config: Config,
    pub runtime: tokio::runtime::Handle,
    pub logs: Vec<LogSource>,
    pub filtered_indices: Vec<usize>,
    pub selected_index: usize,
    pub screen: Screen,
    pub viewer: Option<LogViewer>,
    pub picker_filter: String,
    pub message: Option<String>,
    pub loading: bool,
    pub discovery_errors: Vec<String>,
    matcher: SkimMatcherV2,
}

impl App {
    pub fn new(session: SshSession, config: Config, runtime: tokio::runtime::Handle) -> Self {
        Self {
            session,
            config,
            runtime,
            logs: Vec::new(),
            filtered_indices: Vec::new(),
            selected_index: 0,
            screen: Screen::Picker,
            viewer: None,
            picker_filter: String::new(),
            message: Some("Press 'r' to load logs".into()),
            loading: false,
            discovery_errors: Vec::new(),
            matcher: SkimMatcherV2::default(),
        }
    }

    pub async fn discover(&mut self) {
        self.loading = true;
        self.message = Some("Loading logs...".into());
        let result = discover_logs(
            &self.session,
            &self.config.extra_paths,
            self.config.discovery_timeout,
        )
        .await;
        self.apply_discovery(result);
        self.loading = false;
    }

    fn apply_discovery(&mut self, result: DiscoveryResult) {
        self.logs = result.sources;
        self.discovery_errors = result.errors;
        self.apply_picker_filter();
        self.selected_index = 0;
        if self.logs.is_empty() {
            self.message = Some("No logs found.".into());
        } else {
            self.message = Some(format!("Found {} logs", self.logs.len()));
        }
    }

    pub fn apply_picker_filter(&mut self) {
        if self.picker_filter.is_empty() {
            self.filtered_indices = (0..self.logs.len()).collect();
        } else {
            let mut scored: Vec<(i64, usize)> = self
                .logs
                .iter()
                .enumerate()
                .filter_map(|(i, log)| {
                    self.matcher
                        .fuzzy_match(&log.search_text(), &self.picker_filter)
                        .map(|score| (score, i))
                })
                .collect();
            scored.sort_by_key(|b| std::cmp::Reverse(b.0));
            self.filtered_indices = scored.into_iter().map(|(_, i)| i).collect();
        }
        self.selected_index = self
            .selected_index
            .min(self.filtered_indices.len().saturating_sub(1));
    }

    pub fn selected_log(&self) -> Option<&LogSource> {
        self.filtered_indices
            .get(self.selected_index)
            .and_then(|&i| self.logs.get(i))
    }

    pub fn next_log(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.filtered_indices.len();
        }
    }

    pub fn previous_log(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected_index = self
                .selected_index
                .checked_sub(1)
                .unwrap_or(self.filtered_indices.len() - 1);
        }
    }

    pub fn open_selected_log(&mut self) -> std::io::Result<()> {
        let Some(source) = self.selected_log().cloned() else {
            return Ok(());
        };
        let session = self.session.clone();
        let stream_source = source.clone();
        let rx = self.runtime.block_on(async move {
            start_stream(&session, &stream_source, 1000)
        })?;
        self.viewer = Some(LogViewer::new(source, rx, self.config.max_log_lines));
        self.screen = Screen::Viewer;
        self.message = None;
        Ok(())
    }

    pub fn close_viewer(&mut self) {
        self.viewer = None;
        self.screen = Screen::Picker;
        self.message = None;
    }

    pub fn update_stream(&mut self) {
        if let Some(viewer) = &mut self.viewer {
            while let Ok(line) = viewer.receiver.try_recv() {
                viewer.push_line(line);
            }
            if let Some(err) = viewer.error.take() {
                self.message = Some(format!("SSH error: {}", err));
            }
        }
    }

    pub fn enter_search(&mut self) {
        self.screen = match self.screen {
            Screen::Picker => Screen::PickerSearch,
            Screen::Viewer => Screen::Search,
            _ => self.screen,
        };
    }

    pub fn exit_search(&mut self) {
        self.screen = match self.screen {
            Screen::PickerSearch => Screen::Picker,
            Screen::Search => Screen::Viewer,
            _ => self.screen,
        };
        if let Some(v) = &mut self.viewer {
            v.search = SearchState::default();
            v.search_cursor = None;
        }
    }

    pub fn confirm_search(&mut self) {
        self.screen = match self.screen {
            Screen::PickerSearch => Screen::Picker,
            Screen::Search => Screen::Viewer,
            _ => self.screen,
        };
    }

    pub fn current_filter_query(&self) -> &str {
        match self.screen {
            Screen::PickerSearch => &self.picker_filter,
            Screen::Search => self.viewer.as_ref().map(|v| v.search.query.as_str()).unwrap_or(""),
            _ => "",
        }
    }

    pub fn current_filter_query_mut(&mut self) -> Option<&mut String> {
        match self.screen {
            Screen::PickerSearch => Some(&mut self.picker_filter),
            Screen::Search => self.viewer.as_mut().map(|v| &mut v.search.query),
            _ => None,
        }
    }

    pub fn apply_current_filter(&mut self) {
        if self.screen == Screen::PickerSearch {
            self.apply_picker_filter();
        } else if let Some(v) = &mut self.viewer {
            let query = v.search.query.clone();
            v.set_search(query);
        }
    }

    pub fn save_viewer_buffer(&self) -> Option<String> {
        let viewer = self.viewer.as_ref()?;
        let name = format!(
            "lview-{}-{}",
            viewer.source.to_string().replace(['/', ' ', ':'], "_"),
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        );
        let path = std::env::temp_dir().join(name);
        let content: String = viewer
            .buffer
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, content).ok()?;
        Some(path.to_string_lossy().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_viewer_push_and_scroll() {
        let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut v = LogViewer::new(
            LogSource::File { path: "/tmp/test.log".into() },
            rx,
            100,
        );
        for i in 0..5 {
            v.push_line(format!("line {}", i));
        }
        assert_eq!(v.buffer.len(), 5);
        assert_eq!(v.scroll, 4);
        v.scroll_up(2);
        assert_eq!(v.scroll, 2);
        v.toggle_live();
        assert!(v.live);
        assert_eq!(v.scroll, 4);
    }

    #[test]
    fn test_app_filter() {
        let session = SshSession::new("ssh".into(), crate::ssh::SshTarget { args: vec!["host".into()] });
        let rt = tokio::runtime::Runtime::new().unwrap();
        let mut app = App::new(session, Config::default(), rt.handle().clone());
        app.logs = vec![
            LogSource::File { path: "/var/log/syslog".into() },
            LogSource::File { path: "/opt/app/app.log".into() },
        ];
        app.picker_filter = "syslog".into();
        app.apply_picker_filter();
        assert_eq!(app.filtered_indices, vec![0]);
    }
}
