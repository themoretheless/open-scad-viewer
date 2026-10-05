//! Whole-parameter agreement of numerically partitioned original curves.
//! Knot insertion proposes pieces; interval jets qualify their deviation.
//! This is tolerance agreement, not exact identity or a topology split proof.
use crate::{
    Result, check,
    curve::Curve,
    curve_jets, curve_surface_agreement as bounds,
    distance_bounds::{Interval as I, box_distance},
};

pub struct Cell {
    pub interval: [f64; 2],
    pub error_upper: Option<f64>,
    pub anchor_error: Option<[f64; 2]>,
    pub admitted: bool,
}
pub struct Report {
    pub cells: Vec<Cell>,
    pub visited: usize,
    pub agreement_proven: bool,
}
fn continuous(c: &Curve) -> bool {
    let d = c.domain();
    c.knots
        .iter()
        .filter(|&&k| k > d[0] && k < d[1])
        .all(|k| c.knots.iter().filter(|q| *q == k).count() <= c.degree)
}
fn span(c: &Curve, d: [f64; 2]) -> usize {
    (c.degree..c.control_points.len())
        .find(|&i| c.knots[i] <= d[0] && d[1] <= c.knots[i + 1] && c.knots[i] < c.knots[i + 1])
        .unwrap()
}
pub fn certify(
    original: &Curve,
    piece: &Curve,
    tolerance: f64,
    max_cells: usize,
) -> Result<Report> {
    original.validate()?;
    piece.validate()?;
    let original_domain = original.domain();
    let domain = piece.domain();
    check(
        !original.periodic && !piece.periodic && continuous(original) && continuous(piece),
        "Curve partition agreement requires nonperiodic continuous source definitions",
    )?;
    check(
        original.control_points[0].len() == piece.control_points[0].len()
            && original_domain[0] <= domain[0]
            && domain[1] <= original_domain[1],
        "Partition pieces must retain original parameter coordinates and dimensions",
    )?;
    check(
        tolerance.is_finite() && tolerance > 0. && (1..=100000).contains(&max_cells),
        "Partition agreement needs a positive tolerance and 1..100000 shared cells",
    )?;
    if original == piece {
        return Ok(Report {
            cells: vec![Cell {
                interval: domain,
                error_upper: Some(0.),
                anchor_error: Some([0., 0.]),
                admitted: true,
            }],
            visited: 1,
            agreement_proven: true,
        });
    }
    let mut breaks = vec![domain[0], domain[1]];
    breaks.extend(
        original
            .knots
            .iter()
            .chain(&piece.knots)
            .copied()
            .filter(|k| *k > domain[0] && *k < domain[1]),
    );
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    let mut pending = breaks
        .windows(2)
        .rev()
        .map(|p| ([p[0], p[1]], 0usize))
        .collect::<Vec<_>>();
    let mut out = Report {
        cells: vec![],
        visited: 0,
        agreement_proven: false,
    };
    while let Some((d, depth)) = pending.pop() {
        let mut leaf = Cell {
            interval: d,
            error_upper: None,
            anchor_error: None,
            admitted: false,
        };
        // Reserve remaining root/leaf work and retain every omitted cell.
        if out.visited == max_cells {
            out.cells.push(leaf);
            continue;
        }
        out.visited += 1;
        let a = curve_jets::enclose(original, span(original, d), d)?;
        let b = curve_jets::enclose(piece, span(piece, d), d)?;
        let middle = d[0] * 0.5 + d[1] * 0.5;
        let anchor_a = bounds::curve_bounds(original, I::point(middle))?;
        let anchor_b = bounds::curve_bounds(piece, I::point(middle))?;
        let anchor = box_distance(&anchor_a, &anchor_b)?;
        // Both jets use the same cell-normalized parameter x in [0,1].
        let derivative = a[1]
            .iter()
            .zip(&b[1])
            .map(|(a, b)| a.sub(*b))
            .collect::<Result<Vec<_>>>()?;
        let zero = vec![I::point(0.); derivative.len()];
        let derivative_upper = box_distance(&derivative, &zero)?.1;
        // The binary64 midpoint is not assumed to be the exact half parameter.
        let width = I::point(d[1]).sub(I::point(d[0]))?;
        let left = I::point(middle).sub(I::point(d[0]))?.div(width)?.hi;
        let right = I::point(d[1]).sub(I::point(middle))?.div(width)?.hi;
        let upper = I::point(anchor.1)
            .add(I::point(derivative_upper).mul(I::point(left.max(right)))?)?
            .hi;
        leaf.error_upper = Some(upper);
        leaf.anchor_error = Some([anchor.0, anchor.1]);
        leaf.admitted = upper <= tolerance;
        if leaf.admitted
            || anchor.0 > tolerance
            || depth == 32
            || middle <= d[0]
            || middle >= d[1]
            || max_cells - out.visited < pending.len() + 2
        {
            out.cells.push(leaf);
        } else {
            pending.push(([middle, d[1]], depth + 1));
            pending.push(([d[0], middle], depth + 1));
        }
    }
    out.agreement_proven = out.cells.iter().all(|c| c.admitted);
    Ok(out)
}
pub struct Piece {
    pub curve: Curve,
    pub agreement: Option<Report>,
}
pub struct Partition {
    pub pieces: Vec<Piece>,
    pub visited: usize,
    pub geometry_preserved: bool,
    /// Bit-identical candidate joins, separate from source tolerance agreement.
    pub endpoint_joins_exact: bool,
}
pub fn partition(
    original: &Curve,
    cuts: &[f64],
    tolerance: f64,
    max_cells: usize,
) -> Result<Partition> {
    original.validate()?;
    let d = original.domain();
    check(
        !original.periodic && continuous(original),
        "Partition source must be nonperiodic and continuous",
    )?;
    check(
        cuts.len() <= 256
            && cuts.iter().all(|k| k.is_finite() && *k > d[0] && *k < d[1])
            && cuts.windows(2).all(|k| k[0] < k[1]),
        "Partition cuts must be finite, strictly ordered interior original parameters",
    )?;
    check(
        tolerance.is_finite() && tolerance > 0. && (1..=100000).contains(&max_cells),
        "Partition needs bounded positive work and tolerance",
    )?;
    let mut breaks = vec![d[0]];
    breaks.extend_from_slice(cuts);
    breaks.push(d[1]);
    let mut out = Partition {
        pieces: vec![],
        visited: 0,
        geometry_preserved: true,
        endpoint_joins_exact: true,
    };
    for interval in breaks.windows(2) {
        let curve = original.trim(interval[0], interval[1])?;
        check(
            curve.domain() == [interval[0], interval[1]],
            "Partition candidate changed original parameter endpoints",
        )?;
        let agreement = if out.visited < max_cells {
            Some(certify(
                original,
                &curve,
                tolerance,
                max_cells - out.visited,
            )?)
        } else {
            None
        };
        out.geometry_preserved &= agreement.as_ref().is_some_and(|r| r.agreement_proven);
        out.visited += agreement.as_ref().map_or(0, |r| r.visited);
        out.pieces.push(Piece { curve, agreement });
    }
    out.endpoint_joins_exact = out.pieces.windows(2).all(|p| {
        let a = p[0].curve.control_points.last().unwrap();
        let b = &p[1].curve.control_points[0];
        a.iter()
            .map(|v| v.to_bits())
            .eq(b.iter().map(|v| v.to_bits()))
    });
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Curve {
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.4, 0.7, 1., 1., 1.],
            control_points: vec![
                vec![0., 0.],
                vec![1., 2.],
                vec![2., -1.],
                vec![3., 2.],
                vec![4., 0.],
            ],
            weights: vec![1., 0.8, 1.2, 0.9, 1.],
            periodic: false,
        }
    }
    #[test]
    fn rational_multispan_partition_retains_original_parameter_coverage_and_bounded_error() {
        for dim in [2, 3] {
            let mut original = source();
            if dim == 3 {
                for p in &mut original.control_points {
                    p.push(p[0] + 2. * p[1]);
                }
            }
            let before = original.clone();
            let r = partition(&original, &[0.23, 0.83], 1e-4, 100000).unwrap();
            assert!(r.geometry_preserved);
            assert!(r.visited <= 100000);
            assert_eq!(original, before);
            assert_eq!(
                r.pieces
                    .iter()
                    .map(|p| p.curve.domain())
                    .collect::<Vec<_>>(),
                vec![[0., 0.23], [0.23, 0.83], [0.83, 1.]]
            );
            for piece in &r.pieces {
                let audit = piece.agreement.as_ref().unwrap();
                assert_eq!(audit.cells[0].interval[0], piece.curve.domain()[0]);
                assert_eq!(
                    audit.cells.last().unwrap().interval[1],
                    piece.curve.domain()[1]
                );
                for cells in audit.cells.windows(2) {
                    assert_eq!(cells[0].interval[1], cells[1].interval[0]);
                }
                for c in &audit.cells {
                    assert!(c.admitted && c.error_upper.unwrap() <= 1e-4);
                }
            }
        }
    }
    #[test]
    fn altered_piece_and_work_stop_never_gain_geometry_preservation() {
        let original = source();
        let mut piece = original.trim(0.23, 0.83).unwrap();
        piece.control_points[1][1] += 0.2;
        assert!(
            !certify(&original, &piece, 1e-4, 1000)
                .unwrap()
                .agreement_proven
        );
        let p = partition(&original, &[0.23, 0.83], 1e-12, 1).unwrap();
        assert!(!p.geometry_preserved);
        assert_eq!(p.pieces.len(), 3);
        assert_eq!(p.visited, 1);
        assert!(p.pieces[1..].iter().all(|p| p.agreement.is_none()));
    }
    #[test]
    fn invalid_cut_order_and_mismatched_parameter_domains_refuse() {
        let original = source();
        assert!(partition(&original, &[0.8, 0.2], 1e-4, 100).is_err());
        let mut piece = original.clone();
        for k in &mut piece.knots {
            *k += 1.;
        }
        assert!(certify(&original, &piece, 1e-4, 100).is_err());
    }
    #[test]
    fn coincident_anchor_does_not_hide_endpoint_error_on_a_nonunit_domain() {
        let original = Curve {
            degree: 1,
            knots: vec![2., 2., 8., 8.],
            control_points: vec![vec![2., 0.], vec![8., 0.]],
            weights: vec![1., 1.],
            periodic: false,
        };
        let mut candidate = original.clone();
        candidate.control_points[0][1] = -0.1;
        candidate.control_points[1][1] = 0.1;
        let r = certify(&original, &candidate, 0.05, 1).unwrap();
        assert!(!r.agreement_proven);
        assert_eq!(r.visited, 1);
        assert!(r.cells[0].anchor_error.unwrap()[1] < 1e-6);
        assert!(r.cells[0].error_upper.unwrap() >= 0.1);
    }
}
