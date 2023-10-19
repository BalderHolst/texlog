use termion::{
    self,
    color::{self, Fg},
};

use std::path::PathBuf;

const TEX_LOG_WIDTH: usize = 78;

use crate::{
    parser::{Node, TexDiagnostic, Visitor},
    text::SourceText,
};

/// A diagnostic with a call trace
#[derive(Clone, Debug, PartialEq)]
pub struct TracedTexDiagnostic {
    call_stack: Vec<PathBuf>,
    diagnostic: TexDiagnostic,
}

impl ToString for TracedTexDiagnostic {
    fn to_string(&self) -> String {
        let width = match termion::terminal_size() {
            Ok((w, _h)) => w as usize,
            Err(_) => TEX_LOG_WIDTH,
        };
        let title = self.diagnostic.kind.to_string();
        let side_padding = (width - title.len()) / 2 - 1;

        let title_color = match self.diagnostic.level() {
            crate::parser::DiagnosticLevel::Warning => Fg(color::Yellow).to_string(),
            crate::parser::DiagnosticLevel::Error => Fg(color::Red).to_string(),
        };

        let mut s = format!(
            "{}{} {} {}{}\n{}",
            title_color,
            "=".repeat(side_padding),
            title,
            "=".repeat(side_padding),
            "=".repeat((width + title.len()) % 2), // Add one extra padding if uneven
            Fg(color::Reset),
        );
        s += self.diagnostic.message.as_str();
        s += "\n\n";
        s += Fg(color::Blue).to_string().as_str();
        for (i, call) in self.call_stack.iter().enumerate() {
            s += &format!("{}{}\n", "  ".repeat(i), call.display());
        }
        s += Fg(color::Reset).to_string().as_str();
        s
    }
}

struct DiagnosticGetter {
    call_stack: Vec<PathBuf>,
    diagsnostics: Vec<TracedTexDiagnostic>,
}

impl DiagnosticGetter {
    fn new() -> Self {
        Self {
            call_stack: Vec::new(),
            diagsnostics: Vec::new(),
        }
    }

    fn populate(&mut self, root_node: &Node) {
        self.visit_node(root_node);
    }
}

impl Visitor for DiagnosticGetter {
    fn visit_node(&mut self, node: &Node) {
        self.call_stack.push(PathBuf::from(node.file.clone()));
        for d in node.diagnostics() {
            self.diagsnostics.push(TracedTexDiagnostic {
                call_stack: self.call_stack.clone(),
                diagnostic: d.clone(),
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
        crate::parser::parse_source(source)
    }

    /// Returns the call stack at an index in the log file. Returns `None` if the index is outside
    /// root node.
    pub fn trace_at(&self, index: usize) -> Vec<PathBuf> {
        let mut trace = Self::trace_from_node(index, &self.root_node);
        trace.reverse();
        trace
    }

    fn trace_from_node(index: usize, root_node: &Node) -> Vec<PathBuf> {
        let file = PathBuf::from(&root_node.file);
        for sub_node in &root_node.calls {
            if sub_node.start_pos <= index && index <= sub_node.end_pos {
                let mut trace = Self::trace_from_node(index, sub_node);
                trace.push(file);
                return trace;
            }
        }

        // This is the leaf node
        let mut trace = Vec::with_capacity(20);
        trace.push(file);
        trace
    }

    pub fn get_diagnostics(&self) -> Vec<TracedTexDiagnostic> {
        let mut getter = DiagnosticGetter::new();
        getter.populate(&self.root_node);
        getter.diagsnostics
    }

    pub fn print_diagnostics(&self) {
        let diags = self.get_diagnostics();
        for d in diags {
            println!("\n{}", d.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::parse_source;

    use super::*;

    #[test]
    fn warnings() {
        let log = Log::from_path("./test/main.log");
        let ds = log.get_diagnostics();
        assert_eq!(ds.len(), 34);
    }

    #[test]
    fn errors() {
        let text = r"
(./main.tex

! Too many }'s.
l.6 \date December 2004}


! Undefined control sequence.
l.6 \dtae
{December 2004}

(./some/math/doc.tex
! Missing $ inserted
)

Runaway argument?
{December 2004 \maketitle
! Paragraph ended before \date was complete.
<to be read again>
\par
l.8

! LaTeX Error: File `paralisy.sty' not found.
Type X to quit or <RETURN> to proceed,
or enter new name. (Default extension: sty)
Enter file name:
)
";
        let source = SourceText::new(text.to_string());
        let log = parse_source(source);
        let ds = log.get_diagnostics();
        log.print_diagnostics();
        assert_eq!(ds.len(), 5);
    }
}
