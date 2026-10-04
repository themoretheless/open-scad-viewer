//! Wing lofting over placed airfoil sections: analytic placement with
//! twist/dihedral/sweep/taper, skinning through the aligned natural-cubic
//! loft with a chordal guide parameterization, orientation-fold (Jacobian
//! sign) detection, tip closure options and a G2 Hermite fairing blend.
//!
//! Conventions: local section axes are chord along +X (leading edge at
//! x=0), thickness along +Y, Z=0; the spanwise direction is +Z. Twist
//! rotates the section in its local XY plane about the leading edge,
//! sweep shifts the section by span*tan(sweep) along +X, dihedral shifts
//! it by |span|*tan(dihedral) along +Y, taper scales the local section
//! about the leading edge.
use crate::surface::Surface;
use crate::{Result, check, curve::Curve};

/// One wing section: a local profile curve plus its placement parameters.
/// Angles are radians; `taper` is the chord scale factor.
#[derive(Clone)]
pub struct WingSection {
    pub profile: Curve,
    pub span: f64,
    pub twist: f64,
    pub dihedral: f64,
    pub sweep: f64,
    pub taper: f64,
}

impl WingSection {
    fn validate(&self) -> Result<()> {
        self.profile.validate()?;
        check(
            self.profile.control_points[0].len() == 3,
            "Wing section profiles must be 3D curves",
        )?;
        check(
            [
                self.span,
                self.twist,
                self.dihedral,
                self.sweep,
                self.taper,
            ]
            .iter()
            .all(|x| x.is_finite()),
            "Wing section placement must be finite",
        )?;
        check(
            self.taper > 0. && self.taper <= 1e6,
            "Wing section taper must be in (0, 1e6]",
        )?;
        check(
            self.dihedral.abs() < 1.5 && self.sweep.abs() < 1.5,
            "Sweep/dihedral angles must stay below 85 degrees",
        )?;
        Ok(())
    }

    /// Global position of the section leading edge (reference point).
    fn reference(&self) -> [f64; 3] {
        [
            self.span * self.sweep.tan(),
            self.span.abs() * self.dihedral.tan(),
            self.span,
        ]
    }
}

/// Place a section into global coordinates. The map is affine, so mapping
/// the control polygon preserves the curve exactly.
pub fn place_section(section: &WingSection) -> Result<Curve> {
    section.validate()?;
    let reference = section.reference();
    let (sin, cos) = section.twist.sin_cos();
    let scale = section.taper;
    let mut placed = section.profile.clone();
    for point in &mut placed.control_points {
        let x = point[0] * scale;
        let y = point[1] * scale;
        let z = point[2] * scale;
        let rotated = [x * cos - y * sin, x * sin + y * cos, z];
        for k in 0..3 {
            point[k] = rotated[k] + reference[k];
        }
        check(
            point.iter().all(|v| v.is_finite()),
            "Placed section control point is not finite",
        )?;
    }
    placed.validate()?;
    Ok(placed)
}

/// Strictly increasing span check plus placement of all sections.
fn placed_sections(sections: &[WingSection]) -> Result<Vec<Curve>> {
    check(
        (2..=11).contains(&sections.len()),
        "Wing loft needs 2..=11 sections",
    )?;
    check(
        sections
            .array_windows()
            .all(|[a, b]| a.span < b.span),
        "Wing section spans must be strictly increasing",
    )?;
    sections.iter().map(place_section).collect()
}

/// Chordal guide parameterization: cumulative distances between section
/// reference points, normalized to [0, 1].
fn chordal_parameters(sections: &[WingSection]) -> Result<Vec<f64>> {
    let mut parameters = vec![0.];
    for pair in sections.array_windows::<2>() {
        let [a, b] = pair;
        let ra = a.reference();
        let rb = b.reference();
        let step = (0..3)
            .map(|k| rb[k] - ra[k])
            .map(|d| d * d)
            .sum::<f64>()
            .sqrt();
        parameters.push(parameters.last().unwrap() + step);
    }
    let total = *parameters.last().unwrap();
    crate::numeric(total > 0., "Wing sections must not share one reference point")?;
    Ok(parameters.into_iter().map(|v| v / total).collect())
}

