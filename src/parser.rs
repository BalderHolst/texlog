use crate::{
    lexer::{self, Token, TokenKind},
    log::Log,
    text::SourceText,
};

pub fn parse_source(source: SourceText) -> Log {
    let tokens = lexer::tokenize(source.as_str());
    let mut parser = Parser::new(tokens);
    parser.parse(source)
}

#[derive(Clone, Debug, PartialEq)]
pub enum DiagnosticLevel {
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TexDiagnosticKind {
    Font,
    Package(String),
    UnderfullHbox,
    OverfullHbox,
    PdfLatex,
    GenericError(String),
}

impl TexDiagnosticKind {
    pub fn level(&self) -> DiagnosticLevel {
        match self {
            TexDiagnosticKind::Font => DiagnosticLevel::Warning,
            TexDiagnosticKind::Package(_) => DiagnosticLevel::Warning,
            TexDiagnosticKind::UnderfullHbox => DiagnosticLevel::Warning,
            TexDiagnosticKind::OverfullHbox => DiagnosticLevel::Warning,
            TexDiagnosticKind::PdfLatex => DiagnosticLevel::Warning,
            TexDiagnosticKind::GenericError(_) => DiagnosticLevel::Error,
        }
    }
}

impl ToString for TexDiagnosticKind {
    fn to_string(&self) -> String {
        match self {
            TexDiagnosticKind::Font => "Font Warning".to_string(),
            TexDiagnosticKind::Package(p_name) => format!("Package ({}) Warning", p_name),
            TexDiagnosticKind::UnderfullHbox => "Underfull Hbox".to_string(),
            TexDiagnosticKind::OverfullHbox => "Overfull Hbox".to_string(),
            TexDiagnosticKind::PdfLatex => "PdfLaTeX Warning".to_string(),
            TexDiagnosticKind::GenericError(e) => format!("Error: {}", e),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TexDiagnostic {
    pub(crate) kind: TexDiagnosticKind,
    pub(crate) message: String,
}

impl TexDiagnostic {
    pub fn level(&self) -> DiagnosticLevel {
        self.kind.level()
    }
}

#[derive(Debug)]
pub(crate) struct Node {
    /// Path to file that this node represents
    pub(crate) file: String,

    /// Log messages for this node
    pub(crate) messages: String,

    /// Position of the node in log file
    pub(crate) start_pos: usize,
    pub(crate) end_pos: usize,

    /// The other files that this one calls
    pub(crate) calls: Vec<Node>,

    /// List of diagnostics in node
    diagnostics: Vec<TexDiagnostic>,
}

impl Node {
    pub fn diagnostics(&self) -> &Vec<TexDiagnostic> {
        &self.diagnostics
    }

    pub fn warnings(&self) -> Vec<&TexDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.level() == DiagnosticLevel::Warning)
            .collect()
    }

    pub fn errors(&self) -> Vec<&TexDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.level() == DiagnosticLevel::Error)
            .collect()
    }
}

pub struct Parser {
    cursor: usize,
    tokens: Vec<Token>,
}

