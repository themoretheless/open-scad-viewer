use super::*;

pub struct Parser<'a> {
    pub(super) tokens: &'a [Token],
    pub(super) pos: usize,
    pub(super) nodes: usize,
    pub(super) expression_depth: usize,
    pub(super) statement_depth: usize,
    pub(super) last_token_end: usize,
    pub(super) profile: LanguageProfile,
}