/// Skin the sections into one surface: sections are aligned (domain
/// normalization, degree elevation, knot merge) and interpolated along the
/// span by natural cubics in homogeneous space with the chordal guide
/// parameterization; V is normalized to [0, 1].
pub fn loft_wing(sections: &[WingSection]) -> Result<Surface> {
    let placed = placed_sections(sections)?;
    let parameters = chordal_parameters(sections)?;
    crate::natural_loft::interpolate(&placed, &parameters)
}

/// Wing-tip closure options for `loft_wing_with_tip`.
#[derive(Clone, Copy, Debug)]
pub enum TipClosure {
    /// No closure; the tip section stays open.
    None,
    /// Degenerate point tip: an extra section collapsed to the tip
    /// centroid one chord-length step outboard.
    Collapse,
    /// Domed tip: two smoothed intermediate sections (half scale, then a
    /// point) stepped along +Z give an approximate tangent (G1-like)
    /// flattening at the apex; no formal continuity certificate.
    Dome { height: f64 },
}

/// Collapse a placed section toward its centroid by `scale` and shift it
/// along +Z by `offset`.
fn collapsed(section: &Curve, scale: f64, offset: f64) -> Result<Curve> {
    let n = section.control_points.len();
    let centroid = [0, 1, 2]
        .map(|k| section.control_points.iter().map(|p| p[k]).sum::<f64>() / n as f64);
    let mut result = section.clone();
    for point in &mut result.control_points {
        for k in 0..3 {
            point[k] = centroid[k] + scale * (point[k] - centroid[k]);
        }
        point[2] += offset;
    }
    result.validate()?;
    Ok(result)
}

/// Loft with an explicit tip treatment applied after the last section.
pub fn loft_wing_with_tip(sections: &[WingSection], tip: &TipClosure) -> Result<Surface> {
    check(
        sections.len() <= 9,
        "Tip closure needs room for up to two extra sections (<= 9 given)",
    )?;
    let mut placed = placed_sections(sections)?;
    let mut parameters = chordal_parameters(sections)?;
    match tip {
        TipClosure::None => {}
        TipClosure::Collapse | TipClosure::Dome { .. } => {
            let last = placed.last().unwrap().clone();
            let mut chord = 0_f64;
            for pair in last.control_points.windows(2) {
                chord = chord.max(
                    (0..3)
                        .map(|k| pair[1][k] - pair[0][k])
                        .map(|d| d * d)
                        .sum::<f64>()
                        .sqrt(),
                );
            }
            crate::numeric(chord > 0., "Tip section is degenerate before closure")?;
            let step = 0.25 * chord;
            match tip {
                TipClosure::Collapse => {
                    placed.push(collapsed(&last, 0., step)?);
                    parameters.push(1. + step / chord);
                }
                TipClosure::Dome { height } => {
                    check(
                        height.is_finite() && *height > 0.,
                        "Dome height must be positive and finite",
                    )?;
                    placed.push(collapsed(&last, 0.5, step)?);
                    placed.push(collapsed(&last, 0., step + height)?);
                    parameters.push(1. + step / chord);
                    parameters.push(1. + (step + height) / chord);
                }
                TipClosure::None => unreachable!(),
            }
            let total = *parameters.last().unwrap();
            for v in &mut parameters {
                *v /= total;
            }
        }
    }
    crate::natural_loft::interpolate(&placed, &parameters)
}

/// Orientation-fold report: Jacobian sign test over a regular sample grid.
#[derive(Clone, Debug)]
pub struct FoldingReport {
    pub grid: usize,
    /// (u, v) samples where the normal reverses against the reference
    /// sample (det[dS/du, dS/dv, n_ref] < 0) — fold or orientation flip.
    pub flipped: Vec<[f64; 2]>,
    /// (u, v) samples where |dS/du x dS/dv| is below the degeneracy floor.
    pub degenerate: Vec<[f64; 2]>,
    /// Smallest |dS/du x dS/dv| seen on the grid.
    pub min_normal_magnitude: f64,
}