impl Parser {
    /// Create a new parser from a vec of tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    fn peak(&self, offset: isize) -> &Token {
        let index = self.cursor as isize + offset;
        self.tokens
            .get(index.clamp(0, self.tokens.len() as isize - 1) as usize)
            .expect("Index should be clamped to a valid index.")
    }

    /// Get token under cursor
    fn current(&self) -> &Token {
        self.tokens
            .get(self.cursor.clamp(0, self.tokens.len() - 1))
            .expect("Index should be clamped to a valid index.")
    }

    /// Get token under cursor and increment cursor
    fn consume(&mut self) -> &Token {
        if self.tokens.is_empty() {
            if cfg!(Debug) {
                eprintln!("Warning: Called `consume` but token stream is empty.");
            }
            self.tokens.push(Token {
                kind: TokenKind::EOF,
                pos: 0,
            });
            return self.tokens.last().unwrap();
        }

        let token = match self.tokens.get(self.cursor) {
            Some(t) => t,
            None => {
                println!("Warning: Tried to consume at the end of token stream.");
                debug_assert!(false, "Please fix this.");
                self.cursor -= 1;
                self.tokens
                    .last()
                    .expect("This is handled by the if statement before this match")
            }
        };
        self.cursor += 1;
        token
    }

    fn consume_diagnostic_message(&mut self) -> String {
        let start_index = self.cursor;

        let mut paren_level = 0;

        loop {
            let this = &self.current().kind;
            let next = &self.peak(1).kind;
            match this {
                TokenKind::LeftParen => paren_level += 1,
                TokenKind::RightParen => {
                    if paren_level > 0 {
                        paren_level -= 1;
                    } else {
                        break;
                    }
                },
                TokenKind::Newline if next == &TokenKind::Newline => {
                    self.consume();
                    break;
                },
                _ => {},
            }
            self.consume();
        }

        let end_index = self.cursor;

        let message: String = self.tokens[start_index..end_index]
            .iter()
            .map(|t| t.to_string())
            .collect();

        message.trim().to_string()
    }

    fn consume_diag_if_diag(&mut self) -> Option<TexDiagnostic> {
        // Must be at newline
        if self.peak(-1).kind != TokenKind::Newline {
            return None;
        }

        match &self.current().kind {
            // pdfTeX warning:
            TokenKind::Word(w) if w.as_str() == "pdfTeX" => {
                if self.peak(2).kind != TokenKind::Word("warning".to_string()) {
                    return None;
                }
                if self.peak(3).kind != TokenKind::Punctuation(':') {
                    return None;
                }
                Some(TexDiagnostic {
                    kind: TexDiagnosticKind::PdfLatex,
                    message: self.consume_diagnostic_message(),
                })
            }

            // LaTeX Font Warning:
            TokenKind::Word(w) if w.as_str() == "LaTeX" => {
                if self.peak(2).kind != TokenKind::Word("Font".to_string()) {
                    return None;
                }
                if self.peak(4).kind != TokenKind::Word("Warning".to_string()) {
                    return None;
                }
                if self.peak(5).kind != TokenKind::Punctuation(':') {
                    return None;
                }
                Some(TexDiagnostic {
                    kind: TexDiagnosticKind::Font,
                    message: self.consume_diagnostic_message(),
                })
            }

            // Overfull \hbox
            TokenKind::Word(w) if w.as_str() == "Overfull" => {
                if self.peak(2).kind != TokenKind::Punctuation('\\') {
                    return None;
                }
                if self.peak(3).kind != TokenKind::Word("hbox".to_string()) {
                    return None;
                }
                Some(TexDiagnostic {
                    kind: TexDiagnosticKind::OverfullHbox,
                    message: self.consume_diagnostic_message(),
                })
            }

            // Underfull \hbox
            TokenKind::Word(w) if w.as_str() == "Underfull" => {
                if self.peak(2).kind != TokenKind::Punctuation('\\') {
                    return None;
                }
                if self.peak(3).kind != TokenKind::Word("hbox".to_string()) {
                    return None;
                }
                Some(TexDiagnostic {
                    kind: TexDiagnosticKind::UnderfullHbox,
                    message: self.consume_diagnostic_message(),
                })
            }

            // Package wrapfig Warning:
            TokenKind::Word(w) if w.as_str() == "Package" => {
                let package_name;
                if let TokenKind::Word(name) = &self.peak(2).kind {
                    package_name = name.clone();
                } else {
                    return None;
                }
                if self.peak(4).kind != TokenKind::Word("Warning".to_string()) {
                    return None;
                }
                if self.peak(5).kind != TokenKind::Punctuation(':') {
                    return None;
                }
                Some(TexDiagnostic {
                    kind: TexDiagnosticKind::Package(package_name),
                    message: self.consume_diagnostic_message(),
                })
            }

            // GenericError
            TokenKind::ExclamationMark => {
                let err_start = self.cursor;

                assert_eq!(self.consume().kind, TokenKind::ExclamationMark);

                // Get error title
                loop {
                    match &self.current().kind {
                        TokenKind::Newline => break,
                        TokenKind::EOF => break,
                        _ => {}
                    }
                    self.consume();
                }
                let title: String = self.tokens[err_start + 2..self.cursor]
                    .iter()
                    .map(|t| t.to_string())
                    .collect();

                // Reset cursor to get full diagnostic
                self.cursor = err_start;

                // Look back for start of error message
                loop {
                    match &self.peak(-1).kind {
                        TokenKind::Newline if self.peak(-2).kind == TokenKind::Newline => break,
                        TokenKind::EOF => break,
                        TokenKind::Path(_) => break,
                        _ => self.cursor -= 1,
                    }
                }

                Some(TexDiagnostic {
                    kind: TexDiagnosticKind::GenericError(title),
                    message: self.consume_diagnostic_message(),
                })
            }

            _ => None,
        }
    }

    fn parse_node(&mut self) -> Node {
        let pos = self.current().pos;

        assert!(self.consume().has_kind(&TokenKind::LeftParen));

        let file = match &self.current().kind {
            TokenKind::Path(p) => p.clone(),
            _ => "no path...".to_string(),
        };

        let mut messages: String = "".to_string();
        let mut calls = vec![];
        let mut diagnostics = vec![];

        let mut unclosed_text_parens: usize = 0;

        loop {
            if let Some(diag) = self.consume_diag_if_diag() {
                diagnostics.push(diag);
            }

            match &self.current().kind {
                TokenKind::LeftParen => {
                    if let TokenKind::Path(_) = self.peak(1).kind {
                        calls.push(self.parse_node())
                    } else {
                        messages += "(";
                        unclosed_text_parens += 1;
                        self.consume();
                    }
                }
                TokenKind::RightParen => {
                    if unclosed_text_parens > 0 {
                        messages += ")";
                        unclosed_text_parens -= 1;
                        self.consume();
                    } else {
                        let end_token = self.consume();
                        return Node {
                            file,
                            messages,
                            start_pos: pos,
                            end_pos: end_token.pos,
                            calls,
                            diagnostics,
                        };
                    }
                }
                TokenKind::Path(p) => {
                    messages += p.as_str();
                    self.consume();
                }
                TokenKind::Word(w) => {
                    messages += w.as_str();
                    self.consume();
                }
                TokenKind::Whitespace(w) => {
                    messages += w.as_str();
                    self.consume();
                }
                TokenKind::ExclamationMark => {
                    messages += "!";
                    self.consume();
                }
                TokenKind::Punctuation(c) => {
                    messages.push(*c);
                    self.consume();
                }
                TokenKind::Newline => {
                    messages += "\n";
                    self.consume();
                }
                TokenKind::EOF => panic!("EOF in the middle of {}", file),
            }
        }
    }

    /// Parse source text to `Log`
    pub fn parse(&mut self, source: SourceText) -> Log {
        let mut info = "".to_string();
        loop {
            match &self.current().kind {
                TokenKind::LeftParen => {
                    if let TokenKind::Path(_) = self.peak(1).kind {
                        break;
                    } else {
                        info += "(";
                    }
                }
                TokenKind::Path(p) => info += p.to_string().as_str(),
                TokenKind::EOF => todo!(),
                kind => info += kind.to_string().as_str(),
            }
            self.consume();
        }
        let root_node = self.parse_node();
        Log {
            info,
            root_node,
            source,
        }
    }
}

pub(crate) trait Visitor {
    fn visit_node(&mut self, node: &Node) {
        self.do_visit_node(node)
    }

    fn do_visit_node(&mut self, node: &Node) {
        for other_node in &node.calls {
            self.visit_node(other_node)
        }
    }
}

pub struct Printer {
    /// Debth in tree
    level: usize,

    /// Source text
    text: SourceText,
}

impl Printer {
    pub fn new(text: SourceText) -> Self {
        Self { text, level: 0 }
    }
}

impl Visitor for Printer {
    fn visit_node(&mut self, node: &Node) {
        println!(
            "{}{:?} at {:?} - {:?}",
            "  ".repeat(self.level),
            node.file,
            self.text.row_col(node.start_pos),
            self.text.row_col(node.end_pos),
        );
        self.level += 1;
        self.do_visit_node(node);
        self.level -= 1;
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn trace() {
        let source = SourceText::from_file("./test/main.log").unwrap();
        let log = parse_source(source.clone());
        let trace = log.trace_at(source.index(7, 1));
        dbg!(&trace);
        assert_eq!(trace, vec![PathBuf::from("./main.tex")])
    }
}
