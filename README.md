<p align="center">
  <img src="assets/icon.svg" width="128" height="128" alt="Write icon">
</p>

<h1 align="center">Write</h1>

<p align="center">
  A distraction-free writing app that silently fixes your spelling as you type.
</p>

<p align="center">
  <a href="https://github.com/cheetohsum/write/releases/latest"><img src="https://img.shields.io/github/v/release/cheetohsum/write?style=flat-square&color=B76040" alt="Latest Release"></a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS-3E2F24?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/badge/rust-2021-C4A661?style=flat-square" alt="Rust 2021">
</p>

---

Write is a standalone writing application built in Rust. It opens its own window — double-click the exe, start writing. An LLM runs in the background to fix typos and spelling mistakes using surrounding context, so you never break flow. Documents save as `.md` files.

The interface uses a warm **Taliesin** color scheme inspired by Frank Lloyd Wright's prairie aesthetic: parchment backgrounds, walnut text, terracotta and gold accents.

## Features

- **Context-aware spell correction** — uses surrounding words to pick the right fix (e.g. "Hen I got home" → "When", not "Hen")
- **Respects your voice** — won't rewrite prose, change style, or take creative liberty
- **Standalone window** — opens as its own app with themed title bar and icon (not inside cmd.exe)
- **Settings screen** — configure API keys, choose preferred provider, browse OpenRouter models
- **Scramble animation** — corrected characters dissolve through random glyphs before resolving
- **Graph-node links** — `[[wiki-links]]` for building interconnected documents
- **Scroll indicator** — gold position marker appears in the left margin on long documents
- **Three LLM providers** — Claude (Anthropic), OpenAI, and OpenRouter
- **Smart triggers** — fires on sentence boundaries (300ms), word gaps (600ms), or idle (1.5s)
- **Zero config** — works as a plain editor with no API key; set one to enable spell correction

## Installation

Download from [Releases](https://github.com/cheetohsum/write/releases/latest):

| Platform | Download |
|----------|----------|
| Windows x86_64 | `Write-windows-x86_64.zip` |
| macOS Apple Silicon | `Write-macos-aarch64.dmg` |
| macOS Intel | `Write-macos-x86_64.dmg` |

Or build from source:

```
git clone https://github.com/cheetohsum/write.git
cd write
cargo build --release
```

## Getting Started

1. Launch Write
2. Go to **Settings** (Tab to ⚙ settings, press Enter)
3. Enter an API key for any provider — click the **✦ Keys** button to open the signup page
4. Press Enter on a provider to set it as preferred (◆ indicator)
5. Press Esc to save and return to the title screen
6. Enter a document title and start writing

API keys are saved to your system config directory and persist across sessions.

## Configuration

Settings are managed in-app (Tab → ⚙ settings on the title screen). Keys are stored in:

- **Windows:** `%APPDATA%\write\.env`
- **macOS:** `~/Library/Application Support/write/.env`

You can also set environment variables directly:

| Variable | Purpose |
|----------|---------|
| `ANTHROPIC_API_KEY` | Claude API |
| `OPENAI_API_KEY` | OpenAI API |
| `OPENROUTER_API_KEY` | OpenRouter API |
| `LLM_PROVIDER` | Force a provider (`claude`, `openai`, `openrouter`) |
| `LLM_MODEL` | Override the default model (useful for OpenRouter) |

## Keybindings

### Title Screen

| Key | Action |
|-----|--------|
| `Tab` | Cycle fields (directory → title → settings) |
| `Enter` | Open document / enter settings |
| `Esc` | Quit |

### Editor

| Key | Action |
|-----|--------|
| `Ctrl+S` | Save document |
| `Esc` | Back to title screen (auto-saves) |
| `Ctrl+Q` | Quit application |
| `Ctrl+G` | Wrap word under cursor in `[[wiki-link]]` |
| `Ctrl+O` | Navigate into `[[link]]` under cursor |
| `Ctrl+L` | Toggle LLM on/off |
| `Ctrl+A` | Select all text |

### Settings

| Key | Action |
|-----|--------|
| `Tab` | Cycle fields (3 providers → model) |
| `Enter` | Set provider as preferred (◆) |
| `↑` `↓` / scroll | Browse OpenRouter models |
| `Esc` | Save and return to title screen |

## Graph-Node Links

Write supports `[[wiki-links]]` for building interconnected documents.

- **Create:** Place cursor on a word → `Ctrl+G` → wraps in `[[brackets]]`
- **Navigate:** Cursor on a `[[link]]` → `Ctrl+O` → zooms into the linked page
- **Back:** `Esc` saves the linked page and returns to the parent

Linked pages are stored alongside your document:

```
~/Documents/
├── my-essay.md
└── my-essay/
    ├── Character Name.md
    └── Location.md
```

Compatible with Obsidian, Logseq, and other wiki-link tools.

## Architecture

```
src/
├── main.rs          # Terminal setup, panic hook
├── platform.rs      # Windows standalone window (conhost, DWM theming, icon)
├── app.rs           # State machine, event loop, animations
├── ui.rs            # Ratatui rendering, scroll indicator, dither overlays
├── editor.rs        # TextArea wrapper, word wrap, content replacement
├── theme.rs         # Taliesin color palette
├── config.rs        # Settings persistence, folder picker, provider detection
├── keybindings.rs   # Key → Action dispatch
└── llm/
    ├── mod.rs       # Background task, mpsc channels
    ├── claude.rs    # Anthropic Messages API
    ├── openai.rs    # OpenAI Chat Completions API
    ├── openrouter.rs
    └── prompt.rs    # Proofreading system prompt
```

## Color Palette

| Element | Color | Hex |
|---------|-------|-----|
| Background | Parchment | `#F5F0E8` |
| Text | Walnut | `#3E2F24` |
| Accent | Terracotta | `#B76040` |
| Strong accent | Maroon | `#782626` |
| Decorative | Gold | `#C4A661` |
| Status bar | Umber | `#5C4A3C` |
| Success | Sage | `#7F9474` |

## License

MIT
