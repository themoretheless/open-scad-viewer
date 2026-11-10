//! Coupled world/pcurve partition proposals with independent source and lift checks.
//! Reports prove tolerance agreement only; they do not admit a topology mutation.
use crate::{
    Result, check, curve::Curve, curve_partition_agreement as partition,
    curve_surface_agreement as lift, surface::Surface,
};
pub struct Piece {
    pub curve: Curve,
    pub pcurve: Curve,
    pub reversed: bool,
    pub agreement: Option<lift::Report>,
}
pub struct Report {
    pub world: partition::Partition,
    pub uv: partition::Partition,
    /// Forward pcurve traversal order, including reversed world-edge ownership.
    pub pieces: Vec<Piece>,
    pub lift_cells: usize,
    pub qualified: bool,
}
/// Cuts are fractions of the forward pcurve traversal, strictly inside (0,1).
/// Budgets independently bound each source partition and all candidate lifts.
pub fn split(
    world: &Curve,
    pcurve: &Curve,
    surface: &Surface,
    reversed: bool,
    cuts: &[f64],
    tolerance_world: f64,
    tolerance_uv: f64,
    partition_cells: usize,
    lift_cells: usize,
) -> Result<Report> {
    world.validate()?;
    pcurve.validate()?;
    surface.validate()?;
    check(
        world.control_points[0].len() == 3 && pcurve.control_points[0].len() == 2,
        "Boundary partition needs a 3D edge and 2D pcurve",
    )?;
    check(
        cuts.len() <= 256
            && cuts.iter().all(|x| x.is_finite() && *x > 0. && *x < 1.)
            && cuts.windows(2).all(|w| w[0] < w[1]),
        "Boundary cuts need ordered interior traversal fractions",
    )?;
    check(
        (1..=100000).contains(&lift_cells),
        "Boundary lift work must be bounded",
    )?;
    let map = |curve: &Curve, reverse: bool| {
        let d = curve.domain();
        cuts.iter()
            .map(|x| d[0] + (d[1] - d[0]) * if reverse { 1. - x } else { *x })
            .collect::<Vec<_>>()
    };
    let mut wc = map(world, reversed);
    if reversed {
        wc.reverse();
    }
    let pc = map(pcurve, false);
    // Rounded mapped cuts are proposals. Strict-domain partition validation rejects
    // collapsed cuts; full lift checks catch incompatible world/UV parameter maps.
    let world_report = partition::partition(world, &wc, tolerance_world, partition_cells)?;
    let uv_report = partition::partition(pcurve, &pc, tolerance_uv, partition_cells)?;
    let mut out = Report {
        qualified: world_report.geometry_preserved
            && uv_report.geometry_preserved
            && world_report.endpoint_joins_exact
            && uv_report.endpoint_joins_exact,
        world: world_report,
        uv: uv_report,
        pieces: vec![],
        lift_cells: 0,
    };
    let n = out.world.pieces.len();
    for i in 0..n {
        let curve = out.world.pieces[if reversed { n - 1 - i } else { i }]
            .curve
            .clone();
        let pcurve = out.uv.pieces[i].curve.clone();
        let agreement = if out.lift_cells < lift_cells {
            Some(lift::verify(
                &curve,
                &pcurve,
                surface,
                reversed,
                tolerance_world,
                lift_cells - out.lift_cells,
            )?)
        } else {
            None
        };
        out.lift_cells += agreement.as_ref().map_or(0, |a| a.cells);
        out.qualified &= agreement
            .as_ref()
            .is_some_and(|a| a.status == lift::Status::WithinTolerance);
        out.pieces.push(Piece {
            curve,
            pcurve,
            reversed,
            agreement,
        });
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn line(points: Vec<Vec<f64>>, d: [f64; 2]) -> Curve {
        Curve {
            degree: 1,
            knots: vec![d[0], d[0], d[1], d[1]],
            weights: vec![1.; 2],
            control_points: points,
            periodic: false,
        }
    }
    fn plane() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn forward_and_reversed_edges_keep_original_domains_and_traversal() {
        for reversed in [false, true] {
            let mut points = vec![vec![0., 0.5, 0.], vec![1., 0.5, 0.]];
            if reversed {
                points.reverse();
            }
            let world = line(points, [2., 8.]);
            let uv = line(vec![vec![0., 0.5], vec![1., 0.5]], [10., 14.]);
            let r = split(
                &world,
                &uv,
                &plane(),
                reversed,
                &[0.25, 0.75],
                1e-7,
                1e-7,
                10000,
                10000,
            )
            .unwrap();
            assert!(r.qualified);
            assert_eq!(r.pieces.len(), 3);
            assert_eq!(r.pieces[0].pcurve.domain(), [10., 11.]);
            assert_eq!(
                r.pieces[0].curve.domain(),
                if reversed { [6.5, 8.] } else { [2., 3.5] }
            );
            assert!(r.pieces.iter().all(|p| p.reversed == reversed));
        }
    }
    #[test]
    fn wrong_lift_and_exhaustion_retain_candidates_without_qualification() {
        let world = line(vec![vec![0., 0.5, 0.], vec![1., 0.5, 0.]], [2., 8.]);
        let uv = line(vec![vec![0., 0.6], vec![1., 0.6]], [10., 14.]);
        let r = split(
            &world,
            &uv,
            &plane(),
            false,
            &[0.25, 0.75],
            1e-7,
            1e-7,
            10000,
            1,
        )
        .unwrap();
        assert!(!r.qualified);
        assert_eq!(r.pieces.len(), 3);
        assert_eq!(r.lift_cells, 1);
        assert!(r.pieces[1..].iter().all(|p| p.agreement.is_none()));
    }
}
