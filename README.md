# herdr-input-bar

A herdr plugin that adds a dedicated **Input Bar** below a terminal pane so you can compose multi-line text and submit it into that pane.

The Input Bar stays bound to one **Target Pane** for its lifetime. Submissions go to that pane whether it is running a shell or an AI agent.

## Requirements

- [Rust](https://rustup.rs/) (edition 2021 toolchain)
- herdr `>= 0.9.0`
- macOS or Linux

## Build

```bash
cargo build --release
```

The binary is written to `target/release/herdr-input-bar`.

Plugin metadata and actions are declared in [`herdr-plugin.toml`](herdr-plugin.toml). herdr runs `cargo build --release` via the `[[build]]` entry, then launches:

| Command | Role |
| --- | --- |
| `herdr-input-bar open` | Open / refocus the Input Bar for the current pane |
| `herdr-input-bar bar` | Run the Input Bar pane UI |

## Install

Clone this repository into your herdr plugins directory (or symlink it there), then reload plugins in herdr so it picks up `herdr-plugin.toml`:

```bash
git clone https://github.com/AlanusMeminius/herdr-input-bar.git
# place or link the clone where herdr loads plugins from, then reload plugins
```

After install, use the **Open Input Bar** action from the workspace context (see `[[actions]]` in `herdr-plugin.toml`).

## Usage

1. Focus a terminal pane you want to type into.
2. Run **Open Input Bar** — a composer opens below that pane (the Target Pane).
3. Write your draft (multi-line supported).
4. Press **Enter** to submit the draft into the Target Pane (and clear it).
5. Press **Ctrl+Enter** or **Cmd+Enter** to paste the draft as text without a trailing Enter (bracketed paste).
6. Press **Ctrl+C** to close the Input Bar.

History within one Input Bar session can be recalled with the up/down keys; it is discarded when the bar closes.

## Development

```bash
cargo test
cargo build --release
```

Domain terms and relationships are documented in [`CONTEXT.md`](CONTEXT.md).

## License

No license file is included yet. Add one if you intend to redistribute under specific terms.
