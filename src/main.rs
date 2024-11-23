use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::{
    fs,
    io::stdout,
    path::PathBuf,
};
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::SyntaxSet,
    util::{as_24_bit_terminal_escaped, LinesWithEndings},
};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;
use walkdir::WalkDir;

struct App {
    files: Vec<PathBuf>,
    filtered_files: Vec<PathBuf>,
    selected_index: usize,
    input: Input,
    ps: SyntaxSet,
    ts: ThemeSet,
}

impl App {
    fn new() -> Self {
        let mut files = Vec::new();
        for entry in WalkDir::new(".")
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            files.push(entry.path().to_path_buf());
        }

        App {
            files: files.clone(),
            filtered_files: files,
            selected_index: 0,
            input: Input::default(),
            ps: SyntaxSet::load_defaults_newlines(),
            ts: ThemeSet::load_defaults(),
        }
    }

    fn filter_files(&mut self) {
        let matcher = SkimMatcherV2::default();
        let query = self.input.value();
        
        self.filtered_files = self
            .files
            .iter()
            .filter(|path| {
                matcher
                    .fuzzy_match(
                        path.to_string_lossy().as_ref(),
                        &query,
                    )
                    .is_some()
            })
            .cloned()
            .collect();

        self.selected_index = self.selected_index.min(self.filtered_files.len().saturating_sub(1));
    }

    fn get_file_preview(&self) -> String {
        if self.filtered_files.is_empty() {
            return String::new();
        }

        let path = &self.filtered_files[self.selected_index];
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => return String::from("Unable to read file"),
        };

        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        let syntax = self
            .ps
            .find_syntax_by_extension(extension)
            .unwrap_or_else(|| self.ps.find_syntax_plain_text());

        let mut h = HighlightLines::new(syntax, &self.ts.themes["base16-ocean.dark"]);
        let mut colored_content = String::new();

        for line in LinesWithEndings::from(&content) {
            let ranges = h.highlight_line(line, &self.ps).unwrap();
            let escaped = as_24_bit_terminal_escaped(&ranges[..], false);
            colored_content.push_str(&escaped);
        }

        colored_content
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
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Percentage(70),
                ])
                .split(frame.size());

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

            let preview = Paragraph::new(app.get_file_preview())
                .block(Block::default().borders(Borders::ALL).title("Preview"));

            let input_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(3),
                ])
                .split(layout[1]);

            let input = Paragraph::new(app.input.value())
                .block(Block::default().borders(Borders::ALL).title("Search"));

            frame.render_widget(file_list, layout[0]);
            frame.render_widget(preview, input_layout[0]);
            frame.render_widget(input, input_layout[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char(c) => {
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
                            app.selected_index = (app.selected_index + 1).min(app.filtered_files.len() - 1);
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