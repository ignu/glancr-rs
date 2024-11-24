# Glancr 🔍

A terminal-based file preview and search tool inspired by Telescope.nvim, built in Rust. Glancr provides fast file searching with fuzzy finding capabilities and syntax-highlighted previews right in your terminal.

## Features

- 🔎 Fuzzy file search
- 📄 Content search with regex support
- 📄 Syntax-highlighted file previews
- ⌨️ Keyboard navigation
- 🎨 Terminal UI powered by [ratatui](https://github.com/ratatui-org/ratatui)
- 🚀 Fast and lightweight
- 📁 Respects .gitignore

## Prerequisites

- Rust toolchain
- Git (for installation from source)

## Installation

### From source

```bash
bash
git clone https://github.com/yourusername/glancr.git
cd glancr
cargo install --path .
```

## Keyboard Controls

- Type to search files
- `↑` / `↓` to navigate through results
- `Enter` to open selected file in editor defined in `~/.glancr.yml`
- `Esc` to exit

## Configuration

Glancr can be configured through `~/.glancr.yml:

```yaml
open_command: 'code'
```
