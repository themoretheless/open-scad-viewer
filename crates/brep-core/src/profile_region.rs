//! Material roles of retained general XY NURBS loops, with bounded proofs.
//! Analytic profiles retain their existing event-arrangement path.
use crate::{Result, invalid, planar_trim};
use nurbs_core::{
    curve::Curve,
    curve_distance, planar_area,
    trim_domain::{Location, TrimDomain},
    trim_region_audit,
};

pub struct Report {
    pub areas_mm2: Vec<[f64; 2]>,
    pub depths: Vec<usize>,
    pub winding: Vec<i32>,
    pub components: Vec<(usize, Vec<usize>)>,
}
fn failure(loop_index: usize, reason: &str) -> nurbs_core::Error {
    invalid(format!("Profile loop {loop_index}: {reason}"))
}
/// Proves each original loop simple, all distinct loops disjoint, and nesting.
/// Unknown results always refuse admission. No display tessellation is used.
pub fn inspect(loops: &[Vec<Curve>], tolerance: f64, material_left: bool) -> Result<Report> {
    if !tolerance.is_finite()
        || tolerance <= 0.
        || loops.len() > 64
        || loops.iter().map(Vec::len).sum::<usize>() > 254
        || loops
            .iter()
            .flatten()
            .map(|c| c.control_points.len())
            .sum::<usize>()
            > 8192
    {
        return Err(invalid("Profile resource limits or tolerance are invalid"));
    }
    let mut out = Report {
        areas_mm2: vec![],
        depths: vec![0; loops.len()],
        winding: vec![],
        components: vec![],
    };
    if loops.is_empty() {
        return Ok(out);
    }
    let (mut cells, mut domain_cells, mut pairs, mut area_cells) = (0, 0, 0, 0);
    for (i, wire) in loops.iter().enumerate() {
        for c in wire {
            c.validate()?;
            if c.control_points
                .iter()
                .any(|p| p.len() != 2 || p.iter().any(|x| x.abs() > 1e6))
            {
                return Err(failure(i, "requires bounded XY controls"));
            }
        }
        if cells >= 100000 || domain_cells >= 1000000 || pairs >= 100000 {
            return Err(failure(i, "topology work limit"));
        }
        let audit = trim_region_audit::inspect(
            std::slice::from_ref(wire),
            tolerance,
            100000 - pairs,
            100000 - cells,
            (1000000 - domain_cells).min(100000),
        )?;
        cells += audit.cells;
        domain_cells += audit.domain_cells;
        pairs += audit.pairs;
        if audit.valid != Some(true) {
            return Err(failure(i, audit.reason));
        }
        out.winding
            .push(audit.winding[0].ok_or_else(|| failure(i, "orientation unproven"))?);
        // Absolute area tolerance is separate from the length tolerance.
        // Scale it using a translation-independent control extent.
        let bounds = wire.iter().flat_map(|c| &c.control_points).fold(
            [[f64::INFINITY, f64::NEG_INFINITY]; 2],
            |mut b, p| {
                for k in 0..2 {
                    b[k][0] = b[k][0].min(p[k]);
                    b[k][1] = b[k][1].max(p[k]);
                }
                b
            },
        );
        let area_tolerance =
            1e-7_f64.max((bounds[0][1] - bounds[0][0]) * (bounds[1][1] - bounds[1][0]) * 1e-10);
        if area_cells >= 100000 {
            return Err(failure(i, "area work limit"));
        }
        let area = planar_area::measure(wire, area_tolerance, 100000 - area_cells)?;
        area_cells += area.cells;
        if !area.converged {
            return Err(failure(i, area.reason));
        }
        if !(area.area_interval_mm2[0] > 0. || area.area_interval_mm2[1] < 0.) {
            return Err(failure(i, "area sign unproven"));
        }
        out.areas_mm2.push(area.area_interval_mm2);
    }
    for i in 0..loops.len() {
        for j in i + 1..loops.len() {
            for a in &loops[i] {
                for b in &loops[j] {
                    if cells >= 100000 || pairs >= 100000 {
                        return Err(failure(i, "loop separation work limit"));
                    }
                    let distance =
                        curve_distance::prove_separation(a, b, tolerance, (100000 - cells).min(4096))?;
                    cells += distance.cells;
                    pairs += 1;
                    if distance.distance_interval_mm[0] <= 0. {
                        return Err(failure(i, &format!("separation from loop {j} unproven")));
                    }
                }
            }
        }
    }
    let domains = loops
        .iter()
        .map(|l| TrimDomain::new(std::slice::from_ref(l), tolerance))
        .collect::<Result<Vec<_>>>()?;
    let mut inside = vec![vec![false; loops.len()]; loops.len()];
    for i in 0..loops.len() {
        for j in 0..loops.len() {
            if i == j {
                continue;
            }
            if domain_cells >= 1000000 {
                return Err(failure(i, "nesting work limit"));
            }
            // Exact joined source endpoint, separated from the queried boundary.
            let p = &loops[i][0].control_points[0];
            let query = domains[j].classify(
                [[p[0], p[0]], [p[1], p[1]]],
                (1000000 - domain_cells).min(100000),
            )?;
            domain_cells += query.cells;
            inside[i][j] = match query.location {
                Location::Inside => true,
                Location::Outside => false,
                _ => {
                    return Err(failure(
                        i,
                        &format!("nesting relative to loop {j} unproven"),
                    ));
                }
            };
        }
    }
    out.depths = inside
        .iter()
        .map(|row| row.iter().filter(|&&x| x).count())
        .collect();
    let mut parents = vec![None; loops.len()];
    for i in 0..loops.len() {
        if material_left && out.winding[i] != (if out.depths[i] % 2 == 0 { 1 } else { -1 }) {
            return Err(failure(i, "material-left orientation mismatch"));
        }
        if out.depths[i] > 0 {
            let candidates = (0..loops.len())
                .filter(|&j| inside[i][j] && out.depths[j] + 1 == out.depths[i])
                .collect::<Vec<_>>();
            if candidates.len() != 1 {
                return Err(failure(i, "nesting parent unproven"));
            }
            parents[i] = Some(candidates[0]);
        }
    }
    for i in 0..loops.len() {
        if out.depths[i] % 2 == 0 {
            out.components.push((
                i,
                (0..loops.len())
                    .filter(|&j| parents[j] == Some(i))
                    .collect(),
            ));
        }
    }
    Ok(out)
}
pub fn validate(loops: &[Vec<Curve>], tolerance: f64) -> Result<()> {
    if planar_trim::validate(loops, tolerance).is_ok() {
        return Ok(());
    }
    inspect(loops, tolerance, true).map(|_| ())
}
pub fn components(loops: &[Vec<Curve>], tolerance: f64) -> Result<Vec<(usize, Vec<usize>)>> {
    if let Ok(c) = planar_trim::components(loops, tolerance) {
        return Ok(c);
    }
    Ok(inspect(loops, tolerance, true)?.components)
}
pub fn orient_even_odd(loops: &[Vec<Curve>], tolerance: f64) -> Result<Vec<Vec<Curve>>> {
    if let Ok(c) = planar_trim::orient_even_odd(loops, tolerance) {
        return Ok(c);
    }
    let r = inspect(loops, tolerance, false)?;
    loops
        .iter()
        .enumerate()
        .map(|(i, l)| {
            if r.winding[i] == (if r.depths[i] % 2 == 0 { 1 } else { -1 }) {
                Ok(l.clone())
            } else {
                l.iter().rev().map(Curve::reverse).collect()
            }
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn loop_at(x: f64, y: f64, size: f64) -> Vec<Curve> {
        let p = [[x, y], [x + size, y], [x + size, y + size], [x, y + size]];
        let mut l = (0..4)
            .map(|i| Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap())
            .collect::<Vec<_>>();
        l[0] = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 1., 1., 1.],
            control_points: vec![
                vec![x, y],
                vec![x + size * 0.25, y - size * 0.1],
                vec![x + size * 0.75, y - size * 0.1],
                vec![x + size, y],
            ],
            weights: vec![1., 0.8, 1.2, 1.],
            periodic: false,
        };
        l
    }
    #[test]
    fn unordered_nested_islands_and_disjoint_components() {
        let loops = vec![
            loop_at(3., 3., 1.),
            loop_at(20., 0., 3.),
            loop_at(0., 0., 10.),
            loop_at(2., 2., 4.),
        ];
        let before = value_codec::to_string(&loops).unwrap();
        assert!(inspect(&loops, 1e-7, true).is_err());
        let oriented = orient_even_odd(&loops, 1e-7).unwrap();
        let r = inspect(&oriented, 1e-7, true).unwrap();
        assert_eq!(r.depths, vec![2, 0, 0, 1]);
        assert_eq!(r.winding, vec![1, 1, 1, -1]);
        assert_eq!(r.components, vec![(0, vec![]), (1, vec![]), (2, vec![3])]);
        assert_eq!(value_codec::to_string(&loops).unwrap(), before);
    }
    #[test]
    fn refuses_crossing_contact_and_open_joins() {
        for other in [loop_at(1., 1., 3.), loop_at(0., 1., 1.)] {
            assert!(inspect(&[loop_at(0., 0., 3.), other], 1e-7, false).is_err());
        }
        let mut l = loop_at(0., 0., 3.);
        l[0].control_points[0][0] += 0.01;
        assert!(inspect(&[l], 1e-7, true).is_err());
    }
    #[test]
    fn general_extrusion_retains_rational_boundaries() {
        let loops = vec![loop_at(0., 0., 3.)];
        let model = crate::prism::extrude(&loops, 0., 5.).unwrap();
        model.validate().unwrap();
        assert!(
            model
                .edges
                .iter()
                .any(|e| e.curve.degree == 2 && e.curve.weights.iter().any(|&w| w != 1.))
        );
        assert_eq!(model.bodies.len(), 1);
    }
}
