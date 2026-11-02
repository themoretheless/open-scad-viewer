//! Two original cutter roots own one shared middle edge on both support faces.
use crate::{Model, TopoId, face_crossings, feature_ops::edge_partition as shared};
use nurbs_core::{Error, Result, interval_eval::Interval as I};
fn refuse(s: &str) -> Error {
    Error::new("BREP_ROOT_INTERVAL_UNPROVEN", s)
}
pub struct RootInterval {
    split: shared::Split,
    cutters: [Model; 2],
    faces: [usize; 2],
    ordinals: [usize; 2],
    edge: usize,
    key: String,
    tolerance_uv: f64,
    limits: shared::Limits,
    coverage: [face_crossings::Report; 2],
    bands: [[f64; 2]; 2],
    errors: [f64; 2],
    cells: usize,
}
impl RootInterval {
    pub fn model(&self) -> &Model {
        self.split.model()
    }
    pub fn middle_edge(&self) -> usize {
        self.split.edges()[1]
    }
    pub fn endpoints(&self) -> [usize; 2] {
        [self.split.vertices()[0], self.split.vertices()[1]]
    }
    pub fn endpoint_identities(&self) -> [TopoId; 2] {
        self.endpoints().map(|v| self.model().1.vertices[v])
    }
    pub fn coverage(&self) -> &[face_crossings::Report; 2] {
        &self.coverage
    }
    pub fn root_parameters(&self) -> [[f64; 2]; 2] {
        self.bands
    }
    pub fn root_error_upper(&self) -> [f64; 2] {
        self.errors
    }
    pub fn cells(&self) -> usize {
        self.cells
    }
    pub fn partition(&self) -> &shared::Split {
        &self.split
    }
    /// Re-isolate on immutable original geometry, then reconstruct all uses.
    pub fn reverify(&self) -> Result<()> {
        let actual = between_faces(
            self.split.source(),
            self.edge,
            [
                (&self.cutters[0], self.faces[0], self.ordinals[0]),
                (&self.cutters[1], self.faces[1], self.ordinals[1]),
            ],
            &self.key,
            self.tolerance_uv,
            self.limits,
        )?;
        if actual.model() != self.model()
            || actual.endpoints() != self.endpoints()
            || actual.bands != self.bands
            || actual.errors != self.errors
        {
            return Err(refuse(
                "Saved root-owned edge differs from fresh original reconstruction",
            ));
        }
        self.split.reverify()?;
        Ok(())
    }
}
/// Isolate two roots on the original edge and subdivide once at both. All
/// original-world and independently clocked/reversed face-local uses are
/// checked by the shared partition kernel. Root and partition cells share one
/// ledger. Only refinement is admitted: no new material, G2 or global embedding.
pub fn between_faces(
    source: &Model,
    edge: usize,
    cutters: [(&Model, usize, usize); 2],
    key: &str,
    tolerance_uv: f64,
    limits: shared::Limits,
) -> Result<RootInterval> {
    source.validate()?;
    if !(1..=1000000).contains(&limits.max_cells) || !tolerance_uv.is_finite() || tolerance_uv <= 0.
    {
        return Err(refuse("Need bounded shared work and positive UV tolerance"));
    }
    let original = source
        .edges
        .get(edge)
        .ok_or_else(|| refuse("Missing original edge"))?;
    if original.degenerate || original.curve.periodic {
        return Err(refuse("Need nondegenerate nonperiodic original edge"));
    }
    let uses: Vec<_> = source
        .faces
        .iter()
        .enumerate()
        .flat_map(|(f, face)| {
            std::iter::once(face.outer)
                .chain(face.holes.iter().copied())
                .flat_map(move |w| {
                    source.loops[w]
                        .coedges
                        .iter()
                        .filter(move |c| c.edge == edge)
                        .map(move |_| f)
                })
        })
        .collect();
    if uses.len() != 2 || uses[0] == uses[1] {
        return Err(refuse("Need two distinct incident original support faces"));
    }
    let tolerance = source
        .1
        .local_tolerances_mm
        .get(&source.1.edges[edge])
        .copied()
        .unwrap_or(source.tolerance_mm)
        .min(source.tolerance_mm);
    let mut cells = 0;
    let mut coverage = Vec::new();
    for &(cutter, face, _) in &cutters {
        if cells >= limits.max_cells {
            return Err(refuse("Root queries exhausted shared work"));
        }
        let report = face_crossings::inspect(
            source,
            edge,
            cutter,
            face,
            tolerance_uv,
            (limits.max_cells - cells).min(100000),
        )?;
        cells += report.cells;
        if !report.complete {
            return Err(refuse(
                "Original root coverage or trimmed membership unresolved",
            ));
        }
        coverage.push(report);
    }
    let bands = std::array::from_fn::<_, 2, _>(|i| {
        coverage[i]
            .eligible
            .get(cutters[i].2)
            .map(|&r| coverage[i].roots[r].root.parameter_box[0])
    });
    let [Some(first), Some(last)] = bands else {
        return Err(refuse("Retained root ordinal missing"));
    };
    if first[1] >= last[0] {
        return Err(refuse("Root bands must be ordered and disjoint"));
    }
    let domain = original.curve.domain();
    let parameters = [first, last].map(|b| super::root_representative(b, domain));
    if !(domain[0] < parameters[0] && parameters[0] < parameters[1] && parameters[1] < domain[1]) {
        return Err(refuse(
            "Root representatives must be distinct interior original parameters",
        ));
    }
    let remaining = shared::Limits {
        max_cells: limits.max_cells.saturating_sub(cells),
        ..limits
    };
    if remaining.max_cells == 0 {
        return Err(refuse("No work remains for face-local correspondence"));
    }
    let split =
        shared::split_with_uv_tolerance(source, edge, &parameters, key, tolerance_uv, remaining)?;
    cells += split.evidence().cells;
    if split.edges().len() != 3
        || split.vertices().len() != 2
        || split.model().edges[split.edges()[1]].vertices
            != [split.vertices()[0], split.vertices()[1]]
    {
        return Err(refuse("Middle edge does not own both root vertices"));
    }
    let mut errors = [0.; 2];
    // Knot insertion and direct evaluation may round the same rational point
    // differently. Certify the actual single saved vertex against the root
    // enclosure; never rewrite either incident face's endpoints independently.
    for i in 0..2 {
        let root = &coverage[i].roots[coverage[i].eligible[cutters[i].2]].root;
        let saved = split.model().vertices[split.vertices()[i]].point;
        let mut squared = I::point(0.);
        for k in 0..3 {
            let delta = I::point(saved[k]).sub(I::new(
                root.position_bounds[k][0],
                root.position_bounds[k][1],
            )?)?;
            squared = squared.add(delta.mul(delta)?)?;
        }
        errors[i] = squared.hi.max(0.).sqrt().next_up();
        if !errors[i].is_finite() || errors[i] > tolerance {
            return Err(refuse(
                "Actual saved shared vertex is not bounded to its original root",
            ));
        }
    }
    let mut reports = coverage.into_iter();
    Ok(RootInterval {
        split,
        cutters: [cutters[0].0.clone(), cutters[1].0.clone()],
        faces: cutters.map(|c| c.1),
        ordinals: cutters.map(|c| c.2),
        edge,
        key: key.into(),
        tolerance_uv,
        limits,
        coverage: [reports.next().unwrap(), reports.next().unwrap()],
        bands: [first, last],
        errors,
        cells,
    })
}