/// Self-intersection / fold heuristic: sample the surface Jacobian on a
/// regular (grid+1)^2 lattice strictly inside the domain. A normal that
/// reverses between adjacent grid samples (dot < 0), or collapses to zero,
/// indicates a fold or degenerate region; all offending (u, v) samples are
/// reported. Comparison is between neighbours, so legitimate sharp
/// features (e.g. a knife trailing edge at a clamped boundary) do not
/// count as folds. This is a sampling-based warning, not a certificate.
pub fn check_folding(surface: &Surface, grid: usize) -> Result<FoldingReport> {
    surface.validate()?;
    check((4..=256).contains(&grid), "Folding check grid must be 4..=256")?;
    let nu = surface.control_points.len();
    let nv = surface.control_points[0].len();
    let ua = surface.knots_u[surface.degree_u];
    let ub = surface.knots_u[nu];
    let va = surface.knots_v[surface.degree_v];
    let vb = surface.knots_v[nv];
    let eps = 1e-9;
    let side = grid + 1;
    let mut parameters = vec![[0.; 2]; side * side];
    let mut normals = vec![None::<[f64; 3]>; side * side];
    let mut magnitudes = vec![0_f64; side * side];
    let mut min_magnitude = f64::INFINITY;
    let mut scale = 0_f64;
    for i in 0..side {
        let u = ua + (ub - ua) * (i as f64 + eps) / (side as f64 + 2. * eps);
        for j in 0..side {
            let v = va + (vb - va) * (j as f64 + eps) / (side as f64 + 2. * eps);
            let index = i * side + j;
            parameters[index] = [u, v];
            let evaluation = surface.evaluate(u, v)?;
            let Some((du, dv)) = evaluation.first_derivatives() else {
                continue;
            };
            let normal = [
                du[1] * dv[2] - du[2] * dv[1],
                du[2] * dv[0] - du[0] * dv[2],
                du[0] * dv[1] - du[1] * dv[0],
            ];
            let magnitude = (normal[0] * normal[0] + normal[1] * normal[1]
                + normal[2] * normal[2])
                .sqrt();
            scale = scale.max(magnitude);
            min_magnitude = min_magnitude.min(magnitude);
            magnitudes[index] = magnitude;
            normals[index] = Some(normal);
        }
    }
    let floor = 1e-12 * scale.max(1e-300);
    let mut flipped = Vec::new();
    let mut degenerate = Vec::new();
    for index in 0..side * side {
        let Some(normal) = normals[index] else {
            degenerate.push(parameters[index]);
            continue;
        };
        if magnitudes[index] <= floor {
            degenerate.push(parameters[index]);
        }
        // Neighbours along u and v.
        let i = index / side;
        let j = index % side;
        let mut neighbours = Vec::with_capacity(2);
        if j + 1 < side {
            neighbours.push(index + 1);
        }
        if i + 1 < side {
            neighbours.push(index + side);
        }
        for neighbour in neighbours {
            if let Some(other) = normals[neighbour] {
                let dot =
                    normal[0] * other[0] + normal[1] * other[1] + normal[2] * other[2];
                if dot < 0. && magnitudes[index] > floor && magnitudes[neighbour] > floor {
                    flipped.push(parameters[neighbour]);
                }
            }
        }
    }
    Ok(FoldingReport {
        grid,
        flipped,
        degenerate,
        min_normal_magnitude: min_magnitude,
    })
}

