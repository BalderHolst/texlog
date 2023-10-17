use std::{fs, path::PathBuf};

use crate::{
    lexer::{self, Token, TokenKind},
    log::Log,
    text::SourceText,
};

fn parse_log_file(file_path: PathBuf) -> Log {
    let text = fs::read_to_string(file_path).unwrap();
    let source = SourceText::new(text);
    parse_source(source)
}

pub fn parse_source(source: SourceText) -> Log {
    let tokens = lexer::tokenize(source.as_str());
    let mut parser = Parser::new(tokens);
    parser.parse(source)
}

#[derive(Clone, Debug, PartialEq)]
pub enum TexWarningKind {
    Font,
    Package(String),
    UnderfullHbox,
    OverfullHbox,
    PdfLatex,
}

impl ToString for TexWarningKind {
    fn to_string(&self) -> String {
        match self {
            TexWarningKind::Font => "Font Warning".to_string(),
            TexWarningKind::Package(p_name) => format!("Package ({}) Warning", p_name),
            TexWarningKind::UnderfullHbox => "Underfull Hbox".to_string(),
            TexWarningKind::OverfullHbox => "Overfull Hbox".to_string(),
            TexWarningKind::PdfLatex => "PdfLaTeX Warning".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TexWarning {
    pub(crate) kind: TexWarningKind,
    pub(crate) message: String,
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

    /// List of warnings in node
    warnings: Vec<TexWarning>,

    /// List of errors in node
    errors: Vec<TexWarning>,
}

impl Node {
    pub fn warnings(&self) -> &Vec<TexWarning> {
        &self.warnings
    }

    pub fn errors(&self) -> &Vec<TexWarning> {
        &self.errors
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
            eprintln!("Warning: Called `consume` but token stream is empty.");
            self.tokens.push(Token {
                kind: TokenKind::Word("".to_string()),
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

    fn consume_warning_message(&mut self) -> String {
        let start_index = self.cursor;
        let end_index;

        let mut paren_level = 0;

        loop {
            let this = &self.current().kind;
            let next = &self.peak(1).kind;

            if this == &TokenKind::LeftParen {
                paren_level += 1;
            }
            if this == &TokenKind::RightParen {
                if paren_level > 0 {
                    paren_level -= 1;
                } else {
                    end_index = self.cursor;
                    break;
                }
            }

            if this == &TokenKind::Newline && next == &TokenKind::Newline {
                end_index = self.cursor;
                self.consume();
                break;
            }

            self.consume();
        }

        let end_index = self.cursor;

        let message: String = self.tokens[start_index..end_index]
            .iter()
            .map(|t| t.to_string())
            .collect();

        message
    }

    fn consume_warning_if_warning(&mut self) -> Option<TexWarning> {
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
                Some(TexWarning {
                    kind: TexWarningKind::PdfLatex,
                    message: self.consume_warning_message(),
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
                Some(TexWarning {
                    kind: TexWarningKind::Font,
                    message: self.consume_warning_message(),
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
                Some(TexWarning {
                    kind: TexWarningKind::OverfullHbox,
                    message: self.consume_warning_message(),
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
                Some(TexWarning {
                    kind: TexWarningKind::UnderfullHbox,
                    message: self.consume_warning_message(),
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
                Some(TexWarning {
                    kind: TexWarningKind::Package(package_name),
                    message: self.consume_warning_message(),
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
        let mut warnings = vec![];
        let mut errors = vec![];

        let mut unclosed_text_parens: usize = 0;

        loop {
            if let Some(warning) = self.consume_warning_if_warning() {
                warnings.push(warning);
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
                            warnings,
                            errors,
                        };
                    }
                }
                TokenKind::Path(p) => {
                    messages += p.as_str().clone();
                    self.consume();
                }
                TokenKind::Word(w) => {
                    messages += w.as_str().clone();
                    self.consume();
                }
                TokenKind::Whitespace(w) => {
                    messages += w.as_str().clone();
                    self.consume();
                }
                TokenKind::ExclamationMark => {
                    messages += "!";
                    self.consume();
                }
                TokenKind::Punctuation(c) => {
                    messages.push(c.clone());
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
                TokenKind::Path(p) => panic!("Log should not start with `path`: {p}"),
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
