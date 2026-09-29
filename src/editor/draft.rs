use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    pub lines: Vec<String>,
    pub cursor: Cursor,
}

impl Default for Draft {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            cursor: Cursor { row: 0, col: 0 },
        }
    }
}

impl Draft {
    pub fn from_text(text: &str) -> Self {
        if text.is_empty() {
            return Self::default();
        }
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        let row = lines.len() - 1;
        let col = lines[row].chars().count();
        Self {
            lines,
            cursor: Cursor { row, col },
        }
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }

    pub fn on_first_line(&self) -> bool {
        self.cursor.row == 0
    }

    pub fn on_last_line(&self) -> bool {
        self.cursor.row + 1 == self.lines.len()
    }

    fn line_mut(&mut self, row: usize) -> &mut String {
        &mut self.lines[row]
    }

    fn clamp_cursor(&mut self) {
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        if self.cursor.row >= self.lines.len() {
            self.cursor.row = self.lines.len() - 1;
        }
        let char_len = self.lines[self.cursor.row].chars().count();
        if self.cursor.col > char_len {
            self.cursor.col = char_len;
        }
    }

    pub fn insert_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        for part in s.split_inclusive('\n') {
            if part.ends_with('\n') {
                let without_nl = &part[..part.len() - 1];
                if !without_nl.is_empty() {
                    self.insert_at_cursor(without_nl);
                }
                self.insert_newline();
            } else {
                self.insert_at_cursor(part);
            }
        }
        self.clamp_cursor();
    }

    fn insert_at_cursor(&mut self, text: &str) {
        let row = self.cursor.row;
        let byte_idx = char_index_to_byte(&self.lines[row], self.cursor.col);
        self.line_mut(row).insert_str(byte_idx, text);
        self.cursor.col += text.chars().count();
    }

    pub fn insert_newline(&mut self) {
        let row = self.cursor.row;
        let byte_idx = char_index_to_byte(&self.lines[row], self.cursor.col);
        let rest = self.lines[row].split_off(byte_idx);
        self.cursor.row += 1;
        self.cursor.col = 0;
        self.lines.insert(self.cursor.row, rest);
    }

    pub fn move_left(&mut self) {
        if self.cursor.col > 0 {
            self.cursor.col -= 1;
            return;
        }
        if self.cursor.row > 0 {
            self.cursor.row -= 1;
            self.cursor.col = self.lines[self.cursor.row].chars().count();
        }
    }

    pub fn move_right(&mut self) {
        let char_len = self.lines[self.cursor.row].chars().count();
        if self.cursor.col < char_len {
            self.cursor.col += 1;
            return;
        }
        if self.cursor.row + 1 < self.lines.len() {
            self.cursor.row += 1;
            self.cursor.col = 0;
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor.row == 0 {
            return;
        }
        self.cursor.row -= 1;
        self.clamp_cursor();
    }

    pub fn move_down(&mut self) {
        if self.cursor.row + 1 >= self.lines.len() {
            return;
        }
        self.cursor.row += 1;
        self.clamp_cursor();
    }

    /// Clicking inside a wide character lands before it.
    pub fn move_to(&mut self, row: usize, display_col: usize) {
        self.cursor.row = row.min(self.lines.len() - 1);
        let mut width = 0;
        let mut col = 0;
        for ch in self.lines[self.cursor.row].chars() {
            let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if width + w > display_col {
                break;
            }
            width += w;
            col += 1;
        }
        self.cursor.col = col;
    }

    pub fn move_home(&mut self) {
        self.cursor.col = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor.col = self.lines[self.cursor.row].chars().count();
    }

    pub fn delete_backward(&mut self) {
        if self.cursor.col > 0 {
            let row = self.cursor.row;
            let byte_idx = char_index_to_byte(&self.lines[row], self.cursor.col);
            let prev_byte = prev_char_byte(&self.lines[row], byte_idx);
            self.line_mut(row).replace_range(prev_byte..byte_idx, "");
            self.cursor.col -= 1;
            return;
        }
        if self.cursor.row == 0 {
            return;
        }
        let row = self.cursor.row;
        let prev_len = self.lines[row - 1].chars().count();
        let current = self.lines.remove(row);
        self.cursor.row -= 1;
        self.cursor.col = prev_len;
        self.line_mut(self.cursor.row).push_str(&current);
    }

    pub fn delete_forward(&mut self) {
        let row = self.cursor.row;
        let char_len = self.lines[row].chars().count();
        if self.cursor.col < char_len {
            let byte_idx = char_index_to_byte(&self.lines[row], self.cursor.col);
            let next_byte = next_char_byte(&self.lines[row], byte_idx);
            self.line_mut(row).replace_range(byte_idx..next_byte, "");
            return;
        }
        if self.cursor.row + 1 >= self.lines.len() {
            return;
        }
        let next = self.lines.remove(row + 1);
        self.line_mut(row).push_str(&next);
    }

    pub fn kill_line_prefix(&mut self) {
        let row = self.cursor.row;
        let byte_idx = char_index_to_byte(&self.lines[row], self.cursor.col);
        self.line_mut(row).replace_range(0..byte_idx, "");
        self.cursor.col = 0;
    }

    pub fn kill_word_backward(&mut self) {
        let row = self.cursor.row;
        let line = self.lines[row].clone();
        if self.cursor.col == 0 {
            return;
        }
        let byte_cursor = char_index_to_byte(&line, self.cursor.col);
        let before = &line[..byte_cursor];
        let new_byte = find_word_start_byte(before);
        let remove_len = byte_cursor - new_byte;
        let new_col = before[..new_byte].chars().count();
        let start = byte_cursor - remove_len;
        self.line_mut(row).replace_range(start..byte_cursor, "");
        self.cursor.col = new_col;
    }
}

