//! Independent box-tool material proof for an interior rectangular graph cut.
//! General graph/graph intersections and cuts through original seams refuse.
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct GraphBoxBooleanCertificate {
    pub version: u32,
    pub sources: [String; 2],
    pub operation: String,
    pub source_face: usize,
    pub depth: u8,
    pub column: [[ExactRatio; 2]; 3],
    pub boundary_uses: usize,
    pub boundary_cells: usize,
    pub sewn_edges: usize,
    pub geometry: PocketModelCertificate,
    pub result: String,
}
#[derive(Clone, Debug)]
pub struct CertifiedGraphBoxBoolean {
    model: Model,
    certificate: GraphBoxBooleanCertificate,
}
impl CertifiedGraphBoxBoolean {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn certificate(&self) -> &GraphBoxBooleanCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}

fn box_bounds(tool: &Model) -> Result<[[f64; 2]; 3]> {
    if tool.vertices.len() != 8 || tool.faces.len() != 6 {
        return Err(fail("graph cut tool is not a canonical exact box"));
    }
    let b = std::array::from_fn::<_, 3, _>(|k| {
        [
            tool.vertices
                .iter()
                .map(|p| p.point[k])
                .min_by(f64::total_cmp)
                .unwrap(),
            tool.vertices
                .iter()
                .map(|p| p.point[k])
                .max_by(f64::total_cmp)
                .unwrap(),
        ]
    });
    let mut expected = crate::cuboid(b.map(|r| r[0]), b.map(|r| r[1]))?;
    expected.tolerance_mm = tool.tolerance_mm;
    if expected.0 != tool.0 {
        return Err(fail(
            "complete box topology, carriers and trims do not match exact construction",
        ));
    }
    Ok(b)
}

