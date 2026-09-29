use crate::editor::{
    cursor_display_col, cursor_visual_pos, display_width, scroll_viewport, wrap_lines, Draft,
    History, Viewport, VisualRow,
};
use crate::herdr;
use crate::state::{state_dir_from_env, TargetBarState};
use crossterm::cursor::{MoveTo, SetCursorStyle};
use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, KeyboardEnhancementFlags, MouseButton, MouseEvent, MouseEventKind,
    PushKeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
};
use crossterm::style::{Attribute, Print, ResetColor, SetAttribute};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, BeginSynchronizedUpdate, Clear, ClearType,
    EndSynchronizedUpdate,
};
use crossterm::{execute, queue};
use std::env;
use std::io::{self, stdout, Write};
use std::time::{Duration, Instant};

const TARGET_ENV: &str = "INPUT_BAR_TARGET_PANE";
const WATCH_INTERVAL: Duration = Duration::from_secs(2);

pub fn run() -> io::Result<()> {
    let target_pane = env::var(TARGET_ENV).map_err(|_| {
        io::Error::new(io::ErrorKind::NotFound, "INPUT_BAR_TARGET_PANE unset")
    })?;
    let bar_pane = env::var("HERDR_PANE_ID").unwrap_or_default();
    let state_dir = state_dir_from_env()?;

    {
        let mut state = TargetBarState::load(&state_dir)?;
        if !bar_pane.is_empty() {
            state.register(&target_pane, &bar_pane);
            state.save(&state_dir)?;
        }
    }

    let mut draft = Draft::default();
    let mut history = History::default();
    let mut rejected: Option<String> = None;
    let mut viewport = Viewport {
        first_line: 0,
        height: 1,
    };
    let mut last_watch = Instant::now();

    let mut out = stdout();
    enable_raw_mode()?;
    let _ = execute!(
        out,
        EnableBracketedPaste,
        EnableFocusChange,
        EnableMouseCapture,
        SetCursorStyle::SteadyBar,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
        ),
    );

    let run_result = run_loop(
        &mut out,
        &target_pane,
        &bar_pane,
        &state_dir,
        &mut draft,
        &mut history,
        &mut rejected,
        &mut viewport,
        &mut last_watch,
    );

    let _ = execute!(
        out,
        PopKeyboardEnhancementFlags,
        DisableBracketedPaste,
        DisableFocusChange,
        DisableMouseCapture,
        ResetColor,
        SetCursorStyle::DefaultUserShape,
    );
    disable_raw_mode()?;

    {
        let mut state = TargetBarState::load(&state_dir)?;
        if !bar_pane.is_empty() {
            state.unregister_bar(&bar_pane);
            state.save(&state_dir)?;
        }
    }

    run_result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopExit {
    Close,
    EscToTarget,
}

fn run_loop(
    out: &mut io::Stdout,
    target_pane: &str,
    bar_pane: &str,
    state_dir: &std::path::Path,
    draft: &mut Draft,
    history: &mut History,
    rejected: &mut Option<String>,
    viewport: &mut Viewport,
    last_watch: &mut Instant,
) -> io::Result<()> {
    let mut dirty = true;
    let mut visual: Vec<VisualRow> = Vec::new();
    loop {
        if last_watch.elapsed() >= WATCH_INTERVAL {
            *last_watch = Instant::now();
            if herdr::pane_get(target_pane)?.is_none() {
                return Ok(());
            }
        }

        if dirty {
            let (cols, rows) = crossterm::terminal::size()?;
            let status_rows = u16::from(rejected.is_some());
            let edit_rows = rows.saturating_sub(status_rows).max(1) as usize;
            // Keep the last column free so a cursor at the end of a full row stays on screen.
            visual = wrap_lines(draft, (cols as usize).saturating_sub(1));
            let cursor = cursor_visual_pos(draft, &visual);
            *viewport = scroll_viewport(cursor.0, visual.len(), edit_rows, viewport.first_line);
            draw(
                out,
                &visual,
                cursor,
                viewport,
                rows,
                cols,
                rejected.as_deref(),
            )?;
            dirty = false;
        }

        if crossterm::event::poll(Duration::from_millis(100))? {
            dirty = true;
            match crossterm::event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Release => {}
                Event::Key(key) => {
                    if history.browsing() && is_editing_key(&key) {
                        history.reset_browse();
                    }
                    match handle_key(
                        key,
                        draft,
                        history,
                        rejected,
                        target_pane,
                        bar_pane,
                    )? {
                        Some(LoopExit::Close) => return Ok(()),
                        Some(LoopExit::EscToTarget) => {
                            if !bar_pane.is_empty() {
                                let _ = herdr::focus_pane_toward(bar_pane, target_pane);
                            } else {
                                let _ = herdr::focus_neighbor(target_pane, "down");
                            }
                            continue;
                        }
                        None => {}
                    }
                }
                Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left) | MouseEventKind::Drag(MouseButton::Left),
                    row,
                    column,
                    ..
                }) if (row as usize) < viewport.height => {
                    if let Some(r) = visual.get(viewport.first_line + row as usize) {
                        let line = &draft.lines[r.line];
                        let row_start = cursor_display_col(line, r.start_col);
                        let x = (column as usize).min(display_width(&r.text));
                        draft.move_to(r.line, row_start + x);
                    }
                }
                Event::Paste(text) => {
                    draft.insert_str(&text);
                    *rejected = None;
                }
                Event::FocusGained => {
                    let _ = execute!(out, SetCursorStyle::SteadyBar);
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
        let _ = state_dir;
    }
}

