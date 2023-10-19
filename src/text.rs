use std::{fs, io, path::Path, rc::Rc};

#[derive(Clone)]
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

    pub fn text(&self) -> Rc<String> {
        self.text.clone()
    }

    pub fn row_col(&self, index: usize) -> (usize, usize) {
        let mut row = 1;
        let mut last_line_start = 0;
        for (i, c) in self.text[..index].chars().enumerate() {
            if c == '\n'  {
                row += 1;
                last_line_start = i + 1;
            }
        }
        (row, index - last_line_start + 1)
    }

    pub fn index(&self, row: usize, col: usize) -> usize {
        let row = usize::max(1, row);
        let col = usize::max(1, col);

        let mut index = 0;
        let mut chars = self.text.chars();

        // Find row
        for _ in 1..row {
            for c in &mut chars {
                index += 1;
                if c == '\n' {
                    break;
                }
            }
        }

        // Add col
        index += col - 1;

        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_col() {
        let source = SourceText::from_file("./test/main.log").unwrap();
        for input_index in &[0, 1, 120, 121, 300, 600, 700, 900, 1100, 1500] {
            let (row, col) = source.row_col(*input_index);
            let output_index = source.index(row, col);
            println!("{} -> {:?} -> {}", input_index, (&row, &col), output_index);
            assert_eq!(*input_index, output_index)
        }
    }
}
