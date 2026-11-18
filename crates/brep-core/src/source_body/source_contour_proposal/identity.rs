use super::*;
impl SourceRegion {
    /// Literal original construction, excluding cached qualification results.
    pub fn same_definition(&self, other: &Self) -> bool {
        same(&self.recipe, &other.recipe)
    }
}
fn original(a: &Original, b: &Original) -> bool {
    a.context == b.context
        && a.surface == b.surface
        && a.tolerance == b.tolerance
        && a.wires.len() == b.wires.len()
        && a.wires.iter().zip(&b.wires).all(|(a, b)| {
            a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| {
                    a.curve == b.curve && a.pcurve == b.pcurve && a.reversed == b.reversed
                })
        })
}
fn same(a: &Recipe, b: &Recipe) -> bool {
    match (a, b) {
        (Recipe::Original(a), Recipe::Original(b)) => original(a, b),
        (
            Recipe::SplitParameter {
                parent: a,
                address: aa,
                parameter: pa,
            },
            Recipe::SplitParameter {
                parent: b,
                address: ab,
                parameter: pb,
            },
        ) => aa == ab && pa == pb && same(a, b),
        (
            Recipe::SplitRoot {
                parent: a,
                address: aa,
                point: pa,
                role: ra,
            },
            Recipe::SplitRoot {
                parent: b,
                address: ab,
                point: pb,
                role: rb,
            },
        ) => aa == ab && ra == rb && pa.same_definition(pb) && same(a, b),
        (
            Recipe::Cut {
                original: a,
                kind: ka,
                contact: ca,
                address: aa,
                target_width: wa,
                axis: xa,
            },
            Recipe::Cut {
                original: b,
                kind: kb,
                contact: cb,
                address: ab,
                target_width: wb,
                axis: xb,
            },
        ) => {
            matches!(
                (ka, kb),
                (CutKind::Linear, CutKind::Linear) | (CutKind::Curved, CutKind::Curved)
            ) && original(a, b)
                && ca == cb
                && aa == ab
                && wa == wb
                && xa == xb
        }
        _ => false,
    }
}