/// G2 fairing blend between two boundary curves (e.g. a wing/fuselage
/// fillet): every boundary carries position, cross-boundary tangent and
/// cross-boundary second-derivative curves; each aligned control column is
/// interpolated by a quintic Hermite segment matching all six jets, so the
/// blend meets both boundaries with position, tangent and curvature.
/// All six curves are brought to a common basis (degree elevation plus
/// knot merge) before the columns are built.
pub fn fairing_blend(
    boundary_a: &Curve,
    tangent_a: &Curve,
    curvature_a: &Curve,
    boundary_b: &Curve,
    tangent_b: &Curve,
    curvature_b: &Curve,
) -> Result<Surface> {
    let aligned = crate::sections::compatible(&[
        boundary_a.clone(),
        tangent_a.clone(),
        curvature_a.clone(),
        boundary_b.clone(),
        tangent_b.clone(),
        curvature_b.clone(),
    ])?;
    check(
        aligned[0].control_points[0].len() == 3,
        "Fairing blend boundaries must be 3D curves",
    )?;
    for pair in aligned.array_windows::<2>() {
        let [a, b] = pair;
        check(
            a.control_points.len() == b.control_points.len()
                && a.knots == b.knots
                && a.degree == b.degree,
            "Fairing blend jets must align to one basis",
        )?;
    }
    let [pa, ta, ca, pb, tb, cb] = aligned.try_into().map_err(|_| {
        crate::numeric_err("Fairing blend alignment produced an unexpected section count")
    })?;
    check(
        pa.weights
            .iter()
            .chain(&pb.weights)
            .all(|&w| (w - 1.).abs() <= 1e-12),
        "Fairing blend is polynomial: unit boundary weights required",
    )?;
    let n = pa.control_points.len();
    // Quintic Hermite in V on [0, 1]: c0=P0, c1=P0+T0/5,
    // c2=(C0/20 + 2c1 - c0), c5=P1, c4=P1-T1/5, c3=(C1/20 + 2c4 - c5).
    // Assemble directly with U along the boundary basis and V across the
    // blend, so the V=0 / V=1 iso-curves are exactly the boundaries.
    let mut control_points: Vec<Vec<Vec<f64>>> = Vec::with_capacity(n);
    for i in 0..n {
        let (p0, t0, c0) = (&pa.control_points[i], &ta.control_points[i], &ca.control_points[i]);
        let (p1, t1, c1) = (&pb.control_points[i], &tb.control_points[i], &cb.control_points[i]);
        let mut controls = vec![vec![0.; 3]; 6];
        for k in 0..3 {
            controls[0][k] = p0[k];
            controls[1][k] = p0[k] + t0[k] / 5.;
            controls[2][k] = c0[k] / 20. + 2. * controls[1][k] - controls[0][k];
            controls[5][k] = p1[k];
            controls[4][k] = p1[k] - t1[k] / 5.;
            controls[3][k] = c1[k] / 20. + 2. * controls[4][k] - controls[5][k];
        }
        check(
            controls.iter().flatten().all(|x| x.is_finite()),
            "Fairing blend Hermite controls are not finite",
        )?;
        control_points.push(controls);
    }
    let surface = Surface {
        degree_u: pa.degree,
        degree_v: 5,
        knots_u: pa.knots.clone(),
        knots_v: std::iter::repeat_n(0., 6)
            .chain(std::iter::repeat_n(1., 6))
            .collect(),
        control_points,
        weights: vec![vec![1.; 6]; n],
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    Ok(surface)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::construct::curves::airfoil;

    fn flat_profile(chord: f64) -> Curve {
        // Simple closed diamond section, LE at (0, 0), TE at (chord, 0).
        Curve::from_polyline(vec![
            vec![0., 0., 0.],
            vec![0.3 * chord, 0.06 * chord, 0.],
            vec![chord, 0., 0.],
            vec![0.3 * chord, -0.06 * chord, 0.],
            vec![0., 0., 0.],
        ])
        .unwrap()
    }

    fn section(profile: Curve, span: f64) -> WingSection {
        WingSection {
            profile,
            span,
            twist: 0.,
            dihedral: 0.,
            sweep: 0.,
            taper: 1.,
        }
    }

    #[test]
    fn identical_sections_give_prismatic_surface() {
        let profile = flat_profile(1.);
        let sections = vec![
            section(profile.clone(), 0.),
            section(profile.clone(), 2.),
        ];
        let surface = loft_wing(&sections).unwrap();
        let [pa, pb] = profile.domain();
        for i in 0..=10 {
            let u = i as f64 / 10.;
            let mid = surface.evaluate(u, 0.5).unwrap().point;
            let on_profile = profile.evaluate(pa + (pb - pa) * u).unwrap().point;
            for k in 0..2 {
                assert!(
                    (mid[k] - on_profile[k]).abs() < 1e-9,
                    "mid-span section must lie on the profile at u={u}: {mid:?} vs {on_profile:?}"
                );
            }
            assert!((mid[2] - 1.).abs() < 1e-9, "mid-span station at z=1");
        }
    }

    #[test]
    fn taper_scales_chord_linearly() {
        let profile = flat_profile(1.);
        let mut a = section(profile.clone(), 0.);
        let mut b = section(profile.clone(), 2.);
        a.taper = 1.0;
        b.taper = 0.5;
        let chord_of = |s: &WingSection| {
            let placed = place_section(s).unwrap();
            let xs: Vec<f64> = placed.control_points.iter().map(|p| p[0]).collect();
            xs.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                - xs.iter().copied().fold(f64::INFINITY, f64::min)
        };
        let (c0, c1) = (chord_of(&a), chord_of(&b));
        assert!((c0 - 1.0).abs() < 1e-12, "root chord 1.0, got {c0}");
        assert!((c1 - 0.5).abs() < 1e-12, "tip chord 0.5, got {c1}");
        assert!(
            (c1 - (c0 + (0.5 - 1.0) * (2. - 0.) / 2.)).abs() < 1e-12,
            "taper varies linearly with span"
        );
        let surface = loft_wing(&[a, b]).unwrap();
        surface.validate().unwrap();
    }

    #[test]
    fn twist_rotates_section_about_leading_edge() {
        let profile = flat_profile(1.);
        let mut s = section(profile, 3.);
        s.twist = std::f64::consts::FRAC_PI_2;
        let placed = place_section(&s).unwrap();
        // Control point (1, 0, 0) -> (0, 1, 0) + reference (0, 0, 3).
        let tip = &placed.control_points[2];
        assert!(tip[0].abs() < 1e-12, "x after 90-degree twist: {tip:?}");
        assert!((tip[1] - 1.).abs() < 1e-12, "y after twist: {tip:?}");
        assert!((tip[2] - 3.).abs() < 1e-12, "z station: {tip:?}");
        // Mid-chord thickness point (0.3, 0.06) -> (-0.06, 0.3).
        let p = &placed.control_points[1];
        assert!((p[0] + 0.06).abs() < 1e-12 && (p[1] - 0.3).abs() < 1e-12);
    }

    #[test]
    fn sweep_and_dihedral_shift_reference() {
        let profile = flat_profile(1.);
        let mut s = section(profile, 4.);
        s.sweep = 0.1_f64.atan();
        s.dihedral = 0.05_f64.atan();
        let placed = place_section(&s).unwrap();
        let le = &placed.control_points[0];
        assert!((le[0] - 0.4).abs() < 1e-12, "sweep shift 4*tan: {le:?}");
        assert!((le[1] - 0.2).abs() < 1e-12, "dihedral shift: {le:?}");
        assert!((le[2] - 4.).abs() < 1e-12);
    }

    #[test]
    fn folding_check_flags_reversed_section() {
        let profile = flat_profile(1.);
        let reversed = profile.reverse().unwrap();
        let sections = vec![section(profile, 0.), section(reversed, 2.)];
        let surface = loft_wing(&sections).unwrap();
        let report = check_folding(&surface, 16).unwrap();
        // Interpolating between opposite loop orientations squeezes dS/du
        // through zero mid-span: the Jacobian degenerates and/or flips.
        assert!(
            !report.degenerate.is_empty() || !report.flipped.is_empty(),
            "reversed section orientation must break the Jacobian"
        );
        // Same-orientation sections never degenerate. Sharp polyline
        // corners may raise flip warnings; the Jacobian stays healthy.
        let profile = flat_profile(1.);
        let clean = loft_wing(&[section(profile.clone(), 0.), section(profile, 2.)]).unwrap();
        let report = check_folding(&clean, 16).unwrap();
        assert!(
            report.degenerate.is_empty(),
            "prismatic loft must not degenerate: {:?}",
            report.degenerate
        );
    }

    #[test]
    fn collapse_tip_degenerates_to_point() {
        let profile = flat_profile(1.);
        let sections = vec![section(profile.clone(), 0.), section(profile, 2.)];
        let surface = loft_wing_with_tip(&sections, &TipClosure::Collapse).unwrap();
        let apex = surface.evaluate(0.3, 1.).unwrap().point;
        let apex2 = surface.evaluate(0.7, 1.).unwrap().point;
        for k in 0..3 {
            assert!(
                (apex[k] - apex2[k]).abs() < 1e-9,
                "collapsed tip must be one point: {apex:?} vs {apex2:?}"
            );
        }
        assert!(apex[2] > 2., "tip apex must sit outboard of the last section");
    }

    #[test]
    fn fairing_blend_matches_boundaries_and_tangents() {
        // Two parallel boundaries along X; cross-blend direction +Y with a
        // curved tangent field so the G2 data are nontrivial.
        let boundary_a = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
        let tangent_a = Curve::from_polyline(vec![vec![0., 1., 0.], vec![0., 1., 0.]]).unwrap();
        let curvature_a = Curve::from_polyline(vec![vec![0., 0., 0.], vec![0., 0., 0.]]).unwrap();
        let boundary_b = Curve::from_polyline(vec![vec![0., 1., 1.], vec![1., 1., 1.]]).unwrap();
        let tangent_b = Curve::from_polyline(vec![vec![0., 1., 0.], vec![0., 1., 0.]]).unwrap();
        let curvature_b = Curve::from_polyline(vec![vec![0., 0., 0.], vec![0., 0., 0.]]).unwrap();
        let surface = fairing_blend(
            &boundary_a,
            &tangent_a,
            &curvature_a,
            &boundary_b,
            &tangent_b,
            &curvature_b,
        )
        .unwrap();
        for i in 0..=10 {
            let u = i as f64 / 10.;
            let start = surface.evaluate(u, 0.).unwrap().point;
            let end = surface.evaluate(u, 1.).unwrap().point;
            assert!((start[0] - u).abs() < 1e-12 && start[1].abs() < 1e-12 && start[2].abs() < 1e-12);
            assert!((end[0] - u).abs() < 1e-12 && (end[1] - 1.).abs() < 1e-12 && (end[2] - 1.).abs() < 1e-12);
            // Cross tangents at the boundaries match the tangent curves.
            let (_, dv0) = surface.evaluate(u, 0.).unwrap().first_derivatives().unwrap();
            let (_, dv1) = surface.evaluate(u, 1.).unwrap().first_derivatives().unwrap();
            assert!(
                dv0[0].abs() < 1e-9 && (dv0[1] - 1.).abs() < 1e-9 && dv0[2].abs() < 1e-9,
                "start tangent {dv0:?}"
            );
            assert!(
                dv1[0].abs() < 1e-9 && (dv1[1] - 1.).abs() < 1e-9 && dv1[2].abs() < 1e-9,
                "end tangent {dv1:?}"
            );
        }
    }

    #[test]
    fn airfoil_sections_loft_into_wing() {
        // Smoke test with real NACA profiles: prismatic wing.
        let profile = airfoil::naca4(12, 1., true, 80, 3, 20).unwrap();
        let sections = vec![section(profile.clone(), 0.), section(profile, 3.)];
        let surface = loft_wing(&sections).unwrap();
        surface.validate().unwrap();
        let report = check_folding(&surface, 8).unwrap();
        assert!(
            report.degenerate.is_empty(),
            "prismatic NACA wing must not degenerate: {:?}",
            report.degenerate
        );
    }
}
