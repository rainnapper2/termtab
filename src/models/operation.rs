use serde::{Serialize, Deserialize};
use super::measure::VirtualColumn;
use super::document::TabDocument;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Operation {
    InsertColumn { col: usize, content: VirtualColumn },
    DeleteColumn { col: usize, content: VirtualColumn },
    SetChar { col: usize, string: usize, old_c: char, new_c: char },
    DeleteChar { col: usize, string: usize, old_c: char },
    InsertChar { col: usize, string: usize, c: char },
    SetAnnotation { col: usize, old_ann: Option<String>, new_ann: Option<String> },
    InsertBarline { col: usize },
    DeleteBarline { col: usize },
    ReplaceChars { start_col: usize, string: usize, old_chars: Vec<char>, new_chars: Vec<char> },
    DeleteColumnsRange { start: usize, end: usize, contents: Vec<VirtualColumn> },
    InsertColumnsRange { start: usize, contents: Vec<VirtualColumn> },
}

impl Operation {
    pub fn is_noop(&self) -> bool {
        match self {
            Operation::SetChar { old_c, new_c, .. } => old_c == new_c,
            Operation::ReplaceChars { old_chars, new_chars, .. } => old_chars == new_chars,
            Operation::SetAnnotation { old_ann, new_ann, .. } => old_ann == new_ann,
            Operation::InsertColumnsRange { contents, .. } | Operation::DeleteColumnsRange { contents, .. } => contents.is_empty(),
            _ => false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Commit {
    pub description: String,
    pub operations: Vec<Operation>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct VersionController {
    pub undo_stack: Vec<Commit>,
    pub redo_stack: Vec<Commit>,
    #[serde(skip)]
    pub current_commit: Vec<Operation>,
}

impl VersionController {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_commit: Vec::new(),
        }
    }

    pub fn apply(&mut self, doc: &mut TabDocument, op: Operation) {
        if op.is_noop() { return; }
        let inverse_op = doc.invert_operation(&op);
        doc.apply_operation(&op);
        self.current_commit.push(inverse_op);
        self.redo_stack.clear();
    }

    pub fn commit(&mut self, description: &str) {
        if !self.current_commit.is_empty() {
            let commit = std::mem::take(&mut self.current_commit);
            self.undo_stack.push(Commit { description: description.to_string(), operations: commit });
        }
    }

    pub fn undo(&mut self, doc: &mut TabDocument) -> Option<String> {
        if let Some(commit) = self.undo_stack.pop() {
            let mut redo_ops = Vec::new();
            for inverse_op in commit.operations.into_iter().rev() {
                let forward_op = doc.invert_operation(&inverse_op);
                doc.apply_operation(&inverse_op);
                redo_ops.push(forward_op);
            }
            self.redo_stack.push(Commit { description: commit.description.clone(), operations: redo_ops.into_iter().rev().collect() });
            Some(commit.description)
        } else {
            None
        }
    }

    pub fn redo(&mut self, doc: &mut TabDocument) -> Option<String> {
        if let Some(commit) = self.redo_stack.pop() {
            let mut undo_ops = Vec::new();
            for forward_op in commit.operations.into_iter() {
                let inverse_op = doc.invert_operation(&forward_op);
                doc.apply_operation(&forward_op);
                undo_ops.push(inverse_op);
            }
            self.undo_stack.push(Commit { description: commit.description.clone(), operations: undo_ops });
            Some(commit.description)
        } else {
            None
        }
    }
}
