use std::{fs, path::PathBuf};

use crate::lexer::{self, Token, TokenKind};

fn parse_log_file(file_path: PathBuf) -> Log {
    let text = fs::read_to_string(file_path).unwrap();
    parse_source(&text)
}

fn parse_source(source: &String) -> Log {
    let tokens = lexer::tokenize(source.as_str());
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

impl Node {
    pub fn row_col(&self, source: &String) -> (usize, usize) {
        let mut row = 1;
        let mut last_newline = 0;
        for (i, c) in source[..self.start_pos].chars().enumerate() {
            match c {
                '\n' => {
                    row += 1;
                    last_newline = i;
                }
                _ => {}
            }
        }
        (row, self.start_pos - last_newline)
    }
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

    fn parse(&mut self) -> Log {
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

    /// Source text
    text: String,
}

impl Printer {
    fn new(text: String) -> Self {
        Self { text, level: 0 }
    }
}

impl Visitor for Printer {
    fn visit_node(&mut self, node: &Node) {
        println!(
            "{}{:?} at {:?} (len: {})",
            "  ".repeat(self.level),
            node.file,
            node.row_col(&self.text),
            node.messages.len()
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
    fn print_tree() {
        let source = fs::read_to_string("./test/main.log").unwrap();
        let log = parse_source(&source);
        let mut printer = Printer::new(source);
        printer.visit_node(&log.root_node);
    }
}