fn is_editing_key(key: &KeyEvent) -> bool {
    !matches!(
        key.code,
        KeyCode::Up | KeyCode::Down if key.modifiers.is_empty()
    )
}

fn handle_key(
    key: KeyEvent,
    draft: &mut Draft,
    history: &mut History,
    rejected: &mut Option<String>,
    target_pane: &str,
    _bar_pane: &str,
) -> io::Result<Option<LoopExit>> {
    use KeyCode::*;
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let super_ = key.modifiers.contains(KeyModifiers::SUPER);

    match key.code {
        Char('c') if ctrl => return Ok(Some(LoopExit::Close)),
        Char('d') if ctrl && draft.is_empty() => return Ok(Some(LoopExit::Close)),
        Esc => return Ok(Some(LoopExit::EscToTarget)),
        Enter if shift || alt => {
            draft.insert_newline();
            *rejected = None;
        }
        Enter if ctrl || super_ => {
            submit_text_only(draft, history, rejected, target_pane)?;
        }
        Enter => {
            submit(draft, history, rejected, target_pane)?;
        }
        Backspace => {
            draft.delete_backward();
            *rejected = None;
        }
        Delete => {
            draft.delete_forward();
            *rejected = None;
        }
        Left => draft.move_left(),
        Right => draft.move_right(),
        Up if draft.on_first_line() => {
            if let Some(text) = history.history_up(&draft.text()) {
                *draft = Draft::from_text(&text);
                *rejected = None;
            } else {
                draft.move_up();
            }
        }
        Up => draft.move_up(),
        Down if draft.on_last_line() => {
            if let Some(text) = history.history_down(&draft.text()) {
                *draft = Draft::from_text(&text);
                *rejected = None;
            } else {
                draft.move_down();
            }
        }
        Down => draft.move_down(),
        Home => draft.move_home(),
        End => draft.move_end(),
        Char('a') if ctrl => draft.move_home(),
        Char('e') if ctrl => draft.move_end(),
        Char('u') if ctrl => {
            draft.kill_line_prefix();
            *rejected = None;
        }
        Char('w') if ctrl => {
            draft.kill_word_backward();
            *rejected = None;
        }
        Char(ch) if !ctrl && !alt => {
            draft.insert_str(&ch.to_string());
            *rejected = None;
        }
        _ => {}
    }
    Ok(None)
}

fn submit(
    draft: &mut Draft,
    history: &mut History,
    rejected: &mut Option<String>,
    target_pane: &str,
) -> io::Result<()> {
    let text = draft.text();
    let info = herdr::pane_get(target_pane)?;
    let Some(info) = info else {
        return Ok(());
    };

    if text.is_empty() {
        herdr::send_keys(target_pane, &["enter"])?;
        *rejected = None;
        return Ok(());
    }

    if herdr::is_agent_target(&info) {
        match herdr::agent_prompt(target_pane, &text) {
            Ok(()) => {
                history.push_submission(text);
                *draft = Draft::default();
                *rejected = None;
            }
            Err(msg) => {
                *rejected = Some(msg);
            }
        }
        return Ok(());
    }

    let payload = format!("\x1b[200~{text}\x1b[201~");
    herdr::send_text(target_pane, &payload)?;
    herdr::send_keys(target_pane, &["enter"])?;
    history.push_submission(text);
    *draft = Draft::default();
    *rejected = None;
    Ok(())
}

fn submit_text_only(
    draft: &mut Draft,
    history: &mut History,
    rejected: &mut Option<String>,
    target_pane: &str,
) -> io::Result<()> {
    let text = draft.text();
    if text.is_empty() {
        return Ok(());
    }
    let payload = format!("\x1b[200~{text}\x1b[201~");
    herdr::send_text(target_pane, &payload)?;
    history.push_submission(text);
    *draft = Draft::default();
    *rejected = None;
    Ok(())
}

fn draw(
    out: &mut io::Stdout,
    visual: &[VisualRow],
    cursor: (usize, usize),
    viewport: &Viewport,
    rows: u16,
    cols: u16,
    rejected: Option<&str>,
) -> io::Result<()> {
    queue!(out, BeginSynchronizedUpdate)?;
    let width = cols as usize;
    for (i, row_idx) in (viewport.first_line..viewport.first_line + viewport.height).enumerate() {
        let text = visual.get(row_idx).map(|r| r.text.as_str()).unwrap_or("");
        queue!(
            out,
            MoveTo(0, i as u16),
            Print(text),
            Clear(ClearType::UntilNewLine)
        )?;
    }

    let mut next_y = viewport.height as u16;
    if let Some(r) = rejected {
        queue!(
            out,
            MoveTo(0, next_y),
            SetAttribute(Attribute::Dim),
            Print(truncate_to_width(r, width)),
            SetAttribute(Attribute::Reset),
            Clear(ClearType::UntilNewLine),
        )?;
        next_y += 1;
    }
    if next_y < rows {
        queue!(out, MoveTo(0, next_y), Clear(ClearType::FromCursorDown))?;
    }

    let cursor_row = cursor.0.saturating_sub(viewport.first_line) as u16;
    queue!(
        out,
        MoveTo(cursor.1 as u16, cursor_row),
        EndSynchronizedUpdate,
    )?;
    out.flush()?;
    Ok(())
}

fn truncate_to_width(s: &str, max_width: usize) -> String {
    let mut w = 0;
    let mut out = String::new();
    for ch in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if w + cw > max_width {
            break;
        }
        w += cw;
        out.push(ch);
    }
    out
}
