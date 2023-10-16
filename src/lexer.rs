#[derive(Debug, PartialEq)]
pub enum TokenKind {
    LeftParen,
    RightParen,
    Path(String),
    Message(String),
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
}

impl Lexer {
    /// Create a lexer from a source string
    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            cursor: 0,
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

    /// Lex next token
    fn next_token(&mut self) -> Option<Token> {
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
                let end_index;

                // Stop at ')' or end of text.
                loop {
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
        self.next_token()
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
            ]
        )
    }
}
