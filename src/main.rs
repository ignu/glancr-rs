use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use grep::{
    regex::RegexMatcher,
    searcher::{sinks::UTF8, BinaryDetection, SearcherBuilder},
};
use ignore::WalkBuilder;
use ratatui::{
    prelude::*,
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
};
use std::{fs::File, io::stdout, io::Read, path::PathBuf};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input as TextInput;

mod preview;
use preview::get_file_preview;

#[derive(Debug, Clone, Copy, PartialEq)]
enum SearchMode {
    Filename,
    Contents,
}

struct App {
    files: Vec<PathBuf>,
    filtered_files: Vec<PathBuf>,
    selected_index: usize,
    input: TextInput,
    search_mode: SearchMode,
}

// Helper function to check if a file is likely binary
fn is_binary_file(path: &std::path::Path) -> bool {
    if let Ok(mut file) = File::open(path) {
        let mut buffer = [0; 1024];
        if let Ok(n) = file.read(&mut buffer) {
            // Check first 1024 bytes for null bytes or other binary indicators
            return buffer[..n].iter().any(|&byte| byte == 0);
        }
    }
    false
}

// Add this helper function to check for directories/files we want to ignore
fn should_ignore_path(path: &std::path::Path) -> bool {
    let path_str = path.to_string_lossy().to_lowercase();

    // Common directories to ignore
    let ignored_dirs = [
        "/.git/",
        "/node_modules/",
        "/target/",
        "/dist/",
        "/build/",
        "/.idea/",
        "/.vscode/",
        "/vendor/",
        "/.next/",
        "/coverage/",
    ];

    // Common file patterns to ignore
    let ignored_patterns = [
        ".lock", ".log", ".map", ".min.js", ".min.css", ".bundle.", ".cache",
    ];

    // Check if path contains any of the ignored directory patterns
    if ignored_dirs.iter().any(|dir| path_str.contains(dir)) {
        return true;
    }

    // Check if the file name matches any ignored patterns
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        let file_name_lower = file_name.to_lowercase();
        if ignored_patterns
            .iter()
            .any(|pattern| file_name_lower.contains(pattern))
        {
            return true;
        }
    }

    false
}

impl App {
    fn new() -> Self {
        let mut files = Vec::new();

        for entry in WalkBuilder::new(".")
            .hidden(false)
            .git_ignore(true)
            .build()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let path = e.path();

                // Skip directories and special paths
                if !e.file_type().map_or(false, |ft| ft.is_file()) {
                    return false;
                }

                // Skip common ignored paths
                if should_ignore_path(path) {
                    return false;
                }

                // Skip binary files
                !is_binary_file(path)
            })
        {
            files.push(entry.path().to_path_buf());
        }

        App {
            files: files.clone(),
            filtered_files: files,
            selected_index: 0,
            input: TextInput::default(),
            search_mode: SearchMode::Filename,
        }
    }

    fn filter_files(&mut self) {
        let query = self.input.value();
        if query.is_empty() {
            self.filtered_files = self.files.clone();
            return;
        }
        match self.search_mode {
            SearchMode::Filename => self.filter_by_filename(query.to_string()),
            SearchMode::Contents => self.filter_by_contents(query.to_string()),
        }

        // Keep selected index in bounds
        self.selected_index = self
            .selected_index
            .min(self.filtered_files.len().saturating_sub(1));
    }

    fn filter_by_filename(&mut self, query: String) {
        let matcher = SkimMatcherV2::default();
        self.filtered_files = self
            .files
            .iter()
            .filter(|path| {
                let path_str = path.to_string_lossy();
                matcher.fuzzy_match(&path_str, &query).is_some()
            })
            .cloned()
            .collect();
    }

    fn filter_by_contents(&mut self, query: String) {
        if let Some(regex_matcher) = RegexMatcher::new(&query).ok() {
            let mut searcher = SearcherBuilder::new()
                .binary_detection(BinaryDetection::quit(0))
                .build();

            self.filtered_files = self
                .files
                .iter()
                .filter(|path| {
                    let mut found = false;
                    let sink = UTF8(|_line_num, _line| {
                        found = true;
                        Ok(false) // Stop searching after first match
                    });

                    searcher
                        .search_path(&regex_matcher, path, sink)
                        .unwrap_or_else(|_| {
                            found = false;
                        });
                    found
                })
                .cloned()
                .collect();
        } else {
            self.filtered_files.clear();
        }
    }

    fn get_file_preview(&self) -> (Text<'static>, Option<u16>) {
        if self.filtered_files.is_empty() {
            return (Text::raw(""), None);
        }

        let path = &self.filtered_files[self.selected_index];
        get_file_preview(path, self.input.value(), self.search_mode)
    }
}

