//! Parsed G-code words and blocks. Classification lives in `command`.

#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    pub letter: char,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub line: usize,
    pub n: Option<u32>,
    pub words: Vec<Word>,
    pub comment: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Program {
    pub blocks: Vec<Block>,
}
