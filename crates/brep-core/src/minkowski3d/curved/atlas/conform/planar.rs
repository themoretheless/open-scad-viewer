//! Eliminate an algebraic boundary clock only after proving the entire planar
//! source chart has exactly its corner-triangle image. No sampled replacement.
use super::*;
pub(super) fn canonical(p: &Patch, work: &mut Work) -> Result<Option<Net>> {
    if p.h.len() == 2 {
        return Ok(None);
    }
    let c = bernstein::corners(&p.h);
    let n = cross(&sub(&c[1], &c[0]), &sub(&c[2], &c[0]));
    if zero(&n) {
        return Ok(None);
    }
    for h in p.h.iter().flatten() {
        work.charge(1)?;
        let delta = std::array::from_fn(|k| &h[k] - &c[0][k] * &h[3]);
        if !dot(&n, &delta).is_zero() {
            return Ok(None);
        }
    }
    for (k, h) in bernstein::edges(&p.h).into_iter().enumerate() {
        let direction = sub(&c[(k + 1) % 3], &c[k]);
        for point in h {
            work.charge(1)?;
            let delta = std::array::from_fn(|axis| &point[axis] - &c[k][axis] * &point[3]);
            if !zero(&cross(&direction, &delta)) {
                return Ok(None);
            }
        }
    }
    // Strict interior Jacobian, complete boundary Jordan proof and the homotopy
    // prove a homeomorphism onto this planar triangle even at stationary clocks.
    if !regular_parameter_disk(p, &n, work, true)? {
        return Ok(None);
    }
    let h = |p: &P| [p[0].clone(), p[1].clone(), p[2].clone(), Q::one()];
    Ok(Some(vec![vec![h(&c[0]), h(&c[2])], vec![h(&c[1])]]))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn irrational_planar_seam_clock_is_eliminated_inside_a_closed_atlas() {
        let points = [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]].map(|p| p.map(integer));
        let faces = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
        let bary = [[1, 0, 0], [0, 1, 0], [0, 0, 1]].map(|p| p.map(integer));
        let half = Q::new(1.into(), 2.into());
        let mid = [half.clone(), Q::zero(), half.clone()];
        let mut vertices = BTreeMap::new();
        let mut patches = Vec::new();
        let mut work = Work::default();
        for (index, face) in faces.into_iter().enumerate() {
            let h = if index == 1 {
                // X=u², Y=v(1+u): all boundary images are straight, but the
                // seam midpoint has original parameter sqrt(1/2).
                (0..=2)
                    .map(|i| {
                        (0..=2 - i)
                            .map(|j| {
                                let x = if i == 2 { Q::one() } else { Q::zero() };
                                let y = Q::new(j.into(), 2.into())
                                    + if i == 1 && j == 1 {
                                        half.clone()
                                    } else {
                                        Q::zero()
                                    };
                                std::array::from_fn(|k| {
                                    if k == 3 {
                                        Q::one()
                                    } else {
                                        &points[face[0]][k]
                                            + &x * (&points[face[1]][k] - &points[face[0]][k])
                                            + &y * (&points[face[2]][k] - &points[face[0]][k])
                                    }
                                })
                            })
                            .collect()
                    })
                    .collect::<Net>()
            } else {
                let h = |index: usize| {
                    [
                        points[index][0].clone(),
                        points[index][1].clone(),
                        points[index][2].clone(),
                        Q::one(),
                    ]
                };
                vec![vec![h(face[0]), h(face[2])], vec![h(face[1])]]
            };
            if index == 0 {
                for tri in [
                    [bary[0].clone(), bary[1].clone(), mid.clone()],
                    [mid.clone(), bary[1].clone(), bary[2].clone()],
                ] {
                    append_patch(
                        bernstein::restrict(&h, tri, &mut work).unwrap(),
                        false,
                        &mut vertices,
                        &mut patches,
                        &mut work,
                    )
                    .unwrap();
                }
            } else {
                append_patch(h, false, &mut vertices, &mut patches, &mut work).unwrap();
            }
        }
        let mut positions = vec![[Q::zero(), Q::zero(), Q::zero()]; vertices.len()];
        for (p, i) in vertices {
            positions[i as usize] = p;
        }
        let coarse = &patches[2];
        let edge = bernstein::edges(&coarse.h)[0].clone();
        let target = [half, Q::zero(), Q::zero()];
        assert!(nonlinear_cancel::parameter(&edge, &target, &mut work)
            .unwrap()
            .is_none());
        assert!(canonical(coarse, &mut work).unwrap().is_some());
        let mut curved = coarse.clone();
        curved.h[1][0][1] += Q::new(1.into(), 16.into());
        assert!(canonical(&curved, &mut work).unwrap().is_none());
        synchronize(&positions, &mut patches, &mut work).unwrap();
        assert_eq!(patches.len(), 6);
        assert!(open(&patches).is_empty());
        let mut edges = BTreeMap::<(u32, u32), Vec<Vec<sb::H>>>::new();
        for p in &patches {
            for (k, mut h) in bernstein::edges(&p.h).into_iter().enumerate() {
                let (a, b) = (p.ids[k], p.ids[(k + 1) % 3]);
                if a > b {
                    h.reverse();
                }
                edges.entry((a.min(b), a.max(b))).or_default().push(h);
            }
        }
        assert_eq!(positions.len() + patches.len(), edges.len() + 2);
        for pair in edges.values() {
            assert_eq!(pair.len(), 2);
            assert!(sb::same(&pair[0], &pair[1]));
        }
    }
}
