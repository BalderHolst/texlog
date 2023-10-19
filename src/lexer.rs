use std::collections::VecDeque;

#[allow(clippy::upper_case_acronyms)]
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
    EOF, // End of log
}

impl ToString for TokenKind {
    fn to_string(&self) -> String {
        match self {
            TokenKind::LeftParen => "(".to_string(),
            TokenKind::RightParen => "(".to_string(),
            TokenKind::ExclamationMark => "!".to_string(),
            TokenKind::Path(p) => p.to_owned(),
            TokenKind::Word(w) => w.to_owned(),
            TokenKind::Punctuation(p) => String::from_iter(&[*p]),
            TokenKind::Newline => "\n".to_string(),
            TokenKind::Whitespace(w) => w.to_owned(),
            TokenKind::EOF => panic!("EOF should never be converted to string."),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub pos: usize,
}

impl ToString for Token {
    fn to_string(&self) -> String {
        self.kind.to_string()
    }
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
            whitespace.push(*c);
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
            word.push(*c);
            self.consume();
        }
        word
    }

    /// Lex next token
    fn next_token(&mut self) -> Option<Token> {
        if !self.queue.is_empty() {
            return self.queue.pop_front(); // This should always be `Some`
        }

        let pos = self.cursor;
        match *self.current()? {
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
            }
        }
    }

    /// Returns `true` when cursor is at the start of a path
    fn at_path_start(&self) -> bool {
        match self.current().cloned() {
            Some('.') => {
                self.peak(1) == Some(&'/')
            }
            Some('/') => true,
            _ => false,
        }
    }

    /// Consume a path
    fn consume_path(&mut self) -> String {
        let mut chars = vec![];
        while self.at_path_start() {
            chars.push(*self.consume().unwrap());
        }

        loop {
            match self.current() {
                Some(&'(') => break,
                Some(&')') => break,
                Some(&'<') => break,
                Some(&'>') => break,
                Some(&'[') => break,
                Some(&']') => break,
                Some(&'!') => break,
                Some(&'\\') => break,

                // TODO: This is an awful solution
                // Break if any of these strings are next in the path. Of course,
                // this means that paths that include these strings will be cut and
                // reported incorrectly, but i cannot figure out a way to determine
                // if the paths continue on the next line.
                Some(_)
                    if [
                        "\n! ", // Error
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
                        > 0 =>
                {
                    break
                }
                Some(&'\n') if self.peak(1) != Some(&'\n') => {
                    self.consume();
                }

                Some(c) if c.is_whitespace() => break,
                Some(c) => {
                    chars.push(*c);
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
