use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TabColumn {
    pub strings: [char; 6],
    pub annotation: Option<String>,
}

impl TabColumn {
    pub fn new() -> Self {
        Self {
            strings: ['-'; 6],
            annotation: None,
        }
    }

    pub fn barline() -> Self {
        Self {
            strings: ['|'; 6],
            annotation: None,
        }
    }

    pub fn is_blank(&self) -> bool {
        self.strings.iter().all(|&c| c == '-') && self.annotation.is_none()
    }

    pub fn is_barline(&self) -> bool {
        self.strings.iter().all(|&c| c == '|')
    }
}

impl Default for TabColumn {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TabDocument {
    pub columns: Vec<TabColumn>,
    pub tuning: [char; 6],
    pub clipboard: Vec<TabColumn>,
}

impl TabDocument {
    pub fn new() -> Self {
        // Start with an empty document of 80 columns.
        let mut columns = Vec::new();
        for _ in 0..80 {
            columns.push(TabColumn::new());
        }
        
        Self {
            columns,
            // Standard guitar tuning (high e to low E). 
            // 0 = highest string (e), 5 = lowest string (E)
            tuning: ['e', 'B', 'G', 'D', 'A', 'E'],
            clipboard: Vec::new(),
        }
    }
}

impl Default for TabDocument {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Cursor {
    pub col: usize,
    pub string: usize, // 0..=5
}

impl Cursor {
    pub fn new() -> Self {
        Self { col: 0, string: 0 }
    }
}
