use crate::il::Il;
use crate::intern::Interner;

/// A whole codebase: many lowered files sharing one interner. `files[i].file ==
/// FileId(i)`.
#[derive(Clone)]
pub struct Corpus {
    pub interner: Interner,
    pub files: Vec<Il>,
}

impl Corpus {
    pub fn new(interner: Interner, files: Vec<Il>) -> Self {
        Corpus { interner, files }
    }

    /// Total node count across all files (handy for diagnostics).
    pub fn node_count(&self) -> usize {
        self.files.iter().map(|f| f.nodes.len()).sum()
    }
}
