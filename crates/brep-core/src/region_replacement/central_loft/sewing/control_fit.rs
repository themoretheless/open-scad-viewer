//! Bounded proposal, not a certificate: all G2/pole inspections are repeated.
//! Positive unchanged rational weights bound the whole-surface displacement by
//! the largest literal control displacement, before exact image reclocking.
use super::*;
use nurbs_core::interval_eval::Interval as I;
pub(super) struct Proposal {
    pub surface: Surface,
    pub displacement_upper_mm: f64,
}
fn region(c: &CentralLoftCandidate<'_, '_, '_>, i: usize) -> Result<(usize, usize, usize)> {
    let s = &c.surface;
    let pc = &c.pcurves[i];
    let lo = pc.control_points[0][0].min(pc.control_points.last().unwrap()[0]);
    let hi = pc.control_points[0][0].max(pc.control_points.last().unwrap()[0]);
    let p = s.degree_u;
    let n = s.control_points.len();
    let first = (p..n)
        .find(|&u| s.knots_u[u] == lo && s.knots_u[u] < s.knots_u[u + 1])
        .ok_or_else(|| refuse("Missing original fit start span"))?
        - p;
    let last = (p..n)
        .find(|&u| s.knots_u[u + 1] == hi && s.knots_u[u] < s.knots_u[u + 1])
        .ok_or_else(|| refuse("Missing original fit final span"))?;
    let v = if pc.control_points[0][1] == s.knots_v[s.degree_v] {
        0
    } else {
        s.degree_v
    };
    Ok((first, last, v))
}
fn plane(surface: &Surface) -> Result<(usize, f64)> {
    let axes = (0..3)
        .filter(|&k| {
            surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[k] == surface.control_points[0][0][k])
        })
        .collect::<Vec<_>>();
    require(
        axes.len() == 1,
        "Fit requires a literal original coordinate plane",
    )?;
    Ok((axes[0], surface.control_points[0][0][axes[0]]))
}
pub(super) fn prepare(c: &CentralLoftCandidate<'_, '_, '_>) -> Result<Proposal> {
    let mut surface = c.surface.clone();
    let source = c.body.source();
    let corner = source.vertices[c.strips.plan.vertex].point;
    let q = surface.degree_v;
    require(q == 5, "Bound the six original central rows")?;
    let mut near_box = c.stock;
    for strip in &c.strips.strips {
        let original = &source.edges[strip.source_edge];
        let other = *original
            .vertices
            .iter()
            .find(|&&v| v != c.strips.plan.vertex)
            .unwrap();
        let axes = (0..3)
            .filter(|&k| source.vertices[other].point[k] != corner[k])
            .collect::<Vec<_>>();
        require(axes.len() == 1, "Fit edge is not axis aligned")?;
        let axis = axes[0];
        let fixed = strip.cutters[0].faces[strip.cutter_faces[0]]
            .surface
            .control_points[0][0][axis];
        near_box[axis] = [corner[axis].min(fixed), corner[axis].max(fixed)];
        let near = usize::from(strip.end_order[0] != 0);
        let old = &strip.patch.ends[near].curve;
        let ids = c
            .curves
            .iter()
            .enumerate()
            .filter_map(|(i, curve)| {
                if i % 2 == 0 && (curve == old || curve.reverse().ok().as_ref() == Some(old)) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        require(ids.len() == 1, "Fit lost original strip/rim ownership")?;
        let i = ids[0];
        let (first, last, v) = region(c, i)?;
        for u in first..=last {
            let delta = fixed - surface.control_points[u][v][axis];
            for j in 0..3 {
                let row = if v == 0 { j } else { q - j };
                surface.control_points[u][row][axis] += delta;
            }
            surface.control_points[u][v][axis] = fixed;
        }
        // Original support faces identify the exact zero-height profile jets.
        let old_first = &c.surface.control_points[first][v];
        let side_at_first = if old_first.as_slice() == topology::endpoint(old, 0).as_slice() {
            0
        } else {
            require(
                old_first.as_slice() == topology::endpoint(old, 1).as_slice(),
                "Profile fit endpoint lost its literal owner",
            )?;
            1
        };
        for end in 0..2 {
            let side = if end == 0 {
                side_at_first
            } else {
                1 - side_at_first
            };
            let (height, value) = plane(&source.faces[strip.support_faces[side]].surface)?;
            for k in 0..3 {
                let u = if end == 0 { first + k } else { last - k };
                for j in 0..3 {
                    let row = if v == 0 { j } else { q - j };
                    surface.control_points[u][row][height] = value;
                }
            }
        }
    }
    for i in [1, 3, 5] {
        let (first, last, v) = region(c, i)?;
        let (height, value) = plane(&c.originals[i].0)?;
        for u in first..=last {
            for j in 0..3 {
                surface.control_points[u][if v == 0 { j } else { q - j }][height] = value;
            }
        }
    }
    for p in surface.control_points.iter_mut().flatten() {
        for k in 0..3 {
            p[k] = p[k].clamp(near_box[k][0], near_box[k][1]);
        }
    }
    for row in [0, surface.control_points.len() - 1] {
        let point = surface.control_points[row][0].clone();
        surface.control_points[row].fill(point);
    }
    let mut displacement = 0_f64;
    for (a, b) in c
        .surface
        .control_points
        .iter()
        .flatten()
        .zip(surface.control_points.iter().flatten())
    {
        if a == b {
            continue;
        }
        let mut squared = I::point(0.);
        for k in 0..3 {
            let d = I::point(a[k]).sub(I::point(b[k]))?;
            let upper = d.lo.abs().max(d.hi.abs());
            squared = squared.add(I::point(upper).mul(I::point(upper))?)?;
        }
        displacement = displacement.max(squared.hi.sqrt().next_up());
    }
    require(
        displacement <= c.options.tolerances[0] / 64.,
        &format!(
            "Exact sewing control proposal exceeds its unchanged displacement allowance: upper={displacement}"
        ),
    )?;
    require(
        surface.weights == c.surface.weights
            && surface.knots_u == c.surface.knots_u
            && surface.knots_v == c.surface.knots_v,
        "Control fitting changed rational weights or source basis",
    )?;
    surface.validate()?;
    Ok(Proposal {
        surface,
        displacement_upper_mm: displacement,
    })
}
