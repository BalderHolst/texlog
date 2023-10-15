use std::{fs, io, path::Path, rc::Rc};

pub struct SourceText {
    text: Rc<String>,
}

impl SourceText {
    pub fn new(text: String) -> Self {
        Self {
            text: Rc::new(text),
        }
    }

    pub fn from_file<P>(path: P) -> Result<Self, io::Error>
    where
        P: AsRef<Path>,
    {
        Ok(Self::new(fs::read_to_string(path)?))
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn row_col(&self, index: usize) -> (usize, usize) {
        let mut row = 1;
        let mut last_newline = 0;
        for (i, c) in self.text[..index].chars().enumerate() {
            match c {
                '\n' => {
                    row += 1;
                    last_newline = i;
                }
                _ => {}
            }
        }
        (row, index - last_newline)
    }
}