pub fn display_width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

pub fn cursor_display_col(line: &str, char_col: usize) -> usize {
    let byte_idx = char_index_to_byte(line, char_col);
    display_width(&line[..byte_idx])
}

pub fn char_index_to_byte(line: &str, char_col: usize) -> usize {
    if char_col == 0 {
        return 0;
    }
    line.char_indices()
        .nth(char_col)
        .map(|(i, _)| i)
        .unwrap_or(line.len())
}

fn prev_char_byte(line: &str, byte_idx: usize) -> usize {
    line[..byte_idx]
        .char_indices()
        .next_back()
        .map(|(i, _)| i)
        .unwrap_or(0)
}

fn next_char_byte(line: &str, byte_idx: usize) -> usize {
    line[byte_idx..]
        .char_indices()
        .nth(1)
        .map(|(i, _)| byte_idx + i)
        .unwrap_or(line.len())
}

fn find_word_start_byte(before: &str) -> usize {
    let bytes = before.as_bytes();
    let mut i = bytes.len();
    while i > 0 && before.as_bytes()[i - 1].is_ascii_whitespace() {
        i -= 1;
    }
    while i > 0 {
        let prev = prev_char_byte(before, i);
        if before.as_bytes()[prev..i]
            .iter()
            .all(|b| b.is_ascii_whitespace())
        {
            break;
        }
        i = prev;
    }
    i
}

/// One screen row of a soft-wrapped Draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualRow {
    pub line: usize,
    pub start_col: usize,
    pub text: String,
}

pub fn wrap_lines(draft: &Draft, width: usize) -> Vec<VisualRow> {
    let width = width.max(2);
    let mut rows = Vec::new();
    for (line_idx, line) in draft.lines.iter().enumerate() {
        let mut row = VisualRow { line: line_idx, start_col: 0, text: String::new() };
        let mut row_width = 0;
        for (col, ch) in line.chars().enumerate() {
            let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if row_width + w > width {
                let next = VisualRow { line: line_idx, start_col: col, text: String::new() };
                rows.push(std::mem::replace(&mut row, next));
                row_width = 0;
            }
            row.text.push(ch);
            row_width += w;
        }
        rows.push(row);
    }
    rows
}

