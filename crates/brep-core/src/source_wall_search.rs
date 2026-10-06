//! Sampled normal proposals; only fresh original material proofs admit results.
//! Exhausting proposals never establishes absence of thinner material.
use crate::{
    source_material_chord::{self, Certificate, Limits},
    source_volume::Body,
};
use nurbs_core::{Error, Result, surface_distance::rectangle_bounds};
pub struct Report<'a> {
    pub best: Option<Certificate<'a>>,
    pub attempts: usize,
    pub refused: usize,
    pub candidates_exhausted: bool,
}
pub fn search<'a>(
    body: &'a Body,
    groups: [&[usize]; 2],
    grid: usize,
    max_attempts: usize,
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report<'a>> {
    crate::material_segment::valid_line(
        [0.; 3],
        [1., 0., 0.],
        tolerance_uv,
        limits.cells,
        limits.domain_cells,
    )?;
    if !(1..=100000).contains(&limits.normal_spans)
        || !limits.max_sine_squared.is_finite()
        || !(0. ..1.).contains(&limits.max_sine_squared)
    {
        return Err(Error::new(
            "BREP_SOURCE_WALL_SEARCH_LIMITS",
            "Choose bounded positive normal work and angle tolerance",
        ));
    }
    let regions = body.geometry().shell().regions().unwrap();
    if !(1..=8).contains(&grid)
        || !(1..=256).contains(&max_attempts)
        || groups
            .iter()
            .any(|g| g.is_empty() || g.iter().any(|&f| f >= regions.len()))
        || groups[0].iter().any(|f| groups[1].contains(f))
    {
        return Err(Error::new(
            "BREP_SOURCE_WALL_SEARCH_INPUT",
            "Choose disjoint owned face groups and bounded proposal work",
        ));
    }
    let mut hull = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for r in regions {
        let s = r.loops()[0][0].surface();
        let b = rectangle_bounds(
            s,
            [
                [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
            ],
        )?;
        for k in 0..3 {
            hull[k][0] = hull[k][0].min(b[k][0]);
            hull[k][1] = hull[k][1].max(b[k][1]);
        }
    }
    // Proposal arithmetic only. Exterior membership is independently proved by
    // source_material_chord; overflow/rounding can only prevent a certificate.
    let reach = (0..3).map(|k| hull[k][1] - hull[k][0]).sum::<f64>() + 1.;
    let mut out = Report {
        best: None,
        attempts: 0,
        refused: 0,
        candidates_exhausted: false,
    };
    for &face in groups[0] {
        let s = regions[face].loops()[0][0].surface();
        let d = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        for u in 0..grid {
            for v in 0..grid {
                if out.attempts == max_attempts {
                    return Ok(out);
                }
                out.attempts += 1;
                let uv = [u, v].map(|i| (i as f64 + 0.5) / grid as f64);
                let e = s.evaluate(
                    d[0][0] + uv[0] * (d[0][1] - d[0][0]),
                    d[1][0] + uv[1] * (d[1][1] - d[1][0]),
                )?;
                let Some(n) = e.unit_normal() else {
                    out.refused += 1;
                    continue;
                };
                let origin = std::array::from_fn(|k| e.point[k] - reach * n[k]);
                let direction = n.map(|x| 2. * reach * x);
                if !origin.iter().chain(&direction).all(|x| x.is_finite()) {
                    out.refused += 1;
                    continue;
                }
                let r = source_material_chord::qualify(
                    body,
                    origin,
                    direction,
                    tolerance_uv,
                    Limits {
                        cells: limits.cells,
                        domain_cells: limits.domain_cells,
                        normal_spans: limits.normal_spans,
                        max_sine_squared: limits.max_sine_squared,
                    },
                )?;
                let Some(c) = r.certificate else {
                    out.refused += 1;
                    continue;
                };
                let f = c.faces();
                if !(groups[0].contains(&f[0]) && groups[1].contains(&f[1])
                    || groups[0].contains(&f[1]) && groups[1].contains(&f[0]))
                {
                    out.refused += 1;
                    continue;
                }
                if out
                    .best
                    .as_ref()
                    .is_none_or(|b| c.length_mm()[1] < b.length_mm()[1])
                {
                    out.best = Some(c);
                }
            }
        }
    }
    out.candidates_exhausted = true;
    Ok(out)
}
