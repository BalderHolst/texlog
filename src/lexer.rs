use std::collections::VecDeque;

#[derive(Debug, PartialEq)]
pub enum TokenKind {
    LeftParen,
    RightParen,
    ExclamationMark,
    Path(String),
    Word(String),
    Punctuation(char),
    Newline,
    Whitespace(String),
    EOF,
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub pos: usize,
}

impl Token {
    pub fn new(kind: TokenKind, pos: usize) -> Self {
        Self { kind, pos }
    }

    pub fn has_kind(&self, kind: &TokenKind) -> bool {
        &self.kind == kind
    }
}

pub fn tokenize(log: &str) -> Vec<Token> {
    let lexer = Lexer::new(log);
    lexer.collect()
}

struct Lexer {
    /// The log characters
    chars: Vec<char>,

    // The index of the current character getting lexed
    cursor: usize,

    queue: VecDeque<Token>,

    placed_eof: bool,
}

impl Lexer {
    /// Create a lexer from a source string
    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            cursor: 0,
            queue: VecDeque::with_capacity(10),
            placed_eof: false,
        }
    }

    /// Get characters with and offset from the cursor
    fn peak(&self, offset: isize) -> Option<&char> {
        let index = self.cursor as isize + offset;

        if index < 0 {
            None
        } else {
            self.chars.get(index as usize)
        }
    }

    /// Get the char at the cursor
    fn current(&self) -> Option<&char> {
        self.chars.get(self.cursor)
    }

    /// Get the char at the cursor, and increment the cursor
    fn consume(&mut self) -> Option<&char> {
        let res = self.chars.get(self.cursor);
        self.cursor += 1;
        res
    }

    fn is_whitespace(c: &char) -> bool {
        c.is_whitespace() && c != &'\n'
    }

    fn consume_whitespace(&mut self) -> String {
        let mut whitespace = String::new();
        while let Some(c) = self.current() {
            if !Self::is_whitespace(c) {
                break;
            }
            whitespace.push(c.clone());
            self.consume();
        }
        whitespace
    }

    fn is_word_char(c: &char) -> bool {
        c.is_alphabetic()
    }

    fn consume_word(&mut self) -> String {
        let mut word = String::new();
        while let Some(c) = self.current() {
            if !Self::is_word_char(c) {
                break;
            }
            word.push(c.clone());
            self.consume();
        }
        word
    }

    // fn consume_warning_if_warning(&mut self) -> Option<TexWarningToken> {
    //     // Only check if new line
    //     if self.peak(-1) != Some(&'\n') {
    //         return None;
    //     }

    //     // pdfTeX warning:
    //     // LaTeX Font Warning:
    //     // Package wrapfig Warning:
    //     // Overfull \hbox
    //     // Underfull \hbox

    //     // Only used in package warnings.
    //     let mut package_name = String::new();

    //     const LOOKAHEAD: usize = 25;
    //     if let Some(next_chars_slice) = self.chars.get(self.cursor..self.cursor + LOOKAHEAD) {
    //         match next_chars_slice.iter().collect::<String>() {
    //             next_chars if next_chars.starts_with("LaTeX Font Warning: ") => {
    //                 Some(self.consume_warning(TexWarningKind::Font))
    //             }
    //             next_chars if next_chars.starts_with("pdfTeX warning: ") => {
    //                 Some(self.consume_warning(TexWarningKind::PdfLatex))
    //             }
    //             next_chars if next_chars.starts_with(r"Overfull \hbox ") => {
    //                 Some(self.consume_warning(TexWarningKind::OverfullHbox))
    //             }
    //             next_chars if next_chars.starts_with(r"Underfull \hbox ") => {
    //                 Some(self.consume_warning(TexWarningKind::UnderfullHbox))
    //             }
    //             next_chars
    //                 if {
    //                     let mut little_lexer = Self::new(next_chars.as_str());
    //                     let w1 = little_lexer.consume_word();
    //                     if w1.as_str() == "Package" {
    //                         little_lexer.consume_whitespace();
    //                         let w2 = little_lexer.consume_word();
    //                         little_lexer.consume_whitespace();
    //                         let w3 = little_lexer.consume_word();
    //                         if w3.as_str() == "Warning" {
    //                             package_name = w2;
    //                             true
    //                         } else {
    //                             false
    //                         }
    //                     } else {
    //                         // If we no not match, be sure to not skip anything.
    //                         false
    //                     }
    //                 } =>
    //             {
    //                 Some(self.consume_warning(TexWarningKind::Package(package_name)))
    //             }
    //             _ => None,
    //         }
    //     } else {
    //         None
    //     }
    // }

    fn consume_warning_text(&mut self) -> String {
        let mut text = String::new();
        let mut paren_level: usize = 0;
        while let Some(c) = self.current() {
            match c {
                '(' => paren_level += 1,
                ')' if paren_level == 0 => break,
                ')' => paren_level -= 1,
                _ => {}
            }

            if (self.current(), self.peak(1)) == (Some(&'\n'), Some(&'\n')) {
                break;
            }
            text.push(c.clone());
            self.consume();
        }
        text
    }

    // fn consume_warning(&mut self, kind: TexWarningKind) -> TexWarningToken {
    //     let log_pos = self.cursor;
    //     let message = self.consume_warning_text();
    //     TexWarningToken {
    //         kind,
    //         log_pos,
    //         message,
    //     }
    // }

    /// Lex next token
    fn next_token(&mut self) -> Option<Token> {
        if !self.queue.is_empty() {
            return self.queue.pop_front(); // This should always be `Some`
        }

        let pos = self.cursor;
        match self.current()?.clone() {
            '(' => {
                self.consume();
                Some(Token::new(TokenKind::LeftParen, pos))
            }
            ')' => {
                self.consume();
                Some(Token::new(TokenKind::RightParen, pos))
            }
            '!' => {
                self.consume();
                Some(Token::new(TokenKind::ExclamationMark, pos))
            }
            '\n' => {
                self.consume();
                Some(Token::new(TokenKind::Newline, pos))
            }
            c if Self::is_word_char(&c) => {
                let word = self.consume_word();
                Some(Token::new(TokenKind::Word(word), pos))
            }
            c if Self::is_whitespace(&c) => {
                let whitespace = self.consume_whitespace();
                Some(Token::new(TokenKind::Whitespace(whitespace), pos))
            }
            _ if self.at_path_start() => {
                let path = self.consume_path();
                Some(Token::new(TokenKind::Path(path), pos))
            }
            c => {
                self.consume();
                Some(Token::new(TokenKind::Punctuation(c), pos))
            },
        }
    }

    /// Returns `true` when cursor is at the start of a path
    fn at_path_start(&self) -> bool {
        match self.current().cloned() {
            Some('.') => {
                if self.peak(1) == Some(&'/') {
                    true
                } else {
                    false
                }
            }
            Some('/') => true,
            _ => false,
        }
    }

    /// Consume a path
    fn consume_path(&mut self) -> String {
        let mut chars = vec![];
        while self.at_path_start() {
            chars.push(self.consume().unwrap().clone());
        }

        loop {
            match self.current() {
                Some(&'(') => break,
                Some(&')') => break,
                Some(&'<') => break,
                Some(&'>') => break,
                Some(&'[') => break,
                Some(&']') => break,
                Some(&'\\') => break,

                // TODO: This is an awful solution
                // Break if any of these strings are next in the path. Of course,
                // this means that paths that include these strings will be cut and
                // reported incorrectly, but i cannot figure out a way to determine
                // if the paths continue on the next line.
                Some(_)
                    if &[
                        "\nDictionary:",
                        "\nPackage:",
                        "\nFile:",
                        "\nLaTeX",
                        "\nDocument Class:",
                    ]
                    .map(|s| {
                        self.chars[self.cursor..]
                            .starts_with(s.chars().collect::<Vec<char>>().as_slice())
                    })
                    .iter()
                    .filter(|e| **e)
                    .count()
                        > &0 =>
                {
                    break
                }
                Some(&'\n') if self.peak(1) != Some(&'\n') => {
                    self.consume();
                }

                Some(c) if c.is_whitespace() => break,
                Some(c) => {
                    chars.push(c.clone());
                    self.consume();
                }
                None => break,
            }
        }
        String::from_iter(chars)
    }
}

