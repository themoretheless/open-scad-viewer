//! Seam connectivity: preserves the original arithmetic and proof budgets.
use super::*;

/// Exact connectivity of materialized candidates in lifted UV coordinates.
/// Never forms rounded endpoint+translation sums. For a closed periodic chain,
/// last-first must equal winding*period exactly; a nonzero winding does not
/// imply that the loop bounds a planar trim region. No source-region transfer,
/// simplicity, intersections or surface correspondence is inferred here.
pub fn inspect_connectivity(
    partition: &Report,
    chart: [[f64; 2]; 2],
    winding: Option<[i32; 2]>,
    max_links: usize,
) -> Result<Connectivity> {
    check(
        !partition.pieces.is_empty()
            && partition.pieces.len() <= 256
            && (1..=257).contains(&max_links)
            && chart
                .iter()
                .all(|r| r.iter().all(|x| x.is_finite()) && r[0] < r[1]),
        "Invalid periodic connectivity input or budget",
    )?;
    for (i, piece) in partition.pieces.iter().enumerate() {
        piece.curve.validate()?;
        check(
            !piece.curve.periodic
                && piece.curve.control_points[0].len() == 2
                && piece.source_domain == piece.curve.domain()
                && piece.translation.iter().all(|x| x.is_finite())
                && (i == 0 || partition.pieces[i - 1].source_domain[1] == piece.source_domain[0]),
            "Connectivity requires ordered finite UV candidates",
        )?;
    }
    let n = partition.pieces.len();
    let expected_links = n - 1 + usize::from(winding.is_some());
    let mut out = Connectivity {
        status: LinkStatus::Exact,
        links: Vec::new(),
        expected_links,
        winding,
    };
    let periods: Option<Vec<_>> = chart
        .iter()
        .map(|r| crate::exact_curve_segments::exact_add(r[1], -r[0]))
        .collect();
    let Some(periods) = periods else {
        out.status = LinkStatus::Unresolved;
        return Ok(out);
    };
    for from in 0..expected_links {
        if out.links.len() == max_links {
            if out.status != LinkStatus::Gap {
                out.status = LinkStatus::Unresolved;
            }
            break;
        }
        let to = (from + 1) % n;
        let left = &partition.pieces[from];
        let right = &partition.pieces[to];
        let c = &left.curve;
        let d = &right.curve;
        let lc = c.control_points.len();
        let clamped = c.knots[lc..].iter().all(|k| *k == c.domain()[1])
            && d.knots[..=d.degree].iter().all(|k| *k == d.domain()[0]);
        let closure = from == n - 1;
        let status = if !clamped {
            LinkStatus::Unresolved
        } else {
            let equal = (0..2).all(|axis| {
                let turn = if closure {
                    winding.unwrap()[axis] as f64
                } else {
                    0.
                };
                crate::exact_products::sum_sign(&[
                    (c.control_points[lc - 1][axis], 1.),
                    (left.translation[axis], 1.),
                    (-d.control_points[0][axis], 1.),
                    (-right.translation[axis], 1.),
                    (-turn, periods[axis]),
                ]) == std::cmp::Ordering::Equal
            });
            if equal {
                LinkStatus::Exact
            } else {
                LinkStatus::Gap
            }
        };
        if status == LinkStatus::Gap {
            out.status = LinkStatus::Gap;
        } else if status == LinkStatus::Unresolved && out.status != LinkStatus::Gap {
            out.status = LinkStatus::Unresolved;
        }
        out.links.push(Link {
            pieces: [from, to],
            closure,
            status,
        });
    }
    Ok(out)
}
