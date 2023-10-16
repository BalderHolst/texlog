use std::collections::VecDeque;

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
            TexWarningKind::Package(p_name) => format!("Package Warning ({})", p_name),
            TexWarningKind::UnderfullHbox => "Underfull Hbox".to_string(),
            TexWarningKind::OverfullHbox => "Overfull Hbox".to_string(),
            TexWarningKind::PdfLatex => "PdfLaTeX Warning".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TexWarningToken {
    pub(crate) kind: TexWarningKind,
    pub(crate) log_pos: usize,
    pub(crate) message: String,
}

#[derive(Debug, PartialEq)]
pub enum TokenKind {
    LeftParen,
    RightParen,
    Path(String),
    Message(String),
    Warning(TexWarningToken),
    EOF,
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub pos: usize,
}

impl Token {
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

    fn consume_warning_if_warning(&mut self) -> Option<TexWarningToken> {
        // pdfTeX warning:
        // LaTeX Font Warning:
        const LOOKAHEAD: usize = 20;
        if let Some(next_chars_slice) = self.chars.get(self.cursor..self.cursor + LOOKAHEAD) {
            match next_chars_slice.iter().collect::<String>() {
                next_chars if next_chars.starts_with("LaTeX Font Warning: ") => {
                    return Some(self.consume_font_warning())
                }
                next_chars if next_chars.starts_with("pdfTeX warning: ") => {
                    return Some(self.consume_pdftex_warning());
                }
                _ => {}
            }
        }
        None
    }

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

    fn consume_font_warning(&mut self) -> TexWarningToken {
        let log_pos = self.cursor;
        let message = self.consume_warning_text();
        TexWarningToken {
            kind: TexWarningKind::Font,
            log_pos,
            message,
        }
    }

    fn consume_pdftex_warning(&mut self) -> TexWarningToken {
        let log_pos = self.cursor;
        let message = self.consume_warning_text();
        TexWarningToken {
            kind: TexWarningKind::PdfLatex,
            log_pos,
            message,
        }
    }

    /// Lex next token
    fn next_token(&mut self) -> Option<Token> {
        if !self.queue.is_empty() {
            return self.queue.pop_front(); // This should always be `Some`
        }

        let pos = self.cursor;
        match self.current()? {
            '(' => {
                self.consume();
                Some(Token {
                    kind: TokenKind::LeftParen,
                    pos,
                })
            }
            ')' => {
                self.consume();
                Some(Token {
                    kind: TokenKind::RightParen,
                    pos,
                })
            }
            _ if self.at_path_start() => {
                // Check of we need to lex a path
                let path = self.consume_path();
                Some(Token {
                    kind: TokenKind::Path(path),
                    pos,
                })
            }
            _ => {
                let start_index = pos;
                let mut end_index = pos;

                // Stop at ')' or end of text.
                loop {
                    if let Some(warning) = self.consume_warning_if_warning() {
                        // Push warning to queue to be returned next
                        self.queue.push_back(Token {
                            kind: TokenKind::Warning(warning),
                            pos,
                        });

                        // Return the current message
                        let bytes = self.chars[start_index..end_index].iter();
                        let message = String::from_iter(bytes);
                        return Some(Token {
                            kind: TokenKind::Message(message),
                            pos,
                        });
                    }

                    match self.current() {
                        Some(c) => {
                            if matches!(c, &')' | &'(') || self.at_path_start() {
                                end_index = self.cursor;
                                break;
                            }
                        }
                        None => {
                            // End of text
                            end_index = self.cursor;
                            break;
                        }
                    }
                    self.consume();
                }
                let bytes = self.chars[start_index..end_index].iter();
                let message = String::from_iter(bytes);
                Some(Token {
                    kind: TokenKind::Message(message),
                    pos,
                })
            }
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
                Some(Token { kind: TokenKind::EOF, pos: self.cursor })
            },
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