impl Iterator for Lexer {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_token() {
            Some(t) => Some(t),
            None if !self.placed_eof => {
                self.placed_eof = true;
                Some(Token {
                    kind: TokenKind::EOF,
                    pos: self.cursor,
                })
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::*;

    #[test]
    fn lex_parens() {
        let s = "((())())";
        let lexed_tokens = tokenize(s);
        assert_eq!(
            lexed_tokens,
            vec![
                Token {
                    kind: TokenKind::LeftParen,
                    pos: 0,
                },
                Token {
                    kind: TokenKind::LeftParen,
                    pos: 1,
                },
                Token {
                    kind: TokenKind::LeftParen,
                    pos: 2,
                },
                Token {
                    kind: TokenKind::RightParen,
                    pos: 3,
                },
                Token {
                    kind: TokenKind::RightParen,
                    pos: 4,
                },
                Token {
                    kind: TokenKind::LeftParen,
                    pos: 5,
                },
                Token {
                    kind: TokenKind::RightParen,
                    pos: 6,
                },
                Token {
                    kind: TokenKind::RightParen,
                    pos: 7,
                },
                Token {
                    kind: TokenKind::EOF,
                    pos: 8,
                },
            ]
        )
    }

    #[test]
    fn lex_path() {
        let s = "(./path/to/interesting/place.awesome)";
        let lexed_tokens = tokenize(s);
        assert_eq!(
            lexed_tokens,
            vec![
                Token {
                    kind: TokenKind::LeftParen,
                    pos: 0,
                },
                Token {
                    kind: TokenKind::Path("./path/to/interesting/place.awesome".to_string()),
                    pos: 1,
                },
                Token {
                    kind: TokenKind::RightParen,
                    pos: 36,
                },
                Token {
                    kind: TokenKind::EOF,
                    pos: 37,
                },
            ]
        )
    }
}
