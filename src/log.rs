use termion::{
    self,
    color::{self, Fg},
};

use std::path::PathBuf;

const TEX_LOG_WIDTH: usize = 78;

use crate::{
    parser::{Node, Visitor, TexWarning},
    text::SourceText,
};

/// A warning with a call stach
#[derive(Clone, Debug, PartialEq)]
pub struct TracedTexWarning {
    call_stack: Vec<PathBuf>,
    warning: TexWarning
}

impl ToString for TracedTexWarning {
    fn to_string(&self) -> String {
        let width = match termion::terminal_size() {
            Ok((w, _h)) => w as usize,
            Err(_) => TEX_LOG_WIDTH,
        };
        let title = self.warning.kind.to_string();
        let side_padding = (width - title.len()) / 2 - 1;

        let mut s = format!(
            "{}{} {} {}{}\n{}",
            Fg(color::Yellow),
            "=".repeat(side_padding),
            title,
            "=".repeat(side_padding),
            "=".repeat((width + title.len()) % 2), // Add one extra padding if uneven
            Fg(color::Reset),
        );
        s += self.warning.message.as_str();
        s += "\n\n";
        s += Fg(color::Blue).to_string().as_str();
        for (i, call) in self.call_stack.iter().enumerate() {
            s += &format!("{}{}\n", "  ".repeat(i), call.display());
        }
        s += Fg(color::Reset).to_string().as_str();
        s
    }
}

struct WarningErrorGetter {
    call_stack: Vec<PathBuf>,
    warnings: Vec<TracedTexWarning>,
    errors: Vec<TracedTexWarning>,
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
            self.warnings.push(TracedTexWarning {
                call_stack: self.call_stack.clone(),
                warning: w.clone()
            })
        }
        for e in node.errors() {
            self.warnings.push(TracedTexWarning {
                call_stack: self.call_stack.clone(),
                warning: e.clone(),
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
    pub fn from_path<P>(path: P) -> Self
    where
        P: AsRef<std::path::Path>,
    {
        let source = SourceText::from_file(path).unwrap();
        crate::parser::parse_source(source.clone())
    }

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

    pub fn get_warnings_and_errors(&self) -> (Vec<TracedTexWarning>, Vec<TracedTexWarning>) {
        let mut getter = WarningErrorGetter::new();
        getter.populate(&self.root_node);
        (getter.warnings, getter.errors)
    }

    pub fn print_warnings_and_errors(&self) {
        let (warnings, errors) = self.get_warnings_and_errors();
        for w in warnings {
            println!("\n{}", w.to_string());
        }
        for e in errors {
            println!("\n{}", e.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warnings() {
        let log = Log::from_path("./test/main.log");
        let (ws, es) = log.get_warnings_and_errors();
        dbg!(&ws);
        assert_eq!(es.len(), 0);
        assert_eq!(ws.len(), 34);
    }
}
