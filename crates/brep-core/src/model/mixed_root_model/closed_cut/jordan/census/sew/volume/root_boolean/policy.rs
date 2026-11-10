use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Selection {
    Empty,
    First,
    Second,
    Both,
}
pub(super) fn reverse(kind: RootRelationKind) -> RootRelationKind {
    match kind {
        RootRelationKind::Disjoint => kind,
        RootRelationKind::FirstInsideSecond => RootRelationKind::SecondInsideFirst,
        RootRelationKind::SecondInsideFirst => RootRelationKind::FirstInsideSecond,
    }
}
pub(super) fn select(kind: RootRelationKind, operation: BooleanMaterialOp) -> Selection {
    use BooleanMaterialOp::*;
    use RootRelationKind::*;
    use Selection::*;
    match (kind, operation) {
        (Disjoint, Union | Xor) => Both,
        (Disjoint, Intersection) => Empty,
        (Disjoint, Difference) => First,
        (FirstInsideSecond, Union) => Second,
        (FirstInsideSecond, Intersection) => First,
        (FirstInsideSecond, Difference) => Empty,
        (FirstInsideSecond, Xor) => Both,
        (SecondInsideFirst, Union) => First,
        (SecondInsideFirst, Intersection) => Second,
        (SecondInsideFirst, Difference | Xor) => Both,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_geometry_matches_boolean_material_for_every_admissible_region() {
        use BooleanMaterialOp::*;
        use RootRelationKind::*;
        for kind in [Disjoint, FirstInsideSecond, SecondInsideFirst] {
            for a in [false, true] {
                for b in [false, true] {
                    let admissible = match kind {
                        Disjoint => !(a && b),
                        FirstInsideSecond => !a || b,
                        SecondInsideFirst => !b || a,
                    };
                    if !admissible {
                        continue;
                    }
                    for op in [Union, Intersection, Difference, Xor] {
                        let selected = match select(kind, op) {
                            Selection::Empty => false,
                            Selection::First => a,
                            Selection::Second => b,
                            Selection::Both => a ^ b,
                        };
                        assert_eq!(
                            selected,
                            op.material(a, b),
                            "kind={kind:?} op={op:?} a={a} b={b}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn operand_reversal_preserves_relation_and_difference_direction() {
        use BooleanMaterialOp::*;
        use RootRelationKind::*;
        use Selection::*;
        assert_eq!(reverse(reverse(FirstInsideSecond)), FirstInsideSecond);
        assert_eq!(reverse(Disjoint), Disjoint);
        assert_eq!(select(reverse(SecondInsideFirst), Difference), Empty);
        assert_eq!(select(reverse(FirstInsideSecond), Difference), Both);
    }
}
