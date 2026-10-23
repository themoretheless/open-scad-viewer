use super::*;

/// `Record<string, V>` semantics: last write wins, first insertion order kept.
pub(super) fn record_insert<V>(vec: &mut Vec<(String, V)>, key: String, value: V) {
    if let Some(entry) = vec.iter_mut().find(|(k, _)| *k == key) {
        entry.1 = value;
    } else {
        vec.push((key, value));
    }
}

pub(super) fn expr_position(expr: &Expr) -> usize {
    match expr {
        Expr::Literal { p, .. }
        | Expr::Identifier { p, .. }
        | Expr::Vector { p, .. }
        | Expr::Range { p, .. }
        | Expr::Unary { p, .. }
        | Expr::Binary { p, .. }
        | Expr::Ternary { p, .. }
        | Expr::Function { p, .. }
        | Expr::Call { p, .. }
        | Expr::Index { p, .. }
        | Expr::Member { p, .. }
        | Expr::Let { p, .. }
        | Expr::Assert { p, .. }
        | Expr::Echo { p, .. }
        | Expr::LcFor { p, .. }
        | Expr::LcForC { p, .. }
        | Expr::LcIf { p, .. }
        | Expr::LcLet { p, .. }
        | Expr::LcEach { p, .. } => *p,
    }
}