/// Screen (row, col) of the cursor within `rows`; a cursor at a wrap boundary sits at the start of the next row.
pub fn cursor_visual_pos(draft: &Draft, rows: &[VisualRow]) -> (usize, usize) {
    let Cursor { row, col } = draft.cursor;
    let idx = rows
        .iter()
        .rposition(|r| r.line == row && r.start_col <= col)
        .unwrap_or(0);
    let r = &rows[idx];
    let x = cursor_display_col(&r.text, col - r.start_col);
    (idx, x)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Viewport {
    pub first_line: usize,
    pub height: usize,
}

/// `cursor_row` and `total_lines` are in the same unit (logical or visual rows).
pub fn scroll_viewport(
    cursor_row: usize,
    total_lines: usize,
    viewport_height: usize,
    current_first_line: usize,
) -> Viewport {
    let total_lines = total_lines.max(1);
    let height = viewport_height.max(1);
    let mut first = current_first_line;
    if cursor_row < first {
        first = cursor_row;
    } else if cursor_row >= first + height {
        first = cursor_row + 1 - height;
    }
    if first + height > total_lines {
        let max_first = total_lines.saturating_sub(height);
        if first > max_first {
            first = max_first;
        }
    }
    Viewport {
        first_line: first,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_newline() {
        let mut d = Draft::default();
        d.insert_str("hello");
        d.insert_newline();
        d.insert_str("world");
        assert_eq!(d.text(), "hello\nworld");
        assert_eq!(d.cursor, Cursor { row: 1, col: 5 });
    }

    #[test]
    fn cjk_width() {
        assert_eq!(display_width("中"), 2);
        assert_eq!(cursor_display_col("a中", 2), 3);
    }

    #[test]
    fn move_to_screen_position() {
        let mut d = Draft::from_text("a中b\nxy");
        d.move_to(0, 2);
        assert_eq!(d.cursor, Cursor { row: 0, col: 1 });
        d.move_to(0, 3);
        assert_eq!(d.cursor, Cursor { row: 0, col: 2 });
        d.move_to(9, 99);
        assert_eq!(d.cursor, Cursor { row: 1, col: 2 });
    }

    #[test]
    fn wraps_wide_chars_and_places_cursor() {
        let mut d = Draft::from_text("ab中文c\nxy");
        let rows = wrap_lines(&d, 4);
        let texts: Vec<_> = rows.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(texts, ["ab中", "文c", "xy"]);
        assert_eq!(rows[1].start_col, 3);
        d.cursor = Cursor { row: 0, col: 3 };
        assert_eq!(cursor_visual_pos(&d, &rows), (1, 0));
        d.cursor = Cursor { row: 0, col: 5 };
        assert_eq!(cursor_visual_pos(&d, &rows), (1, 3));
        d.cursor = Cursor { row: 1, col: 1 };
        assert_eq!(cursor_visual_pos(&d, &rows), (2, 1));
    }

    #[test]
    fn ctrl_u_and_w() {
        let mut d = Draft::from_text("hello world");
        d.cursor.col = 5;
        d.kill_line_prefix();
        assert_eq!(d.lines[0], " world");
        d.cursor.col = 6;
        d.kill_word_backward();
        assert_eq!(d.lines[0], " ");
    }

    #[test]
    fn scroll_keeps_cursor_visible() {
        let mut d = Draft::default();
        for i in 0..10 {
            d.insert_str(&format!("line{i}"));
            d.insert_newline();
        }
        d.cursor.row = 8;
        let vp = scroll_viewport(d.cursor.row, d.lines.len(), 3, 0);
        assert!(d.cursor.row >= vp.first_line);
        assert!(d.cursor.row < vp.first_line + vp.height);
    }

    #[test]
    fn history_gate_rows() {
        let mut d = Draft::from_text("a\nb\nc");
        d.cursor.row = 1;
        assert!(!d.on_first_line());
        assert!(!d.on_last_line());
        d.cursor.row = 0;
        assert!(d.on_first_line());
        d.cursor.row = 2;
        assert!(d.on_last_line());
    }
}
