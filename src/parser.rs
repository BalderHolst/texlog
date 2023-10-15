use std::{fs, path::PathBuf};

use crate::{
    lexer::{self, Token, TokenKind},
    text::SourceText,
};

fn parse_log_file(file_path: PathBuf) -> Log {
    let text = fs::read_to_string(file_path).unwrap();
    parse_source(&text)
}

fn parse_source(source: &str) -> Log {
    let tokens = lexer::tokenize(source);
    let mut parser = Parser::new(tokens);
    parser.parse()
}

#[derive(Debug)]
struct Node {
    /// Path to file that this node represents
    file: String,

    /// Log messages for this node
    messages: String,

    /// Position of the node in log file
    start_pos: usize,
    end_pos: usize,

    /// The other files that this one calls
    calls: Vec<Node>,
}

pub struct Parser {
    cursor: usize,
    tokens: Vec<Token>,
}

impl Parser {
    /// Create a new parser from a vec of tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { cursor: 0, tokens }
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
                kind: TokenKind::Message("".to_string()),
                pos: 0,
            });
            return self.tokens.last().unwrap();
        }

        let token = match self.tokens.get(self.cursor) {
            Some(t) => t,
            None => {
                println!("Warning: Tried to consume at the end of token stream.");
                self.cursor -= 1;
                self.tokens
                    .last()
                    .expect("This is handled by the if statement before this match")
            }
        };
        println!("{:?}", token);
        self.cursor += 1;
        token
    }

    fn parse_node(&mut self) -> Node {
        let pos = self.current().pos;

        assert!(self.consume().has_kind(&TokenKind::LeftParen));

        let file = match &self.current().kind {
            TokenKind::Path(p) => p.clone(),
            _ => "no path...".to_string(),
        };

        println!("Starting node {file}");

        let mut messages: String = "".to_string();
        let mut calls = vec![];

        let mut unclosed_text_parens: usize = 0;

        loop {
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
                        };
                    }
                }
                TokenKind::Path(p) => {
                    messages += p.as_str().clone();
                    self.consume();
                }
                TokenKind::Message(m) => {
                    messages += m.as_str().clone();
                    self.consume();
                }
            }
        }
    }

    pub fn parse(&mut self) -> Log {
        let mut info = "".to_string();
        println!("info...");
        loop {
            match &self.current().kind {
                TokenKind::LeftParen => {
                    if let TokenKind::Path(_) = self.peak(1).kind {
                        break;
                    } else {
                        info += "(";
                        self.consume();
                    }
                }
                TokenKind::RightParen => {
                    info += ")";
                    self.consume();
                }
                TokenKind::Path(p) => panic!("Log should not start with `path`: {p}"),
                TokenKind::Message(m) => {
                    info += m.as_str();
                    self.consume();
                }
            }
        }
        println!("Beyond info!");
        let root_node = self.parse_node();
        Log { info, root_node }
    }
}

pub struct Log {
    info: String,
    root_node: Node,
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
                return trace
            }
        }

        // This is the leaf node
        let mut trace = Vec::with_capacity(20);
        trace.push(file);
        return trace
    }
}

trait Visitor {
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

    // #[test]
    // fn print_tree() {
    //     let source = SourceText::from_file("./test/main.log").unwrap();
    //     let log = parse_source(source.as_str());
    //     let mut printer = Printer::new(source);
    //     printer.visit_node(&log.root_node);
    // }

    #[test]
    fn trace() {
        let source = SourceText::from_file("./test/main.log").unwrap();
        let log = parse_source(source.as_str());
        let trace = log.trace_at(1500);
        dbg!(trace);
    }

}
