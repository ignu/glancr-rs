# Glancr 🔍

A terminal-based file preview and search tool inspired by Telescope.nvim, built in Rust. Glancr provides fast file searching with fuzzy finding capabilities and syntax-highlighted previews right in your terminal.

![CleanShot 2024-11-24 at 09 12 55@2x](https://github.com/user-attachments/assets/5b6257bd-bfb9-4798-9b54-df11132c3191)

## Features

- 🔎 Fuzzy file search
- 📄 Content search with regex support
- 📄 Syntax-highlighted file previews
- ⌨️ Keyboard navigation
- 🎨 Terminal UI powered by [ratatui](https://github.com/ratatui-org/ratatui)
- 🚀 Fast and lightweight
- 📁 Respects .gitignore

## Installation

### From source

```bash
bash
git clone https://github.com/ignu/glancr-rs.git
cd glancr
cargo install --path .
```

## Keyboard Controls

- Type to search files
- `↑` / `↓` to navigate through results
- `F1` or `Ctrl+h` for help
- `Enter` to open selected file in editor defined in `~/.glancr.yml`
- `Ctrl+f` for grepping all files
- `Ctrl+d` to toggle searching dirty files
- `Ctrl+b` to toggle files changed from default branch
- `Ctrl+n` for searching file names
- `PageUp/PageDwn` scroll preview
- `Esc` to exit

## Configuration

Glancr can be configured through `~/.glancr.yml`:

```yaml
open_command: 'code'
```
