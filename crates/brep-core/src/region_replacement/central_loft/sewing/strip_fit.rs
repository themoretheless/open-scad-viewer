//! Whole-image deviation for replacing the original stationary strip by its
//! literal shared near profile. No sample or endpoint-only fit is admitted.
use super::*;
use nurbs_core::interval_eval::Interval as I;
#[derive(Clone, Debug)]
pub struct StripFitReport {
    pub displacement_upper_mm: f64,
    pub cells: usize,
}
fn linear(pc: &Curve) -> bool {
    pc.degree == 1 && pc.control_points.len() == 2 && pc.weights[0] == pc.weights[1] && !pc.periodic
}
fn verify(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reverse: bool,
    range: [f64; 2],
    delta: f64,
    cells: &mut usize,
    max: usize,
) -> Result<()> {
    require(*cells < max, "Strip deviation work exhausted")?;
    let r = agreement::verify_on(c, p, s, reverse, range, delta, (max - *cells).min(10000))?;
    *cells += r.cells;
    require(
        r.status == agreement::Status::WithinTolerance,
        "Whole original strip deviation remains unproven",
    )
}
pub(super) fn inspect(
    c: &CentralLoftCandidate<'_, '_, '_>,
    clocks: &reclock::Reclocked,
    index: usize,
    j: usize,
    axis: usize,
    near_plane: f64,
    far_plane: f64,
    max: usize,
) -> Result<StripFitReport> {
    let strip = &c.strips.strips[j];
    let original = &c.originals[index].0;
    let pc = &c.originals[index].1;
    require(
        linear(pc),
        "Strip fit needs the actual affine source near section",
    )?;
    // A V-dependent weight common to every U row makes the effective U
    // blend independent of station. This exact structural condition is needed
    // for the full transverse variation bound below.
    require(
        original.weights.iter().all(|r| r == &original.weights[0]),
        "Strip weights are not station separable",
    )?;
    let u_domain = [
        original.knots_u[original.degree_u],
        original.knots_u[original.control_points.len()],
    ];
    let ends = [pc.control_points[0][0], pc.control_points[1][0]];
    require(
        [ends[0].min(ends[1]), ends[0].max(ends[1])] == u_domain,
        "Near section does not cover the original profile",
    )?;
    let positive =
        original.control_points[0].last().unwrap()[axis] > original.control_points[0][0][axis];
    require(
        original.control_points.iter().all(|row| {
            row.windows(2).all(|p| {
                if positive {
                    p[0][axis] < p[1][axis]
                } else {
                    p[0][axis] > p[1][axis]
                }
            })
        }),
        "Original axial station is not strictly monotone",
    )?;
    let delta = c.options.tolerances[0] / 64.;
    let mut cells = 0;
    let central_pc = &c.pcurves[index];
    let a = central_pc.control_points[0][0];
    let b = central_pc.control_points[1][0];
    let first = clocks
        .old_breaks
        .iter()
        .position(|&x| x == a.min(b))
        .ok_or_else(|| refuse("Missing fitted seam start"))?;
    let last = clocks
        .old_breaks
        .iter()
        .position(|&x| x == a.max(b))
        .ok_or_else(|| refuse("Missing fitted seam end"))?;
    let q = if central_pc.control_points[0][1] == clocks.surface.knots_v[clocks.surface.degree_v] {
        0
    } else {
        clocks.surface.degree_v
    };
    let pd = pc.domain();
    let parameter = |u: f64| {
        if u == a {
            pd[0]
        } else if u == b {
            pd[1]
        } else {
            pd[0] + ((u - a) / (b - a)) * (pd[1] - pd[0])
        }
    };
    let p = clocks.surface.degree_u;
    for span in first..last {
        let curve = Curve {
            degree: p,
            knots: [vec![0.; p + 1], vec![1.; p + 1]].concat(),
            control_points: (span * p..=span * p + p)
                .map(|i| clocks.surface.control_points[i][q].clone())
                .collect(),
            weights: (span * p..=span * p + p)
                .map(|i| clocks.surface.weights[i][q])
                .collect(),
            periodic: false,
        };
        require(
            curve.control_points.iter().all(|x| x[axis] == near_plane),
            "New strip near section is not exactly planar",
        )?;
        let x = parameter(clocks.old_breaks[span]);
        let y = parameter(clocks.old_breaks[span + 1]);
        verify(
            &curve,
            pc,
            original,
            x > y,
            [x.min(y), x.max(y)],
            delta,
            &mut cells,
            max,
        )?;
    }
    let far_index = usize::from(strip.end_order[1] != 0);
    let far = &strip.patch.ends[far_index];
    require(
        linear(&far.skin_pcurve),
        "Strip far section must retain its affine source chart",
    )?;
    let f = &far.skin_pcurve;
    let fends = [f.control_points[0][0], f.control_points[1][0]];
    require(
        [fends[0].min(fends[1]), fends[0].max(fends[1])] == u_domain,
        "Far section does not cover the original profile",
    )?;
    // Both V sections must bound a nonempty original strip at every U.
    for u in u_domain {
        // The query is literally one of the two U endpoints. Read its
        // authored V ordinate; rounded interpolation is not a sign proof.
        let station = |p: &Curve| {
            if p.control_points[0][0] == u {
                p.control_points[0][1]
            } else {
                p.control_points[1][1]
            }
        };
        require(
            if positive == (far_plane > near_plane) {
                station(pc) < station(f)
            } else {
                station(pc) > station(f)
            },
            "Original strip end sections cross",
        )?;
    }
    verify(
        &far.curve,
        &far.skin_pcurve,
        original,
        false,
        far.skin_pcurve.domain(),
        delta,
        &mut cells,
        max,
    )?;
    let cutter = &strip.cutters[1].faces[strip.cutter_faces[1]].surface;
    require(
        cutter
            .control_points
            .iter()
            .flatten()
            .all(|p| p[axis] == far_plane),
        "Far support is not the actual exact cutter",
    )?;
    verify(
        &far.curve,
        &far.end_face_pcurve,
        cutter,
        false,
        far.end_face_pcurve.domain(),
        delta,
        &mut cells,
        max,
    )?;
    let mut squared = I::point(0.);
    for k in (0..3).filter(|&k| k != axis) {
        let mut width = 0_f64;
        for row in &original.control_points {
            let lo = row.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min);
            let hi = row.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max);
            width = width.max(I::point(hi).sub(I::point(lo))?.hi);
        }
        squared = squared.add(I::point(width).mul(I::point(width))?)?;
    }
    let transverse = I::point(delta).add(I::point(squared.hi.sqrt().next_up()))?;
    let axial = I::point(delta).mul(I::point(2.))?;
    let upper = transverse
        .mul(transverse)?
        .add(axial.mul(axial)?)?
        .hi
        .sqrt()
        .next_up();
    // For every profile U both axial images are continuous monotone intervals;
    // endpoint gaps are <=2*delta. Clamping the axial coordinate gives a
    // correspondence in both directions, with the transverse bound above.
    require(
        upper <= c.options.tolerances[0],
        "Full strip image displacement exceeds the unchanged tolerance",
    )?;
    Ok(StripFitReport {
        displacement_upper_mm: upper,
        cells,
    })
}
