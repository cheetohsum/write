<p align="center">
  <img src="assets/icon.svg" width="128" height="128" alt="Write icon">
</p>

<h1 align="center">Write</h1>

<p align="center">
  A distraction-free terminal editor that silently polishes your prose as you type.
</p>

<p align="center">
  <a href="https://github.com/cheetohsum/write/releases/latest"><img src="https://img.shields.io/github/v/release/cheetohsum/write?style=flat-square&color=B76040" alt="Latest Release"></a>
  <img src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS-3E2F24?style=flat-square" alt="Platform">
  <img src="https://img.shields.io/badge/rust-2021-C4A661?style=flat-square" alt="Rust 2021">
</p>

---

Write is a terminal-based markdown editor built in Rust. As you write, an LLM works in the background to fix typos, grammar, and formatting — so you never break flow. Documents save as `.md` files.

The interface uses a warm **Taliesin** color scheme inspired by Frank Lloyd Wright's prairie aesthetic: parchment backgrounds, walnut text, terracotta and maroon accents, gold decorative lines.

## Features

- **Proactive LLM cleanup** — typos, grammar, and markdown formatting are corrected automatically in the background
- **Screenplay formatting** — scene headings, character names, dialogue, camera directions, transitions, and shots are recognized and formatted to standard conventions
- **Scramble animation** — corrected characters cycle through random letters before resolving, with each character settling independently
- **Dither transitions** — startup and screen changes use a block-character dissolve effect
- **Word wrap** — text wraps at the terminal edge automatically
- **Three LLM providers** — Claude (Anthropic), OpenAI, and OpenRouter with automatic detection
- **Smart triggers** — LLM fires on sentence boundaries (300ms), word boundaries (800ms), or idle (2s)
- **Hash-based staleness** — if you type while the LLM is working, stale responses are discarded
- **Zero config** — works as a plain editor with no API key; just set one to enable cleanup

## Installation

Download the latest binary from [Releases](https://github.com/cheetohsum/write/releases/latest):

| Platform | Asset |
|----------|-------|
| Windows x86_64 | `Write-windows-x86_64.exe` |
| macOS Apple Silicon | `Write-macos-aarch64.zip` |
| macOS Intel | `Write-macos-x86_64.zip` |

Or build from source:

```
git clone https://github.com/cheetohsum/write.git
cd write
cargo build --release
```

## Configuration

Create a `.env` file in the project directory:

```
ANTHROPIC_API_KEY=sk-ant-...
```

The app auto-detects which provider to use based on available keys. Priority: `ANTHROPIC_API_KEY` → `OPENAI_API_KEY` → `OPENROUTER_API_KEY`.

| Variable | Purpose |
|----------|---------|
| `ANTHROPIC_API_KEY` | Claude API (preferred) |
| `OPENAI_API_KEY` | OpenAI API |
| `OPENROUTER_API_KEY` | OpenRouter API |
| `LLM_PROVIDER` | Force a provider (`claude`, `openai`, `openrouter`) |
| `LLM_MODEL` | Override the default model |

## Keybindings

| Key | Action |
|-----|--------|
| `Ctrl+S` | Save document |
| `Ctrl+Q` / `Esc` | Quit (confirms if unsaved) |
| `Ctrl+L` | Toggle LLM on/off |
| `Tab` | Switch fields (startup screen) |
| `Enter` | Begin writing (startup screen) |

## Screenplay Support

Write recognizes standard screenplay elements and formats them automatically:

```markdown
**FADE IN:**

**EXT. DESERT HIGHWAY - DAY**

A heat shimmer ripples across empty asphalt stretching to the horizon.

**WIDE SHOT**

A single car appears in the distance.

**JACK**
*(squinting)*
We should have turned left at Albuquerque.

**MARIA**
That's what I said three hours ago.

**CUT TO:**
```

Supported elements: scene headings (INT./EXT.), character names, dialogue, parentheticals, transitions (CUT TO, FADE IN/OUT, SMASH CUT), camera directions (CLOSE UP, WIDE SHOT, PAN, TRACKING SHOT, POV, OVER THE SHOULDER), shot descriptions (ANGLE ON, INSERT, MONTAGE), and continuation markers.

## Architecture

```
src/
├── main.rs          # Terminal setup, panic hook
├── app.rs           # State machine, event loop, animations
├── ui.rs            # Ratatui rendering, dither overlays
├── editor.rs        # TextArea wrapper, word wrap, content replacement
├── theme.rs         # Taliesin color palette
├── config.rs        # Env loading, provider detection
├── keybindings.rs   # Key → Action dispatch
└── llm/
    ├── mod.rs       # Background task, mpsc channels
    ├── claude.rs    # Anthropic Messages API
    ├── openai.rs    # OpenAI Chat Completions API
    ├── openrouter.rs
    └── prompt.rs    # System prompt with screenplay rules
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
