use regex::Regex;

#[derive(Debug, Clone)]
pub struct SearchState {
    pub query: String,
    pub is_regex: bool,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            is_regex: true,
        }
    }
}

impl SearchState {
    pub fn compile(&self) -> Option<Regex> {
        if self.query.is_empty() {
            return None;
        }
        let pattern = if self.is_regex {
            self.query.clone()
        } else {
            regex::escape(&self.query)
        };
        regex::RegexBuilder::new(&pattern)
            .case_insensitive(true)
            .build()
            .ok()
    }

    pub fn matches(&self, line: &str) -> bool {
        self.compile().map(|re| re.is_match(line)).unwrap_or(true)
    }

    pub fn next_match(&self, lines: &[String], from: usize) -> Option<usize> {
        let re = self.compile()?;
        let mut idx = from + 1;
        if idx >= lines.len() {
            idx = 0;
        }
        let start = idx;
        loop {
            if re.is_match(&lines[idx]) {
                return Some(idx);
            }
            idx = (idx + 1) % lines.len();
            if idx == start {
                return None;
            }
        }
    }

    pub fn prev_match(&self, lines: &[String], from: usize) -> Option<usize> {
        let re = self.compile()?;
        let len = lines.len();
        if len == 0 {
            return None;
        }
        let mut idx = from.checked_sub(1).unwrap_or(len - 1);
        let start = idx;
        loop {
            if re.is_match(&lines[idx]) {
                return Some(idx);
            }
            if idx == 0 {
                idx = len - 1;
            } else {
                idx -= 1;
            }
            if idx == start {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_matches() {
        let s = SearchState {
            query: "err".into(),
            ..Default::default()
        };
        assert!(s.matches("ERROR: something failed"));
        assert!(!s.matches("INFO: ok"));
    }

    #[test]
    fn test_next_prev() {
        let lines = vec![
            "INFO ok".into(),
            "ERROR one".into(),
            "INFO fine".into(),
            "ERROR two".into(),
        ];
        let s = SearchState {
            query: "ERROR".into(),
            ..Default::default()
        };
        assert_eq!(s.next_match(&lines, 0), Some(1));
        assert_eq!(s.next_match(&lines, 1), Some(3));
        assert_eq!(s.next_match(&lines, 3), Some(1));
        assert_eq!(s.prev_match(&lines, 0), Some(3));
    }
}
