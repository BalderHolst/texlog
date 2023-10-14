use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::lexer::{self, Token, TokenKind};

fn parse_log_file(file_path: PathBuf) -> Log {
    let text = fs::read_to_string(file_path).unwrap();
    let tokens = lexer::tokenize(text.as_str());
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
    pos: Option<usize>,

    /// The other files that this one calls
    calls: Vec<Node>,
}

struct Parser {
    cursor: usize,
    tokens: Vec<Token>,
}

impl Parser {
    /// Create a new parser from a vec of tokens
    fn new(tokens: Vec<Token>) -> Self {
        Self { cursor: 0, tokens }
    }

    fn peak(&self, offset: isize) -> &Token {
        let index = self.cursor as isize + offset;
        if index < 0 {
            panic!("Peaked out of range...")
        }
        else {
            self.tokens.get(index as usize).unwrap()
        }
    }

    /// Get token under cursor
    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    /// Get token under cursor and increment cursor
    fn consume(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.cursor);
        self.cursor += 1;
        token
    }

    fn parse_node(&mut self) -> Node {
        assert!(self
            .consume()
            .expect("There should always be a token here to call this function.")
            .has_kind(&TokenKind::LeftParen));

        let file = match &self.current().unwrap().kind {
            TokenKind::Path(p) => p.clone(),
            _ => "no path...".to_string()
        };

        let pos = None;

        let mut messages: String = "".to_string();
        let mut calls = vec![];

        loop {
            match &self.current().unwrap().kind {
                TokenKind::LeftParen => calls.push(self.parse_node()),
                TokenKind::RightParen => {
                    return Node {
                        file,
                        messages,
                        pos,
                        calls,
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

    fn parse(&mut self) -> Log {
        let mut info = "".to_string();
        println!("info...");
        loop {
            match self.current() {
                Some(token) => match &token.kind {
                    TokenKind::LeftParen => {
                        if let TokenKind::Path(_) = self.peak(1).kind {
                            break
                        }
                        else {
                            info += "(";
                            self.consume();
                        }
                    },
                    TokenKind::RightParen => {
                        info += ")";
                        self.consume();
                    },
                    TokenKind::Path(p) => panic!("Log should not start with `path`: {p}"),
                    TokenKind::Message(m) => {
                        info += m.as_str();
                        self.consume();
                    },
                },
                None => todo!("No root node"),
            }
        }
        println!("Beyond info!");
        let root_node = self.parse_node();
        Log { info, root_node }
    }
}

struct Log {
    info: String,
    root_node: Node,
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

struct Printer {
    /// Debth in tree
    level: usize,
}

impl Printer {
    fn new() -> Self {
        Self { level: 0 }
    }
}

impl Visitor for Printer {
    fn visit_node(&mut self, node: &Node) {
        println!("{}{:?}", "  ".repeat(self.level), node.file);
        self.level += 1;
        self.do_visit_node(node);
        self.level -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_tree() {
        let log = parse_log_file(PathBuf::from("./test/main.log"));
        let mut printer = Printer::new();
        printer.visit_node(&log.root_node);
    }
}
