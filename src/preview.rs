use crate::SearchMode;
use bat::assets::HighlightingAssets;
use grep::{
    matcher::Matcher,
    regex::RegexMatcher,
    searcher::{sinks::UTF8, BinaryDetection, SearcherBuilder},
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use syntect::{easy::HighlightLines, highlighting::ThemeSet};

const MAX_FILE_SIZE: u64 = 1024 * 512; // 512KB threshold
const MAX_LINES_TO_FORMAT: usize = 1000; // Reasonable number of lines to syntax highlight

pub fn get_file_preview(
    path: &PathBuf,
    query: &str,
    search_mode: SearchMode,
) -> (Text<'static>, Option<u16>) {
    // Check file size first
    let metadata = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(_) => return (Text::raw("Unable to read file metadata"), None),
    };

    if metadata.len() > MAX_FILE_SIZE {
        return get_large_file_preview(path, query, search_mode);
    }

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
    //let ps = SyntaxSet::load_defaults_newlines();
    let ps = HighlightingAssets::from_binary();
    let ps = ps.get_syntax_set().unwrap();
    let ts = ThemeSet::load_defaults();

    // Try multiple methods to detect the correct syntax
    let syntax = ps
        // First try by file extension
        .find_syntax_for_file(path)
        .ok()
        .flatten()
        // Then try by extension directly for common web files
        .or_else(|| {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            match ext.as_str() {
                "ts" | "tsx" => ps.find_syntax_by_extension("typescript"),
                "js" | "jsx" => ps.find_syntax_by_extension("javascript"),
                _ => None,
            }
        })
        // Then try by first line of content
        .or_else(|| {
            lines
                .first()
                .and_then(|first_line| ps.find_syntax_by_first_line(first_line))
        })
        // Finally fallback to plain text
        .unwrap_or_else(|| ps.find_syntax_by_extension("txt").unwrap());

    let mut text_lines = Vec::new();

    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    for (idx, line) in lines.iter().take(MAX_LINES_TO_FORMAT).enumerate() {
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

    // If we hit the limit, add a notice
    if lines.len() > MAX_LINES_TO_FORMAT {
        text_lines.push(Line::from(vec![Span::styled(
            "⚠️  File truncated - showing first 1000 lines only",
            Style::default().fg(Color::Yellow),
        )]));
    }

    (Text::from(text_lines), scroll_to)
}

// New function to handle large files
fn get_large_file_preview(
    path: &PathBuf,
    query: &str,
    search_mode: SearchMode,
) -> (Text<'static>, Option<u16>) {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(_) => return (Text::raw("Unable to read file"), None),
    };

    let reader = BufReader::new(file);
    let mut text_lines = Vec::new();
    let mut first_match_line = None;

    // Add a warning header
    text_lines.push(Line::from(vec![Span::styled(
        "⚠️  Large file detected - showing plain text without syntax highlighting",
        Style::default().fg(Color::Yellow),
    )]));

    for (idx, line_result) in reader.lines().take(MAX_LINES_TO_FORMAT).enumerate() {
        let line = match line_result {
            Ok(line) => line,
            Err(_) => continue,
        };

        let mut line_spans = Vec::new();
        let line_number = idx + 1;

        // Add line number
        line_spans.push(Span::styled(
            format!("{:4} ", line_number),
            Style::default().fg(Color::DarkGray),
        ));

        // Check for matches if we're searching
        if !query.is_empty() && search_mode == SearchMode::Contents {
            if let Some(regex_matcher) = RegexMatcher::new(query).ok() {
                if let Ok(is_match) = regex_matcher.is_match(line.as_bytes()) {
                    if is_match {
                        first_match_line = first_match_line.or(Some(line_number as u16));
                        // Highlight the matching line
                        line_spans.push(Span::styled(
                            line,
                            Style::default()
                                .bg(Color::DarkGray)
                                .add_modifier(Modifier::BOLD),
                        ));
                        text_lines.push(Line::from(line_spans));
                        continue;
                    }
                }
            }
        }

        // Add regular line
        line_spans.push(Span::raw(line));
        text_lines.push(Line::from(line_spans));
    }

    (Text::from(text_lines), first_match_line)
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
