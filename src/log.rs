use regex;
use std::path::PathBuf;

use crate::{parser::Node, text::SourceText};

pub struct Log {
    pub(crate) info: String,
    pub(crate) source: SourceText,
    pub(crate) root_node: Node,
}

impl Log {
    /// Returns the call stack at an index in the log file. Returns `None` if the index is outside
    /// root node.
    pub fn trace_at(&self, index: usize) -> Vec<PathBuf> {
        let mut trace = self.trace_from_node(index, &self.root_node);
        trace.reverse();
        trace
    }

    fn trace_from_node(&self, index: usize, root_node: &Node) -> Vec<PathBuf> {
        let file = PathBuf::from(&root_node.file);
        for sub_node in &root_node.calls {
            if sub_node.start_pos <= index && index <= sub_node.end_pos {
                let mut trace = self.trace_from_node(index, sub_node);
                trace.push(file);
                return trace;
            }
        }

        // This is the leaf node
        let mut trace = Vec::with_capacity(20);
        trace.push(file);
        return trace;
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::parse_source;

    use super::*;
}
