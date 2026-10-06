//! Exact plane image: a natural boundary plus collapsed transverse endpoints.
//! This does not establish retained-region ownership or chart injectivity.
use cad_predicates::Sign;
use nurbs_core::{Error, Result, surface::Surface};

#[derive(Clone)]
pub struct Certificate {
    surface: Surface,
    plane: [[f64; 3]; 3],
    axis: usize,
    upper: bool,
    side: Sign,
    poles: Vec<(bool, [f64; 3])>,
}
impl Certificate {
    pub fn side(&self) -> Sign {
        self.side
    }
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn plane(&self) -> [[f64; 3]; 3] {
        self.plane
    }
    /// The plane image is confined to this boundary and `poles()`.
    pub fn poles(&self) -> &[(bool, [f64; 3])] {
        &self.poles
    }
    pub fn boundary(&self) -> (usize, bool) {
        (self.axis, self.upper)
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub reason: &'static str,
}
/// A single positive rational Bezier chart has strict plane signs outside
/// one natural boundary and any constant transverse endpoint rows. Interior
/// Bernstein bases are positive, so the only extra plane image is those poles.
pub fn certify(
    surface: &Surface,
    plane: [[f64; 3]; 3],
    axis: usize,
    upper: bool,
    max_work: u64,
) -> Result<Report> {
    surface.validate()?;
    if axis > 1
        || !(1..=100_000_000).contains(&max_work)
        || plane.iter().flatten().any(|x| !x.is_finite())
    {
        return Err(Error::new(
            "BREP_SOURCE_POLE_FIBER",
            "Invalid plane, axis or exact budget",
        ));
    }
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        reason: "source-pole-fiber-layout-unproven",
    };
    let counts = [
        surface.control_points.len(),
        surface.control_points[0].len(),
    ];
    let degrees = [surface.degree_u, surface.degree_v];
    let knots = [&surface.knots_u, &surface.knots_v];
    if surface.periodic_u
        || surface.periodic_v
        || surface
            .weights
            .iter()
            .flatten()
            .any(|w| !w.is_finite() || *w <= 0.)
    {
        return Ok(out);
    }
    for a in 0..2 {
        if degrees[a] == 0 || degrees[a] > 32 || counts[a] != degrees[a] + 1 {
            return Ok(out);
        }
        let lo = knots[a][degrees[a]];
        let hi = knots[a][counts[a]];
        if lo >= hi
            || knots[a][..=degrees[a]].iter().any(|&x| x != lo)
            || knots[a][degrees[a] + 1..counts[a]]
                .iter()
                .any(|&x| x <= lo || x >= hi)
            || knots[a][counts[a]..].iter().any(|&x| x != hi)
        {
            return Ok(out);
        }
    }
    out.reason = "source-pole-fiber-plane-unproven";
    if !crate::source_allowed_contact::independent(
        plane[0],
        plane[1],
        plane[2],
        &mut out.exact_work,
        max_work,
    )? {
        return Ok(out);
    }
    out.reason = "source-pole-fiber-support-unproven";
    let boundary = if upper { counts[axis] - 1 } else { 0 };
    let transverse = 1 - axis;
    let mut poles = Vec::new();
    for end in [false, true] {
        let index = if end { counts[transverse] - 1 } else { 0 };
        let points = (0..counts[axis])
            .map(|i| {
                let uv = if axis == 0 { [i, index] } else { [index, i] };
                &surface.control_points[uv[0]][uv[1]]
            })
            .collect::<Vec<_>>();
        if points.iter().all(|p| *p == points[0]) {
            poles.push((end, [points[0][0], points[0][1], points[0][2]]));
        }
    }
    let mut side = None;
    let mut strict = vec![vec![false; counts[1]]; counts[0]];
    for (u, row) in surface.control_points.iter().enumerate() {
        for (v, p) in row.iter().enumerate() {
            let Some(sign) = crate::source_allowed_contact::orient(
                &[plane[0], plane[1], plane[2], [p[0], p[1], p[2]]],
                None,
                &mut out.exact_work,
                max_work,
            )?
            else {
                return Ok(out);
            };
            let collapsed = poles.iter().any(|(end, _)| {
                [u, v][transverse] == if *end { counts[transverse] - 1 } else { 0 }
            });
            if [u, v][axis] == boundary || collapsed {
                if sign != Sign::Zero {
                    return Ok(out);
                }
            } else {
                if sign != Sign::Zero {
                    if side.is_some_and(|s| s != sign) {
                        return Ok(out);
                    }
                    side = Some(sign);
                    strict[u][v] = true;
                }
            }
        }
    }
    // Positive Bernstein bases exclude interior contact even when some
    // coefficients vanish (e.g. a smoothstep endpoint). The opposite natural
    // edge must have a strict coefficient; each noncollapsed transverse end
    // also needs a strict opposite corner, excluding extra boundary images.
    let opposite = if upper { 0 } else { counts[axis] - 1 };
    let at = |t: usize| {
        let uv = if axis == 0 {
            [opposite, t]
        } else {
            [t, opposite]
        };
        strict[uv[0]][uv[1]]
    };
    if !(0..counts[transverse]).any(at) {
        return Ok(out);
    }
    for end in [false, true] {
        if !poles.iter().any(|(e, _)| *e == end)
            && !at(if end { counts[transverse] - 1 } else { 0 })
        {
            return Ok(out);
        }
    }
    if side.is_none() {
        return Ok(out);
    }
    out.certificate = Some(Certificate {
        surface: surface.clone(),
        plane,
        axis,
        upper,
        side: side.unwrap(),
        poles,
    });
    out.reason = "source-pole-fiber-proven";
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn chart() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.]; 3],
                vec![vec![1., 0., 0.], vec![1., 1., 1.], vec![0., 1., 1.]],
                vec![vec![2., 0., 0.], vec![2., 1., 1.], vec![0., 2., 1.]],
            ],
            weights: vec![vec![1., 0.7, 1.]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    const PLANE: [[f64; 3]; 3] = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
    #[test]
    fn extra_collapsed_boundary_is_an_owned_image_not_a_single_preimage() {
        let s = chart();
        assert!(
            crate::source_plane_fiber::certify(&s, PLANE, 1, false, 1000000)
                .unwrap()
                .certificate
                .is_none()
        );
        let r = certify(&s, PLANE, 1, false, 1000000).unwrap();
        let c = r.certificate.unwrap();
        assert_eq!(c.poles(), &[(false, [0., 0., 0.])]);
        assert_eq!(c.boundary(), (1, false));
        assert_eq!(c.surface(), &s);
        assert!(
            certify(&s, PLANE, 1, false, 1)
                .unwrap()
                .certificate
                .is_none()
        );
    }
    #[test]
    fn original_spherical_meridians_have_boundary_plus_pole_images() {
        for radii in [[0.5, 1.25], [1.25, 0.5], [1., 1.]] {
            for sweep in [std::f64::consts::TAU, -std::f64::consts::TAU] {
                let spans = crate::linear_canal::construct(
                    [[10., -7., 5.], [13., -3., 17.]],
                    radii,
                    [1., 0., 0.],
                    sweep,
                )
                .unwrap();
                let shell = crate::linear_canal::to_capped_source_shell(
                    &spans,
                    1e-7,
                    1e-8,
                    crate::trimmed_face_recipe::Limits {
                        pairs: 10000,
                        region_cells: 10000,
                        domain_cells: 10000,
                        agreement_cells: 10000,
                    },
                    100_000_000,
                )
                .unwrap()
                .shell
                .unwrap();
                let regions = shell.regions().unwrap();
                let mut count = 0;
                for (i, uses) in shell.uses().iter().enumerate() {
                    let surfaces = uses.map(|a| regions[a.face].loops()[0][0].surface());
                    if surfaces.iter().any(|s| s.degree_u != 2) {
                        continue;
                    }
                    let c = shell.edges()[i].world();
                    if c.control_points.len() < 3 {
                        continue;
                    }
                    let plane =
                        std::array::from_fn(|j| std::array::from_fn(|k| c.control_points[j][k]));
                    let certificates = surfaces.map(|s| {
                        (0..2)
                            .flat_map(|axis| [false, true].map(move |end| (axis, end)))
                            .find_map(|(axis, end)| {
                                certify(s, plane, axis, end, 1000000).unwrap().certificate
                            })
                            .expect("original cap meridian needs an exact boundary/pole image")
                    });
                    if certificates.iter().all(|c| c.poles().is_empty()) {
                        continue;
                    }
                    assert_ne!(certificates[0].side(), certificates[1].side());
                    assert!(certificates.iter().all(|c| !c.poles().is_empty()));
                    count += 1;
                }
                assert_eq!(count, 8);
            }
        }
    }
    #[test]
    fn a_zero_nonconstant_row_and_mixed_signs_cannot_be_exempted() {
        let mut s = chart();
        s.control_points[0][1][0] = 0.125;
        assert!(
            certify(&s, PLANE, 1, false, 1000000)
                .unwrap()
                .certificate
                .is_none()
        );
        let mut s = chart();
        s.control_points[1][1][2] = -1.;
        assert!(
            certify(&s, PLANE, 1, false, 1000000)
                .unwrap()
                .certificate
                .is_none()
        );
        let mut s = chart();
        s.control_points[0][0][2] = 1e-12;
        assert!(
            certify(&s, PLANE, 1, false, 1000000)
                .unwrap()
                .certificate
                .is_none()
        );
    }
    #[test]
    fn zero_interior_coefficients_keep_strict_image_but_extra_corner_refuses() {
        let mut surface = chart();
        surface.control_points[1][1][2] = 0.;
        assert!(
            certify(&surface, PLANE, 1, false, 1000000)
                .unwrap()
                .certificate
                .is_some()
        );
        // The opposite noncollapsed corner would introduce an extra plane point.
        surface.control_points[2][2][2] = 0.;
        assert!(
            certify(&surface, PLANE, 1, false, 1000000)
                .unwrap()
                .certificate
                .is_none()
        );
        surface.control_points[2][2][2] = 1.;
        surface.control_points[1][1][2] = -1.;
        assert!(
            certify(&surface, PLANE, 1, false, 1000000)
                .unwrap()
                .certificate
                .is_none()
        );
    }
}