/// Difference/intersection with an independently supplied exact box. The
/// original graph must project axis-aligned onto XY; the tool footprint must
/// lie strictly inside one original trimmed face and its common triangle.
/// A full column exclusion proves no other original boundary enters the tool.
/// Thus the intersection is exactly floor <= z <= graph over the rectangle,
/// not a sector chosen by a user seed or a sampled containment test.
pub fn certify_graph_box_boolean(
    source: &Model,
    tool: &Model,
    operation: &str,
    source_face: usize,
    depth: u8,
) -> Result<CertifiedGraphBoxBoolean> {
    source.validate()?;
    tool.validate()?;
    if !matches!(operation, "difference" | "intersection")
        || depth > 3
        || source_face >= source.faces.len()
        || source.faces.len() > 256
    {
        return Err(fail(
            "graph/box proof needs difference/intersection and a bounded original face",
        ));
    }
    let b = box_bounds(tool)?;
    // The source atlas consumes complete stored pcurves, not partial use ranges.
    for wire in &source.loops {
        for c in &wire.coedges {
            if c.trim().is_some_and(|t| {
                t.curve_parameter != source.edges[c.edge].curve.domain()
                    || t.pcurve_parameter != c.pcurve.domain()
            }) {
                return Err(fail(
                    "graph/box requires full forward original boundary uses",
                ));
            }
        }
    }
    let boundary = crate::boundary_agreement::verify_with_exact(source, 100000, 1000000)?;
    if !boundary.agreement.complete {
        return Err(fail("graph/box original boundary agreement is unproved"));
    }
    // Positive source weights and the exact convex-hull property exclude
    // every original point from the tool roof and everything above it.
    for f in &source.faces {
        if f.surface.weights.iter().flatten().any(|w| *w <= 0.)
            || f.surface
                .control_points
                .iter()
                .flatten()
                .any(|p| p[2] >= b[2][1])
        {
            return Err(fail("box roof is not strictly above the complete source"));
        }
    }
    let mut work = Work::default();
    let upper = &source.faces[source_face].surface;
    let (_, xy) = input(upper, &mut work)?;
    let mut window: [[Q; 2]; 2] = std::array::from_fn(|_| [Q::zero(), Q::zero()]);
    let mut used = BTreeSet::new();
    for world in 0..2 {
        let axis = match (xy[world][1].is_zero(), xy[world][2].is_zero()) {
            (false, true) => 0,
            (true, false) => 1,
            _ => {
                return Err(fail(
                    "box cut needs an exact axis-aligned affine XY projection",
                ));
            }
        };
        if !used.insert(axis) {
            return Err(fail("box projection axes are dependent"));
        }
        let mut r =
            b[world].map(|v| (Q::from_float(v).unwrap() - &xy[world][0]) / &xy[world][axis + 1]);
        if r[0] > r[1] {
            r.swap(0, 1);
        }
        window[axis] = r;
    }
    let column: [[Q; 2]; 3] = b.map(|r| r.map(|x| Q::from_float(x).unwrap()));
    // Stronger than the pocket swept-volume exclusion: prevents another
    // source component above this face from being missed inside the tool.
    bounds::other_faces_avoid(source, source_face, &column, &mut work)?;
    let mut floor = upper.clone();
    for p in floor.control_points.iter_mut().flatten() {
        p[2] = b[2][0];
    }
    let pocket = certify_model_pocket(
        source,
        source_face,
        &floor,
        &vec![vec![roots::ratio(&Q::one())]],
        &[],
        window.each_ref().map(|r| r.each_ref().map(roots::ratio)),
        None,
        depth,
    )?;
    let promoted = certify_pocket_boolean_model(&pocket, operation)?;
    let mut model = promoted.model().clone();
    model.tolerance_mm = source.tolerance_mm.max(tool.tolerance_mm);
    model.inherit_topology_ids(&[source, tool]);
    model.validate()?;
    let result_boundary = crate::boundary_agreement::verify_with_exact(&model, 100000, 1000000)?;
    if !result_boundary.agreement.complete {
        return Err(fail("authored graph/box boundary agreement is unproved"));
    }
    let sew = crate::trim_sew::sew_closed_model_edges(&model)?;
    if !sew.complete || !sew.displacement_budget_ok {
        return Err(fail("graph/box retained-region seams are incomplete"));
    }
    let certificate = GraphBoxBooleanCertificate {
        version: 1,
        sources: [
            source_id(&Source::Brep(source.clone()))?,
            source_id(&Source::Brep(tool.clone()))?,
        ],
        operation: operation.into(),
        source_face,
        depth,
        column: column.each_ref().map(|r| r.each_ref().map(roots::ratio)),
        boundary_uses: boundary.agreement.uses.len(),
        boundary_cells: boundary.agreement.cells,
        sewn_edges: sew.matched,
        geometry: promoted.certificate().clone(),
        result: source_id(&Source::Brep(model.clone()))?,
    };
    Ok(CertifiedGraphBoxBoolean { model, certificate })
}

pub fn verify_graph_box_boolean(
    source: &Model,
    tool: &Model,
    result: &Model,
    certificate: &GraphBoxBooleanCertificate,
) -> Result<()> {
    if certificate.version != 1 {
        return Err(fail("graph/box certificate version mismatch"));
    }
    let fresh = certify_graph_box_boolean(
        source,
        tool,
        &certificate.operation,
        certificate.source_face,
        certificate.depth,
    )?;
    if fresh.model != *result || fresh.certificate != *certificate {
        return Err(fail(
            "graph/box geometry, region selection, source or operation changed",
        ));
    }
    Ok(())
}

pub(crate) fn try_graph_box_boolean(
    source: &Model,
    tool: &Model,
    operation: &str,
) -> Result<Option<Model>> {
    if !matches!(operation, "difference" | "intersection")
        || source.faces.len() > 64
        || box_bounds(tool).is_err()
    {
        return Ok(None);
    }
    for face in 0..source.faces.len() {
        match certify_graph_box_boolean(source, tool, operation, face, 0) {
            Ok(r) => return Ok(Some(r.into_model())),
            Err(e) if e.code == "MINKOWSKI_3D_REFUSED" => {}
            Err(e) => return Err(e),
        }
    }
    Ok(None)
}
