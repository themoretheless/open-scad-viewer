//! Full-field admission of finite G2 runout skins, prior to corner/body sewing.
use super::{ContactQualificationLimits, corner_patch_body::Build, refused};
use crate::{FaceUse, Model, Shell, face_curvature};
use nurbs_core::{
    Result,
    surface::{Axis, Surface},
    surface_blend::{self, Options, Trace},
};

pub struct QualifiedEndRunout {
    transition: surface_blend::TraceReport,
    panels: Vec<Model>,
    fields: Vec<face_curvature::Report>,
    interiors: Vec<nurbs_core::surface_linear_monotonicity::Report>,
    work: (usize, u64, usize, usize),
}
impl QualifiedEndRunout {
    pub fn transition(&self) -> &surface_blend::TraceReport {
        &self.transition
    }
    pub fn panels(&self) -> &[Model] {
        &self.panels
    }
    pub fn fields(&self) -> &[face_curvature::Report] {
        &self.fields
    }
    pub fn interiors(&self) -> &[nurbs_core::surface_linear_monotonicity::Report] {
        &self.interiors
    }
    pub fn work(&self) -> (usize, u64, usize, usize) {
        self.work
    }
}
fn panel(s: &Surface, tolerance: f64) -> Result<Model> {
    let u = [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]];
    let v = [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]];
    let uv = [[u[0], v[0]], [u[1], v[0]], [u[1], v[1]], [u[0], v[1]]];
    let curves = [
        s.iso(Axis::V, v[0])?,
        s.iso(Axis::U, u[1])?,
        s.iso(Axis::V, v[1])?.reverse()?,
        s.iso(Axis::U, u[0])?.reverse()?,
    ];
    let mut b = Build::empty(tolerance);
    let nodes = curves
        .each_ref()
        .map(|c| b.node(c.control_points[0].clone().try_into().unwrap()));
    let mut segments = Vec::new();
    for (i, c) in curves.into_iter().enumerate() {
        segments.push((
            nodes[i],
            nodes[(i + 1) % 4],
            c,
            nurbs_core::curve::Curve::from_polyline(vec![
                uv[i].to_vec(),
                uv[(i + 1) % 4].to_vec(),
            ])?,
        ));
    }
    let f = b.face(s.clone(), segments)?;
    b.m.shells.push(Shell {
        faces: vec![FaceUse {
            face: f,
            reversed: false,
        }],
        closed: false,
    });
    b.m.rebuild_topology_ids();
    b.m.validate()?;
    Ok(b.m)
}
/// Construct a finite transition between immutable original support traces.
/// All original-support and internal G2 seams, each whole panel's regular
/// injectivity and |principal curvature| <=10/mm must pass before publication.
/// Traces/setbacks are explicit: no silent widening or changed source geometry.
/// Open panels are not retained-face membership, corner caps or a closed solid.
pub fn build_g2_end_runout(
    supports: [&Surface; 2],
    traces: [Trace; 2],
    options: Options,
    projection: [[i8; 3]; 2],
    tolerance_mm: f64,
    tolerance_uv: f64,
    limits: ContactQualificationLimits,
) -> Result<QualifiedEndRunout> {
    if !(8..=100_000).contains(&limits.cells)
        || limits.exact_work > 1_000_000
        || !(1..=1_000_000).contains(&limits.domain_cells)
        || !(1..=1_000_000).contains(&limits.shape_cells)
        || !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(refused(
            "End runout requires positive tolerances and bounded work",
        ));
    }
    for support in supports {
        support.validate()?;
        if support.control_points[0][0].len() != 3 {
            return Err(refused(
                "End runout requires three-dimensional original supports",
            ));
        }
    }
    let transition = surface_blend::g2_traces(
        supports,
        traces,
        options,
        [tolerance_mm, 1e-5, 1e-4],
        limits.cells,
        limits.exact_work,
    )?;
    if !transition.complete
        || !transition.within_tolerance
        || transition.construction.patches.is_empty()
    {
        return Err(refused(
            "Original-support or internal runout G2 remains unproven",
        ));
    }
    let mut cells = transition.cells;
    let exact = transition.exact_work;
    let mut domain = 0;
    let mut shape = 0;
    let mut panels = Vec::new();
    let mut fields = Vec::new();
    let mut interiors = Vec::new();
    for s in &transition.construction.patches {
        if cells >= limits.cells {
            return Err(refused("Runout interior work exhausted"));
        }
        let interior =
            nurbs_core::surface_linear_monotonicity::inspect(s, projection, limits.cells - cells)?;
        cells += interior.cells;
        if !interior.certified {
            return Err(refused("Whole runout regular injectivity remains unproven"));
        }
        if cells >= limits.cells || domain >= limits.domain_cells || shape >= limits.shape_cells {
            return Err(refused("Runout field work exhausted"));
        }
        let model = panel(s, tolerance_mm)?;
        let field = face_curvature::inspect(
            &model,
            0,
            10.,
            tolerance_uv,
            face_curvature::Limits {
                cells: (limits.cells - cells).min(50_000),
                domain_cells: limits.domain_cells - domain,
                shape_cells: limits.shape_cells - shape,
            },
            &[],
        )?;
        cells += field.cells;
        domain += field.domain_cells;
        shape += field.shape_cells;
        if field.decision != face_curvature::Decision::Within
            || field
                .absolute_curvature_upper
                .is_none_or(|x| !x.is_finite() || x > 10.)
        {
            return Err(refused("Whole runout curvature <=10/mm remains unproven"));
        }
        panels.push(model);
        fields.push(field);
        interiors.push(interior);
    }
    Ok(QualifiedEndRunout {
        transition,
        panels,
        fields,
        interiors,
        work: (cells, exact, domain, shape),
    })
}

mod fields;
pub use fields::{RunoutPanelField, RunoutPanelFields, inspect_runout_panels};
