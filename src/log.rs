use regex;
use std::path::PathBuf;

use crate::{parser::Node, text::SourceText};

#[derive(Debug)]
pub enum TexWarningKind {
    Font,
    Package,
    UnderfullHbox,
    OverfullHbox,
    PdfLatex,
}

#[derive(Debug)]
pub struct TexWarning {
    kind: TexWarningKind,
    log_pos: usize,
    message: String,
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

    pub fn get_warnings(&self) -> Vec<TexWarning> {
        let text = self.source.text();

        let mut warnings = vec![];

        // Regexes
        let pdflatex_regex = regex::Regex::new(r"pdfTeX warning: (.+)\n").unwrap();

        let mut locs = pdflatex_regex.capture_locations();
        pdflatex_regex.captures_read(&mut locs, text.as_str()).into_iter().for_each(|loc| {
            warnings.push(TexWarning {
                kind: TexWarningKind::PdfLatex,
                log_pos: loc.start(),
                message: loc.as_str().to_string(),
            })
        });

        dbg!(&warnings);

        return warnings
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
        let warnings = log.get_warnings();
        dbg!(&warnings);
        assert_eq!(warnings.len(), 20);
    }
}
