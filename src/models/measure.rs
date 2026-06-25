use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

pub const DEFAULT_MEASURE_LEN: usize = 8;
pub const DEFAULT_BOX_LEN: usize = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MeasureEnd {
    SingleBar,
    DoubleBar,
    RepeatStart,
    RepeatEnd,
    RepeatDouble,
    None,
}

fn default_measure_end() -> MeasureEnd {
    MeasureEnd::SingleBar
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Measure {
    pub strings: Vec<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub annotations: BTreeMap<usize, String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub box_starts: Vec<usize>,
    #[serde(default = "default_measure_end")]
    pub end_decorator: MeasureEnd,
}

impl Measure {
    pub fn new(num_strings: usize, len: usize) -> Self {
        let empty_string = "-".repeat(len);
        Self {
            strings: vec![empty_string; num_strings],
            annotations: BTreeMap::new(),
            box_starts: Vec::new(),
            end_decorator: MeasureEnd::SingleBar,
        }
    }

    pub fn len(&self) -> usize {
        self.strings.first().map_or(0, |s| s.len())
    }

    pub fn get_char(&self, string_idx: usize, col_idx: usize) -> char {
        self.strings.get(string_idx)
            .and_then(|s| s.chars().nth(col_idx))
            .unwrap_or('-')
    }

    pub fn set_char(&mut self, string_idx: usize, col_idx: usize, c: char) {
        if let Some(s) = self.strings.get_mut(string_idx) {
            if col_idx < s.len() {
                s.replace_range(col_idx..col_idx+1, &c.to_string());
            }
        }
    }

    pub fn delete_char(&mut self, string_idx: usize, col_idx: usize) {
        if let Some(s) = self.strings.get_mut(string_idx) {
            if col_idx < s.len() {
                s.remove(col_idx);
                s.push('-');
            }
        }
    }

    pub fn insert_col(&mut self, col_idx: usize) {
        for s in &mut self.strings {
            if col_idx <= s.len() {
                s.insert(col_idx, '-');
            }
        }
        let mut new_ann = BTreeMap::new();
        for (&k, v) in &self.annotations {
            if k >= col_idx {
                new_ann.insert(k + 1, v.clone());
            } else {
                new_ann.insert(k, v.clone());
            }
        }
        self.annotations = new_ann;

        for bs in &mut self.box_starts {
            if *bs >= col_idx {
                *bs += 1;
            }
        }
    }

    pub fn delete_col(&mut self, col_idx: usize) {
        if self.len() == 0 || col_idx >= self.len() { return; }
        for s in &mut self.strings {
            s.remove(col_idx);
        }
        
        let mut new_ann = BTreeMap::new();
        for (&k, v) in &self.annotations {
            if k == col_idx {
            } else if k > col_idx {
                new_ann.insert(k - 1, v.clone());
            } else {
                new_ann.insert(k, v.clone());
            }
        }
        self.annotations = new_ann;

        self.box_starts.retain(|&x| x != col_idx);
        for bs in &mut self.box_starts {
            if *bs > col_idx {
                *bs -= 1;
            }
        }
    }

    pub fn split_at(&mut self, col_idx: usize) -> Self {
        let mut new_measure = Measure {
            strings: Vec::new(),
            annotations: BTreeMap::new(),
            box_starts: Vec::new(),
            end_decorator: self.end_decorator.clone(),
        };

        for s in &mut self.strings {
            let right_part = if col_idx < s.len() { s[col_idx..].to_string() } else { String::new() };
            if col_idx < s.len() { s.truncate(col_idx); }
            new_measure.strings.push(right_part);
        }

        let mut to_remove = Vec::new();
        for (&k, v) in &self.annotations {
            if k >= col_idx {
                new_measure.annotations.insert(k - col_idx, v.clone());
                to_remove.push(k);
            }
        }
        for k in to_remove { self.annotations.remove(&k); }

        self.box_starts.retain(|&x| {
            if x >= col_idx {
                new_measure.box_starts.push(x - col_idx);
                false
            } else {
                true
            }
        });

        self.end_decorator = MeasureEnd::SingleBar;

        new_measure
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VirtualColumn {
    pub strings: Vec<char>,
    pub annotation: Option<String>,
    pub is_box_start: bool,
    pub is_barline: bool,
}

impl VirtualColumn {
    pub fn get_char(&self, string_idx: usize) -> char {
        if self.is_barline { return '|'; }
        self.strings.get(string_idx).copied().unwrap_or('-')
    }

    pub fn is_blank(&self) -> bool {
        self.strings.iter().all(|&c| c == '-') && self.annotation.is_none() && !self.is_box_start && !self.is_barline
    }
}
