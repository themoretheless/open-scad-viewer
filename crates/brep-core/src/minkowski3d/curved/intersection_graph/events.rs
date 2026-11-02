//! All original trim-boundary events bound to actual root components.
use super::*;
use nurbs_core::{
    curve_surface_agreement,
    interval_eval::{self, Interval as I},
};
impl Build<'_> {
    fn query(
        &mut self,
        side: usize,
        edge: usize,
        face: usize,
    ) -> Result<Arc<crate::face_crossings::Report>> {
        let key = (side, edge, face);
        if let Some(r) = self.queries.get(&key) {
            return Ok(r.clone());
        }
        let r = crate::face_crossings::inspect(
            self.models[side],
            edge,
            self.models[1 - side],
            face,
            1e-8,
            self.cells()?,
        )?;
        self.work.cells += r.cells;
        self.work.queries += 1;
        if !r.complete
            || !r.native_complete
            || r.roots
                .iter()
                .any(|r| !matches!(r.location, Location::Inside | Location::Outside))
        {
            return Err(fail(
                "trim graph requires complete isolated boundary roots without seam/seam or tangent ambiguity",
            ));
        }
        let curve = &self.models[side].edges[edge].curve;
        let d = curve.domain();
        if r.roots
            .iter()
            .any(|r| r.root.parameter_box[0][0] <= d[0] || r.root.parameter_box[0][1] >= d[1])
            || r.roots
                .windows(2)
                .any(|p| p[0].root.parameter_box[0][1] >= p[1].root.parameter_box[0][0])
        {
            return Err(fail(
                "original boundary roots need strict interior curve rank separation",
            ));
        }
        let roots = self.queries.values().map(|r| r.roots.len()).sum::<usize>() + r.roots.len();
        if roots > self.limits.max_roots {
            return Err(fail("trim graph original root census exceeds allowance"));
        }
        let r = Arc::new(r);
        self.queries.insert(key, r.clone());
        Ok(r)
    }
}
fn pcurve_box(
    c: &crate::Curve,
    p: &crate::Curve,
    reversed: bool,
    t: [f64; 2],
) -> Result<[[f64; 2]; 2]> {
    if c.periodic
        || p.periodic
        || c.weights.iter().chain(&p.weights).any(|w| *w <= 0.)
        || p.control_points.len() > 256
    {
        return Err(fail(
            "root binding needs bounded positive nonperiodic original clocks",
        ));
    }
    let cd = c.domain();
    let pd = p.domain();
    let mut x = I::new(t[0], t[1])?
        .sub(I::point(cd[0]))?
        .div(I::point(cd[1]).sub(I::point(cd[0]))?)?;
    if reversed {
        x = I::point(1.).sub(x)?;
    }
    let parameter = I::point(pd[0])
        .add(x.mul(I::point(pd[1]).sub(I::point(pd[0]))?)?)?
        .intersect(pd[0], pd[1])?;
    let b = interval_eval::evaluate_interval(p, parameter)?;
    let mut uv = [[0.; 2]; 2];
    for k in 0..2 {
        // Independent positive-weight full control hull preserves exact
        // constant seam coordinates at natural rectangle boundaries.
        let lo = p
            .control_points
            .iter()
            .map(|p| p[k])
            .min_by(f64::total_cmp)
            .unwrap();
        let hi = p
            .control_points
            .iter()
            .map(|p| p[k])
            .max_by(f64::total_cmp)
            .unwrap();
        let bound = b[k].intersect(lo, hi)?;
        uv[k] = [bound.lo, bound.hi];
    }
    Ok(uv)
}
pub(super) fn collect(
    build: &mut Build<'_>,
    faces: [usize; 2],
    owner: &Physical,
) -> Result<Vec<OriginalGraphEvent>> {
    let mut events = Vec::new();
    for side in 0..2 {
        let model = build.models[side];
        let f = &model.faces[faces[side]];
        let mut seen = BTreeSet::new();
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for c in &model.loops[wire].coedges {
                if !seen.insert(c.edge) || model.edges[c.edge].degenerate {
                    return Err(fail(
                        "trim graph needs distinct noncollapsed original boundary edges",
                    ));
                }
                let edge = &model.edges[c.edge];
                let allowance = build
                    .limits
                    .max_exact_work
                    .checked_sub(build.work.exact_work)
                    .filter(|&n| n > 0)
                    .ok_or_else(|| fail("trim graph exact boundary work exhausted"))?;
                let exact = curve_surface_agreement::verify_exact(
                    &edge.curve,
                    &c.pcurve,
                    &f.surface,
                    c.reversed,
                    allowance,
                )?
                .ok_or_else(|| fail("exact original edge/carrier identity is unproved"))?;
                build.work.exact_work += exact.work_used;
                if exact.outcome != cad_predicates::BezierIdentity::Equal {
                    return Err(fail("original trim edge is not exactly the carrier image"));
                }
                let report = build.query(side, c.edge, faces[1 - side])?;
                for (rank, r) in report.roots.iter().enumerate() {
                    let uv =
                        pcurve_box(&edge.curve, &c.pcurve, c.reversed, r.root.parameter_box[0])?;
                    let b = if side == 0 {
                        [
                            uv[0],
                            uv[1],
                            r.root.parameter_box[1],
                            r.root.parameter_box[2],
                        ]
                    } else {
                        [
                            r.root.parameter_box[1],
                            r.root.parameter_box[2],
                            uv[0],
                            uv[1],
                        ]
                    };
                    let mut found = Vec::new();
                    for (branch, meta) in owner.branches().iter().enumerate() {
                        let charts = meta
                            .charts
                            .iter()
                            .copied()
                            .filter(|&i| {
                                let root = owner
                                    .canonical(image(&owner.native.owner().owner().fibers()[i]));
                                !(0..4).any(|k| root[k][1] < b[k][0] || b[k][1] < root[k][0])
                            })
                            .collect::<Vec<_>>();
                        if !charts.is_empty() {
                            found.push((branch, charts));
                        }
                    }
                    // Complete original coverage places the actual root in at
                    // least one chart. Exact box separation excludes every
                    // OTHER component, even if this enclosure crosses chart
                    // boundaries. No seed or nearest-chart match is used.
                    if found.len() != 1 {
                        return Err(fail(&format!(
                            "original edge {}/{}, rank {} has {} possible complete root components",
                            side,
                            c.edge,
                            rank,
                            found.len()
                        )));
                    }
                    let (branch, charts) = found.remove(0);
                    let k = axis(owner.branches()[branch].free_axis);
                    let domain = owner.branches()[branch].domain;
                    let station = [b[k][0].max(domain[0]), b[k][1].min(domain[1])];
                    if station[0] > station[1] {
                        return Err(fail(
                            "original trim station misses its owned complete branch",
                        ));
                    }
                    if !matches!(r.face_crossing_sign, -1 | 1) {
                        return Err(fail(
                            "original seam crossing has no proved transverse orientation",
                        ));
                    }
                    events.push(OriginalGraphEvent {
                        key: OriginalRootKey {
                            operand: side,
                            edge: c.edge,
                            other_face: faces[1 - side],
                            rank,
                        },
                        branch,
                        charts,
                        parameter_box: b,
                        station,
                        inside_other: r.location == Location::Inside,
                        crossing_sign: r.face_crossing_sign,
                        edge_parameter: r.root.parameter_box[0],
                        world_box: r.root.position_bounds,
                    });
                }
            }
        }
    }
    Ok(events)
}
