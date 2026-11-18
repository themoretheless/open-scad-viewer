//! Exact subdivision exclusion; all unprocessed candidates remain in the hull.
use super::*;
struct Cell {
    h: Vec<Vec<[Q; 3]>>,
    uv: [[Q; 2]; 2],
    depth: usize,
}
fn possible(h: &[Vec<[Q; 3]>], r: &[[Q; 2]; 2]) -> bool {
    (0..2).all(|axis| {
        let c: Vec<_> = h.iter().flatten().map(|a| &a[axis] / &a[2]).collect();
        c.iter().min().unwrap() <= &r[axis][1] && c.iter().max().unwrap() >= &r[axis][0]
    })
}
// This module is used only after the global injectivity certificate. Exact
// equality to a point query therefore identifies its UNIQUE inverse. Rounded
// guesses or a non-singleton query never permit singleton contraction.
fn singleton(
    h: &[Vec<[Q; 3]>],
    r: &[[Q; 2]; 2],
    uv: &[[Q; 2]; 2],
    work: &mut Work,
) -> Option<Option<[[Q; 2]; 2]>> {
    if r.iter().any(|a| a[0] != a[1]) {
        return Some(None);
    }
    let matches = |a: &[Q; 3], work: &mut Work| -> Option<bool> {
        for k in 0..2 {
            if work.keep(&a[k] - &r[k][0] * &a[2])? != q(0.) {
                return Some(false);
            }
        }
        Some(true)
    };
    let (p, qv) = (h.len() - 1, h[0].len() - 1);
    for (i, j, a, b) in [(0, 0, 0, 0), (p, 0, 1, 0), (0, qv, 0, 1), (p, qv, 1, 1)] {
        if matches(&h[i][j], work)? {
            return Some(Some([
                [uv[0][a].clone(), uv[0][a].clone()],
                [uv[1][b].clone(), uv[1][b].clone()],
            ]));
        }
    }
    let mut column = Vec::new();
    for row in h {
        let mut d = row.clone();
        for level in 1..=qv {
            for j in 0..=qv - level {
                for k in 0..3 {
                    d[j][k] = work.keep((&d[j][k] + &d[j + 1][k]) / q(2.))?;
                }
            }
        }
        column.push(d[0].clone());
    }
    for level in 1..=p {
        for i in 0..=p - level {
            for k in 0..3 {
                column[i][k] = work.keep((&column[i][k] + &column[i + 1][k]) / q(2.))?;
            }
        }
    }
    if matches(&column[0], work)? {
        let point = uv.clone().map(|a| (a[0].clone() + &a[1]) / q(2.));
        return Some(Some(point.map(|x| [x.clone(), x])));
    }
    Some(None)
}
pub(super) fn enclosure(patches: &[Patch], r: &[[Q; 2]; 2]) -> Result<[[Q; 2]; 2]> {
    let pending = patches
        .iter()
        .map(|p| Cell {
            h: p.h.clone(),
            uv: p.uv.clone(),
            depth: 0,
        })
        .collect();
    enclose_cells(pending, r, LIMIT)
        .ok_or_else(|| crate::input("Joint chart inverse has no candidate"))
}
#[cfg(test)]
fn enclose(h: &[Vec<[Q; 3]>], r: &[[Q; 2]; 2], limit: u64) -> Option<[[Q; 2]; 2]> {
    enclose_cells(
        vec![Cell {
            h: h.to_vec(),
            uv: [[q(0.), q(1.)], [q(0.), q(1.)]],
            depth: 0,
        }],
        r,
        limit,
    )
}
fn enclose_cells(mut pending: Vec<Cell>, r: &[[Q; 2]; 2], limit: u64) -> Option<[[Q; 2]; 2]> {
    let mut hull: Option<[[Q; 2]; 2]> = None;
    let mut work = Work::new(limit);
    let mut nodes = 0;
    let add = |hull: &mut Option<[[Q; 2]; 2]>, uv: [[Q; 2]; 2]| {
        if let Some(a) = hull {
            for k in 0..2 {
                a[k][0] = a[k][0].clone().min(uv[k][0].clone());
                a[k][1] = a[k][1].clone().max(uv[k][1].clone());
            }
        } else {
            *hull = Some(uv);
        }
    };
    while let Some(cell) = pending.pop() {
        nodes += 1;
        if work
            .charge((cell.h.len() * cell.h[0].len() * 8) as u64)
            .is_none()
            || nodes > 4096
        {
            add(&mut hull, cell.uv);
            for c in pending {
                add(&mut hull, c.uv);
            }
            break;
        }
        if !possible(&cell.h, r) {
            continue;
        }
        match singleton(&cell.h, r, &cell.uv, &mut work) {
            Some(Some(point)) => return Some(point),
            Some(None) => {}
            None => {
                add(&mut hull, cell.uv);
                for c in pending {
                    add(&mut hull, c.uv);
                }
                break;
            }
        }
        if cell.depth == 96 {
            add(&mut hull, cell.uv);
            continue;
        }
        let axis = cell.depth % 2;
        let Some((left, right)) = poly::split(cell.h, axis, &mut work) else {
            add(&mut hull, cell.uv);
            for c in pending {
                add(&mut hull, c.uv);
            }
            break;
        };
        let mid = (&cell.uv[axis][0] + &cell.uv[axis][1]) / q(2.);
        let mut a = cell.uv.clone();
        let mut b = cell.uv;
        a[axis][1] = mid.clone();
        b[axis][0] = mid;
        pending.push(Cell {
            h: right,
            uv: b,
            depth: cell.depth + 1,
        });
        pending.push(Cell {
            h: left,
            uv: a,
            depth: cell.depth + 1,
        });
    }
    hull
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zero_work_preserves_all_pending_candidates() {
        let h = vec![
            vec![[q(0.), q(0.), q(1.)], [q(0.), q(1.), q(1.)]],
            vec![[q(1.), q(0.), q(1.)], [q(1.), q(1.), q(1.)]],
        ];
        let r = [[q(0.5), q(0.5)], [q(0.5), q(0.5)]];
        assert_eq!(enclose(&h, &r, 0), Some([[q(0.), q(1.)], [q(0.), q(1.)]]));
        let uv = enclose(&h, &r, LIMIT).unwrap();
        for b in uv {
            assert!(b[0] <= q(0.5) && q(0.5) <= b[1]);
            assert!(&b[1] - &b[0] < q(1e-12));
        }
    }
}

