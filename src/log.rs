use std::path::PathBuf;

use crate::{
    lexer::TexWarningKind,
    parser::{Node, Visitor},
    text::SourceText,
};

/// A warning with a call stach
#[derive(Clone, Debug, PartialEq)]
pub struct TexWarning {
    call_stack: Vec<PathBuf>,
    kind: TexWarningKind,
    log_pos: usize,
    message: String,
}

struct WarningErrorGetter {
    call_stack: Vec<PathBuf>,
    warnings: Vec<TexWarning>,
    errors: Vec<TexWarning>,
}

impl WarningErrorGetter {
    fn new() -> Self {
        Self {
            call_stack: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn populate(&mut self, root_node: &Node) {
        self.visit_node(root_node);
    }
}

impl Visitor for WarningErrorGetter {
    fn visit_node(&mut self, node: &Node) {
        self.call_stack.push(PathBuf::from(node.file.clone()));
        for w in node.warnings() {
            self.warnings.push(TexWarning {
                call_stack: self.call_stack.clone(),
                kind: w.kind.clone(),
                log_pos: w.log_pos.clone(),
                message: w.message.clone(),
            })
        }
        for e in node.errors() {
            self.warnings.push(TexWarning {
                call_stack: self.call_stack.clone(),
                kind: e.kind.clone(),
                log_pos: e.log_pos.clone(),
                message: e.message.clone(),
            })
        }
        self.do_visit_node(node);
        self.call_stack.pop();
    }
}

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

    fn get_warnings_and_errors(&self) -> (Vec<TexWarning>, Vec<TexWarning>) {
        let mut getter = WarningErrorGetter::new();
        getter.populate(&self.root_node);
        (getter.warnings, getter.errors)
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::parse_source;

    use super::*;

    #[test]
    fn warnings() {
        let source = SourceText::from_file("./test/main.log").unwrap();
        let log = parse_source(source.clone());
        let (ws, es) = log.get_warnings_and_errors();
        dbg!(&ws);
        assert_eq!(ws.len(), 20);
    }

}
