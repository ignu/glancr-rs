use grep::{
    matcher::Matcher,
    regex::RegexMatcher,
    searcher::{sinks::UTF8, BinaryDetection, SearcherBuilder},
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use std::path::PathBuf;
use syntect::{easy::HighlightLines, highlighting::ThemeSet, parsing::SyntaxSet};

use crate::SearchMode;

pub fn get_file_preview(
    path: &PathBuf,
    query: &str,
    search_mode: SearchMode,
) -> (Text<'static>, Option<u16>) {
    // Read the file content
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return (Text::raw("Unable to read file"), None),
    };

    let lines: Vec<&str> = content.lines().collect();

    // Find the first matching line index
    let first_match_index = if !query.is_empty() && search_mode == SearchMode::Contents {
        if let Some(regex_matcher) = RegexMatcher::new(query).ok() {
            let mut searcher = SearcherBuilder::new()
                .binary_detection(BinaryDetection::quit(0))
                .build();

            let mut match_line = None;
            let sink = UTF8(|line_num, _line| {
                match_line = Some(line_num);
                Ok(false) // Stop after first match
            });

            searcher.search_path(&regex_matcher, path, sink).ok();
            match_line
        } else {
            None
        }
    } else {
        None
    };

    // Calculate scroll position
    let scroll_to = first_match_index.map(|line_num| line_num as u16);

    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    // Try multiple methods to detect the correct syntax
    let syntax = ps
        // First try by file extension
        .find_syntax_for_file(path)
        .ok()
        .flatten()
        // Then try by first line of content if extension doesn't work
        .or_else(|| {
            lines
                .first()
                .and_then(|first_line| ps.find_syntax_by_first_line(first_line))
        })
        // Finally fallback to plain text
        .unwrap_or_else(|| ps.find_syntax_plain_text());

    let mut text_lines = Vec::new();

    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    for (idx, line) in lines.iter().enumerate() {
        let mut line_spans = Vec::new();
        let line_number = idx + 1;
        line_spans.push(Span::styled(
            format!("{:4} ", line_number),
            Style::default().fg(Color::DarkGray),
        ));

        match h.highlight_line(line, &ps) {
            Ok(ranges) => {
                for (style, text) in ranges.iter() {
                    let fg_color =
                        Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);

                    if !query.is_empty() && search_mode == SearchMode::Contents {
                        if let Some(regex_matcher) = RegexMatcher::new(query).ok() {
                            if let Ok(Some(match_result)) = regex_matcher.find(text.as_bytes()) {
                                let match_start = match_result.start();
                                let mut last_idx = 0;

                                for idx in match_start..match_result.end() {
                                    if idx > last_idx {
                                        line_spans.push(Span::styled(
                                            text[last_idx..idx].to_string(),
                                            Style::default().fg(fg_color),
                                        ));
                                    }

                                    line_spans.push(Span::styled(
                                        text[idx..idx + 1].to_string(),
                                        Style::default()
                                            .fg(fg_color)
                                            .bg(Color::DarkGray)
                                            .add_modifier(Modifier::BOLD),
                                    ));

                                    last_idx = idx + 1;
                                }

                                if last_idx < text.len() {
                                    line_spans.push(Span::styled(
                                        text[last_idx..].to_string(),
                                        Style::default().fg(fg_color),
                                    ));
                                }
                                continue;
                            }
                        }
                    }

                    line_spans.push(Span::styled(
                        text.to_string(),
                        Style::default().fg(fg_color),
                    ));
                }
                text_lines.push(Line::from(line_spans));
            }
            Err(_) => text_lines.push(Line::from("Error reading file".to_string())),
        }
    }

    (Text::from(text_lines), scroll_to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(content: &str) -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.rs");
        let mut file = File::create(&file_path).unwrap();
        write!(file, "{}", content).unwrap();
        (dir, file_path)
    }

    #[test]
    fn test_file_preview_basic() {
        let content = "fn main() {\n    println!(\"Hello\");\n}";
        let (_dir, path) = create_test_file(content);

        let (preview, scroll) = get_file_preview(&path, "", SearchMode::Contents);
        assert!(preview.lines.len() > 0);
        assert_eq!(scroll, None);
    }

    #[test]
    fn test_file_preview_with_query() {
        let content = "line one\nline two\nline three with match\nline four";
        let (_dir, path) = create_test_file(content);

        let (preview, scroll) = get_file_preview(&path, "match", SearchMode::Contents);
        assert!(preview.lines.len() > 0);
        println!("{:?}", scroll);

        assert_eq!(scroll, Some(3));
    }

    #[test]
    fn test_file_preview_nonexistent_file() {
        let path = PathBuf::from("nonexistent_file.txt");
        let (preview, scroll) = get_file_preview(&path, "", SearchMode::Contents);

        assert_eq!(preview.lines.len(), 1);
        assert_eq!(preview.lines[0].spans[0].content, "Unable to read file");
        assert_eq!(scroll, None);
    }

    #[test]
    fn test_file_preview_line_numbers() {
        let content = "line1\nline2\nline3";
        let (_dir, path) = create_test_file(content);

        let (preview, _) = get_file_preview(&path, "", SearchMode::Contents);

        // Check if first line starts with line number
        let first_line_number = preview.lines[0].spans[0].content.trim();
        assert_eq!(first_line_number, "1");
    }

    #[test]
    fn test_file_preview_syntax_highlighting() {
        let content = "fn main() {\n    let x = 42;\n}";
        let (_dir, path) = create_test_file(content);

        let (preview, _) = get_file_preview(&path, "", SearchMode::Contents);

        println!("{:?}", preview.lines[0].spans[1].content);
        assert!(preview.lines[0].spans.len() > 1);
    }
}