#[cfg(test)]
#[test]
fn exact_singleton_acceleration_never_collapses_a_query_rectangle() {
    let h = vec![
        vec![[q(0.), q(0.), q(1.)], [q(0.), q(1.), q(1.)]],
        vec![[q(1.), q(0.), q(1.)], [q(1.), q(1.), q(1.)]],
    ];
    let point = [[q(0.25), q(0.25)], [q(0.75), q(0.75)]];
    assert_eq!(enclose(&h, &point, LIMIT), Some(point));
    let rectangle = [[q(0.375), q(0.625)], [q(0.375), q(0.625)]];
    let uv = enclose(&h, &rectangle, LIMIT).unwrap();
    for b in uv {
        assert!(b[0] <= q(0.375) && b[1] >= q(0.625));
    }
    let nondyadic = [[q(0.3), q(0.3)], [q(0.7), q(0.7)]];
    let uv = enclose(&h, &nondyadic, LIMIT).unwrap();
    for (k, b) in uv.into_iter().enumerate() {
        assert!(b[0] <= nondyadic[k][0] && nondyadic[k][0] <= b[1]);
        assert!(&b[1] - &b[0] < q(1e-12));
    }
}

#[cfg(test)]
#[test]
fn all_initial_knot_cells_survive_inverse_work_exhaustion() {
    let cells = || {
        vec![
            Cell {
                h: vec![
                    vec![[q(0.), q(0.), q(1.)], [q(0.), q(1.), q(1.)]],
                    vec![[q(0.5), q(0.), q(1.)], [q(0.5), q(1.), q(1.)]],
                ],
                uv: [[q(0.), q(0.5)], [q(0.), q(1.)]],
                depth: 0,
            },
            Cell {
                h: vec![
                    vec![[q(0.5), q(0.), q(1.)], [q(0.5), q(1.), q(1.)]],
                    vec![[q(1.), q(0.), q(1.)], [q(1.), q(1.), q(1.)]],
                ],
                uv: [[q(0.5), q(1.)], [q(0.), q(1.)]],
                depth: 0,
            },
        ]
    };
    let r = [[q(0.75), q(0.75)], [q(0.5), q(0.5)]];
    assert_eq!(
        enclose_cells(cells(), &r, 0),
        Some([[q(0.), q(1.)], [q(0.), q(1.)]])
    );
    assert_eq!(enclose_cells(cells(), &r, LIMIT), Some(r));
}
