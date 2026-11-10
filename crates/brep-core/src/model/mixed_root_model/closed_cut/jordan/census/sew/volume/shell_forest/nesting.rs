use super::*;
pub(super) fn qualify(
    roots: &[Arc<QualifiedPairedRootSolid>],
    relations: &[Arc<QualifiedRootRelation>],
) -> Result<(Vec<Option<usize>>, Vec<usize>)> {
    let n = roots.len();
    if relations.len() != n * (n - 1) / 2 {
        return Err(unproved(
            "Root shell forest requires every unordered source pair",
        ));
    }
    let mut seen = vec![vec![false; n]; n];
    let mut inside = seen.clone();
    for proof in relations {
        let source = proof.sources();
        let a = roots.iter().position(|r| Arc::ptr_eq(r, source[0]));
        let b = roots.iter().position(|r| Arc::ptr_eq(r, source[1]));
        let (Some(a), Some(b)) = (a, b) else {
            return Err(unproved(
                "Root forest relation belongs to another source owner",
            ));
        };
        if a == b || seen[a][b] {
            return Err(unproved("Root forest duplicates a source pair"));
        }
        seen[a][b] = true;
        seen[b][a] = true;
        match proof.kind() {
            RootRelationKind::Disjoint => (),
            RootRelationKind::FirstInsideSecond => inside[a][b] = true,
            RootRelationKind::SecondInsideFirst => inside[b][a] = true,
        }
    }
    analyze(&inside)
}
fn analyze(inside: &[Vec<bool>]) -> Result<(Vec<Option<usize>>, Vec<usize>)> {
    let n = inside.len();
    if !(2..=64).contains(&n) || inside.iter().any(|r| r.len() != n) || (0..n).any(|i| inside[i][i])
    {
        return Err(unproved("Invalid strict root containment matrix"));
    }
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                if inside[i][j] && inside[j][k] && !inside[i][k] {
                    return Err(unproved(
                        "Root containment certificates are not transitively consistent",
                    ));
                }
            }
        }
    }
    let mut parents = vec![None; n];
    for i in 0..n {
        let candidates: Vec<_> = (0..n)
            .filter(|&j| inside[i][j] && !(0..n).any(|k| inside[i][k] && inside[k][j]))
            .collect();
        if candidates.len() > 1 {
            return Err(unproved(
                "Root forest has incomparable immediate containers",
            ));
        }
        parents[i] = candidates.first().copied();
    }
    let mut depths = vec![0; n];
    for i in 0..n {
        let mut current = parents[i];
        while let Some(p) = current {
            depths[i] += 1;
            if depths[i] >= n {
                return Err(unproved("Root containment graph has a cycle"));
            }
            current = parents[p];
        }
    }
    Ok((parents, depths))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_cavities_and_material_islands_have_alternating_depths() {
        let mut inside = vec![vec![false; 5]; 5];
        for i in 1..4 {
            for j in 0..i {
                inside[i][j] = true;
            }
        }
        let (parents, depths) = analyze(&inside).unwrap();
        assert_eq!(parents, vec![None, Some(0), Some(1), Some(2), None]);
        assert_eq!(depths, vec![0, 1, 2, 3, 0]);
    }
    #[test]
    fn incomplete_containment_and_incomparable_parents_are_rejected() {
        let mut inside = vec![vec![false; 3]; 3];
        inside[2][1] = true;
        inside[1][0] = true;
        assert!(analyze(&inside).is_err());
        inside[1][0] = false;
        inside[2][0] = true;
        assert!(analyze(&inside).is_err());
        inside[0][2] = true;
        assert!(analyze(&inside).is_err());
    }
    #[test]
    fn source_order_does_not_define_the_material_parent() {
        let mut inside = vec![vec![false; 4]; 4];
        inside[0][3] = true;
        inside[0][2] = true;
        inside[3][2] = true;
        let (parents, depths) = analyze(&inside).unwrap();
        assert_eq!(parents, vec![Some(3), None, None, Some(2)]);
        assert_eq!(depths, vec![2, 0, 0, 1]);
    }
}
