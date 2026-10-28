//! One-sheeted projected disk neighborhoods of complete owned polar stars.
//! The proper-cover/height-order argument is documented in the qualification
//! theorem; no caller-written topology, orientation or smoothness flag admits it.
use super::star_boundary::{self as boundary, Meridian};
use super::*;
use nurbs_core::interval_eval::Interval as I;
pub struct GraphLimits {
    pub charts: ProjectedChartLimits,
    pub boundary_steps: usize,
}
/// A one-sheeted C0 graph disk, differentiable at its pole with the privately
/// proved common tangent plane. Ordinary seams away from that point can remain
/// C0; no whole-neighborhood C1 or pole curvature/G2 claim is made here.
pub struct PoleGraphStar {
    vertex: usize,
    axis: usize,
    point: [f64; 3],
    radius: f64,
    faces: Vec<usize>,
}
impl PoleGraphStar {
    pub fn vertex(&self) -> usize {
        self.vertex
    }
    pub fn dropped_axis(&self) -> usize {
        self.axis
    }
    pub fn point(&self) -> [f64; 3] {
        self.point
    }
    /// Open projected disk over which THIS local body star is one sheet.
    /// Remote portions of the body may project onto the same disk.
    pub fn projected_radius_lower(&self) -> f64 {
        self.radius
    }
    pub fn faces(&self) -> &[usize] {
        &self.faces
    }
}
pub struct PoleGraphAtlas<'a, E: Carrier> {
    charts: ProjectedChartAtlas<'a, E>,
    stars: Vec<PoleGraphStar>,
}
impl<'a, E: Carrier> PoleGraphAtlas<'a, E> {
    pub fn charts(&self) -> &ProjectedChartAtlas<'a, E> {
        &self.charts
    }
    pub fn stars(&self) -> &[PoleGraphStar] {
        &self.stars
    }
}
pub struct GraphReport<'a, E: Carrier> {
    pub certificate: Option<PoleGraphAtlas<'a, E>>,
    pub exact_work: u64,
    pub source_cells: usize,
    pub projection_columns: usize,
    pub projected_cells: usize,
    pub boundary_steps: usize,
    pub completed_stars: usize,
    pub uncertain_vertex: Option<usize>,
    pub reason: &'static str,
}
pub fn qualify_body_pole_graphs<E: Carrier>(
    body: &Body<E>,
    limits: GraphLimits,
) -> Result<GraphReport<'_, E>> {
    if !(1..=100000).contains(&limits.boundary_steps) {
        return Err(Error::new(
            "BREP_SOURCE_POLE_GRAPH",
            "Choose bounded star-boundary work",
        ));
    }
    let r = qualify_body_projected_chart_atlas(body, limits.charts)?;
    let mut out = GraphReport {
        certificate: None,
        exact_work: r.exact_work,
        source_cells: r.source_cells,
        projection_columns: r.projection_columns,
        projected_cells: r.projected_cells,
        boundary_steps: 0,
        completed_stars: 0,
        uncertain_vertex: None,
        reason: r.reason,
    };
    let Some(charts) = r.certificate else {
        return Ok(out);
    };
    let shell = body.geometry().shell();
    let mut uses = BTreeMap::new();
    for (i, pair) in shell.uses().iter().enumerate() {
        for (side, a) in pair.iter().enumerate() {
            uses.insert((a.face, a.wire, a.edge), (i, side));
        }
    }
    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for (i, (a, _)) in charts.charts().iter().enumerate() {
        let v = shell.vertices()[a.face][a.wire][a.edge][0];
        groups.entry(v).or_default().push(i);
    }
    let mut stars = Vec::new();
    for (vertex, indices) in groups {
        out.uncertain_vertex = Some(vertex);
        let first = &charts.charts()[indices[0]].1;
        let axis = first.dropped_axis();
        let point = shell.poles()[indices[0]].1.point();
        let mut separation = f64::INFINITY;
        let mut meridians = BTreeMap::<usize, ([Option<Meridian>; 2], [Option<usize>; 2])>::new();
        let mut faces = BTreeSet::new();
        for &index in &indices {
            if out.boundary_steps == limits.boundary_steps {
                out.reason = "pole-graph-boundary-step-limit";
                return Ok(out);
            }
            out.boundary_steps += 1;
            let (a, chart) = &charts.charts()[index];
            if !faces.insert(a.face)
                || shell.faces()[a.face].len() != 1
                || shell.faces()[a.face][0].edges().len() != 4
                || shell.regions().is_none_or(|r| r[a.face].loops().len() != 1)
                || chart.dropped_axis() != axis
            {
                out.reason = "pole-graph-natural-disk-star-unproven";
                return Ok(out);
            }
            separation = separation.min(chart.outer_radius_lower());
            let mut sides = BTreeSet::new();
            for (edge, f) in shell.faces()[a.face][0].edges().iter().enumerate() {
                let Some(side) = boundary::natural_side(f) else {
                    out.reason = "pole-graph-full-natural-side-unproven";
                    return Ok(out);
                };
                if !sides.insert(side) {
                    out.reason = "pole-graph-duplicate-natural-side";
                    return Ok(out);
                }
                if side.axis == boundary::pole_side(chart.boundary()).axis {
                    continue;
                }
                let Some(m) = Meridian::from_fragment(f, chart.boundary()) else {
                    out.reason = "pole-graph-meridian-unproven";
                    return Ok(out);
                };
                if shell.vertices()[a.face][0][edge][m.pole_end()] != vertex
                    || shell.vertices()[a.face][0][edge][1 - m.pole_end()] == vertex
                {
                    out.reason = "pole-graph-meridian-vertex-unproven";
                    return Ok(out);
                }
                let Some(&(global, slot)) = uses.get(&(a.face, 0, edge)) else {
                    out.reason = "pole-graph-meridian-ownership-unproven";
                    return Ok(out);
                };
                let owned = shell.edges()[global].fragment(slot);
                if owned.surface() != f.surface()
                    || owned.curve() != f.curve()
                    || owned.parameter_bounds() != f.parameter_bounds()
                {
                    out.reason = "pole-graph-original-carrier-unproven";
                    return Ok(out);
                }
                let entry = meridians
                    .entry(global)
                    .or_insert(([None, None], [None, None]));
                entry.0[slot] = Some(m);
                entry.1[slot] = Some(index);
            }
            if sides.len() != 4 {
                out.reason = "pole-graph-natural-rectangle-unproven";
                return Ok(out);
            }
        }
        let Some(link) = body
            .geometry()
            .topology()
            .links
            .iter()
            .find(|l| l.vertex == vertex)
        else {
            out.reason = "pole-graph-owned-link-unproven";
            return Ok(out);
        };
        let expected = link.cycle.iter().map(|n| n.edge).collect::<BTreeSet<_>>();
        if expected != meridians.keys().copied().collect() || meridians.len() != indices.len() {
            out.reason = "pole-graph-incomplete-disk-link";
            return Ok(out);
        }
        for (edge, (maps, owners)) in meridians {
            let ([Some(a), Some(b)], [Some(ia), Some(ib)]) = (maps, owners) else {
                out.reason = "pole-graph-unpaired-star-meridian";
                return Ok(out);
            };
            let ms = [a, b];
            let owners = [ia, ib];
            for from in 0..2 {
                if out.boundary_steps == limits.boundary_steps {
                    out.reason = "pole-graph-boundary-step-limit";
                    return Ok(out);
                }
                out.boundary_steps += 1;
                let source = &charts.charts()[owners[from]].1;
                let target = &charts.charts()[owners[1 - from]].1;
                let cut = boundary::transported_cut(
                    &shell.edges()[edge],
                    from,
                    ms,
                    source.inward_extent(),
                )?;
                let lower = cut.lo.min(target.inward_extent());
                let bound = I::point(target.radial_distance_factor_lower())
                    .mul(I::point(lower))?
                    .lo;
                if bound <= 0. {
                    out.reason = "pole-graph-positive-seam-tail-separation-unproven";
                    return Ok(out);
                }
                separation = separation.min(bound);
            }
        }
        let radius = I::point(separation).mul(I::point(0.5))?.lo;
        if !radius.is_finite() || radius <= 0. {
            out.reason = "pole-graph-positive-disk-radius-unproven";
            return Ok(out);
        }
        stars.push(PoleGraphStar {
            vertex,
            axis,
            point,
            radius,
            faces: faces.into_iter().collect(),
        });
        out.completed_stars += 1;
    }
    out.certificate = Some(PoleGraphAtlas { charts, stars });
    out.uncertain_vertex = None;
    out.reason = "source-body-owned-pole-stars-are-single-sheeted-projected-disks";
    Ok(out)
}
