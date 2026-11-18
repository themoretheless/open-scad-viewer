//! Exact bounded proposals for a positive local projection. Acceptance still
//! requires the complete projected atlas disk and Jordan-link predicates.
use super::*;
pub(super) fn directions(normals: &BTreeSet<P>, work: &mut Work) -> Result<Vec<P>> {
    if normals.len() > 32 {
        return Ok(Vec::new());
    }
    let mut out = BTreeSet::new();
    for bound in [2, 8, 32] {
        let mut rows = normals
            .iter()
            .cloned()
            .map(|p| (p, Q::one()))
            .collect::<Vec<_>>();
        for axis in 0..3 {
            for sign in [-1, 1] {
                rows.push((
                    std::array::from_fn(|k| if k == axis { integer(sign) } else { Q::zero() }),
                    integer(-bound),
                ));
            }
        }
        for i in 0..rows.len() {
            for j in i + 1..rows.len() {
                for k in j + 1..rows.len() {
                    work.charge(rows.len() + 8)?;
                    let (a, b, c) = (&rows[i], &rows[j], &rows[k]);
                    let det = dot(&a.0, &cross(&b.0, &c.0));
                    if det.is_zero() {
                        continue;
                    }
                    let n: P = std::array::from_fn(|q| {
                        (&a.1 * cross(&b.0, &c.0)[q].clone()
                            + &b.1 * cross(&c.0, &a.0)[q].clone()
                            + &c.1 * cross(&a.0, &b.0)[q].clone())
                            / &det
                    });
                    if rows.iter().any(|(p, v)| dot(p, &n) < *v) {
                        continue;
                    }
                    let norm: Q = n
                        .iter()
                        .map(|q| {
                            if *q < Q::zero() {
                                -q.clone()
                            } else {
                                q.clone()
                            }
                        })
                        .sum();
                    if norm.is_zero() {
                        continue;
                    }
                    out.insert(n.map(|q| q / &norm));
                    if out.len() >= 12 {
                        return Ok(out.into_iter().collect());
                    }
                }
            }
        }
    }
    Ok(out.into_iter().collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn positive_projection_is_constructed_and_opposite_faces_refuse() {
        let normals = [[0, 0, -1], [2, 2, 1], [2, -2, 1]]
            .map(|p| p.map(integer))
            .into_iter()
            .collect::<BTreeSet<_>>();
        let n = directions(&normals, &mut Work::default()).unwrap();
        assert!(!n.is_empty());
        assert!(
            n.iter()
                .all(|p| normals.iter().all(|q| dot(p, q) > Q::zero()))
        );
        let bad = [[0, 0, 1], [0, 0, -1]]
            .map(|p| p.map(integer))
            .into_iter()
            .collect();
        assert!(directions(&bad, &mut Work::default()).unwrap().is_empty());
    }
}