fn run_app() -> Result<()> {
    stdout().execute(EnterAlternateScreen)?;
    enable_raw_mode()?;

    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    let mut app = App::new();

    loop {
        terminal.draw(|frame| {
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(frame.size());

            let right_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(3),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ])
                .split(layout[1]);

            let file_list = List::new(
                app.filtered_files
                    .iter()
                    .enumerate()
                    .map(|(i, path)| {
                        let style = if i == app.selected_index {
                            Style::default().bg(Color::DarkGray)
                        } else {
                            Style::default()
                        };
                        ListItem::new(path.to_string_lossy().into_owned()).style(style)
                    })
                    .collect::<Vec<_>>(),
            )
            .block(Block::default().borders(Borders::ALL).title("Files"));

            let (preview_text, scroll_to) = app.get_file_preview();
            let preview = Paragraph::new(preview_text)
                .block(Block::default().borders(Borders::ALL).title("Preview"))
                .wrap(Wrap { trim: true });

            // Only scroll if we have a position to scroll to
            let preview = if let Some(scroll_pos) = scroll_to {
                preview.scroll((scroll_pos, 0))
            } else {
                preview
            };

            // Calculate cursor position
            let cursor_position = app.input.cursor();
            let mut input_value = app.input.value().to_string();
            input_value.insert(cursor_position, '|'); // Insert cursor character

            // Determine the label based on the current search mode
            let search_label = match app.search_mode {
                SearchMode::Filename => "Filename Search",
                SearchMode::Contents => "Content Search",
            };

            let input = Paragraph::new(input_value)
                .block(Block::default().borders(Borders::ALL).title(search_label));

            let status = Paragraph::new(match app.search_mode {
                SearchMode::Filename => {
                    "Mode: Filename Search (Ctrl+N: filename, Ctrl+F: contents)"
                }
                SearchMode::Contents => "Mode: Content Search (Ctrl+N: filename, Ctrl+F: contents)",
            })
            .style(Style::default().fg(Color::Gray));

            frame.render_widget(file_list, layout[0]);
            frame.render_widget(preview, right_layout[0]);
            frame.render_widget(input, right_layout[1]);
            frame.render_widget(status, right_layout[2]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => break,
                    KeyCode::Esc => break,
                    KeyCode::Char('n') if key.modifiers == KeyModifiers::CONTROL => {
                        app.search_mode = SearchMode::Filename;
                        app.filter_files();
                    }
                    KeyCode::Char('f') if key.modifiers == KeyModifiers::CONTROL => {
                        app.search_mode = SearchMode::Contents;
                        app.filter_files();
                    }
                    KeyCode::Char(_) => {
                        app.input.handle_event(&Event::Key(key));
                        app.filter_files();
                    }
                    KeyCode::Backspace => {
                        app.input.handle_event(&Event::Key(key));
                        app.filter_files();
                    }
                    KeyCode::Up => {
                        app.selected_index = app.selected_index.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        if !app.filtered_files.is_empty() {
                            app.selected_index =
                                (app.selected_index + 1).min(app.filtered_files.len() - 1);
                        }
                    }
                    KeyCode::Enter => {
                        if !app.filtered_files.is_empty() {
                            // Here you could implement the action to open the selected file
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

fn main() -> Result<()> {
    run_app().context("Error running application")
}
