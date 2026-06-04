pub trait Tokenizer {
    fn tokenize(&self, input: &str) -> Vec<String>;
}

pub struct SimpleTokenizer;

impl Tokenizer for SimpleTokenizer {
    fn tokenize(&self, input: &str) -> Vec<String> {
        input
            .split_whitespace()
            .map(|token| token.to_lowercase())
            .collect()
    }
}
