#[derive(Debug, Clone, PartialEq, Eq)]
pub struct History {
    entries: Vec<String>,
    browse_index: Option<usize>,
    stashed_draft: Option<String>,
}

impl Default for History {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            browse_index: None,
            stashed_draft: None,
        }
    }
}

impl History {
    pub fn push_submission(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        if self.entries.last() != Some(&text) {
            self.entries.push(text);
        }
        self.browse_index = None;
        self.stashed_draft = None;
    }

    pub fn browsing(&self) -> bool {
        self.browse_index.is_some()
    }

    pub fn reset_browse(&mut self) {
        self.browse_index = None;
        self.stashed_draft = None;
    }

    /// Returns new draft text when history entry changes.
    pub fn history_up(&mut self, current_draft: &str) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }
        let idx = match self.browse_index {
            None => {
                self.stashed_draft = Some(current_draft.to_string());
                self.entries.len().saturating_sub(1)
            }
            Some(0) => return None,
            Some(i) => i - 1,
        };
        self.browse_index = Some(idx);
        Some(self.entries[idx].clone())
    }

    pub fn history_down(&mut self, _current_draft: &str) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }
        let Some(i) = self.browse_index else {
            return None;
        };
        if i + 1 >= self.entries.len() {
            self.browse_index = None;
            return Some(self.stashed_draft.take().unwrap_or_default());
        }
        self.browse_index = Some(i + 1);
        Some(self.entries[i + 1].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupes_consecutive_submissions() {
        let mut h = History::default();
        h.push_submission("a".into());
        h.push_submission("a".into());
        assert_eq!(h.entries.len(), 1);
    }

    #[test]
    fn history_up_down() {
        let mut h = History::default();
        h.push_submission("one".into());
        h.push_submission("two".into());
        let up = h.history_up("hello").unwrap();
        assert_eq!(up, "two");
        let up = h.history_up("hello").unwrap();
        assert_eq!(up, "one");
        let down = h.history_down("").unwrap();
        assert_eq!(down, "two");
        let down = h.history_down("").unwrap();
        assert_eq!(down, "hello");
        assert!(!h.browsing());
    }
}
