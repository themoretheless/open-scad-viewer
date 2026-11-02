//! Exact separating halfspaces prove empty MATERIAL intersection even when
//! original boundaries touch or coincide. This is not a general SSI graph.
use super::*;
#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryOnlyNurbsBooleanCertificate {
    pub version: u32,
    pub sources: [String; 2],
    pub operation: String,
    pub depth: u8,
    pub axis: usize,
    pub first_operand: usize,
    pub separating_band: [f64; 2],
    pub source_forests: [NurbsShellForestCertificate; 2],
    pub exact_uses: [usize; 2],
    pub result: String,
}
pub struct CertifiedBoundaryOnlyNurbsBoolean {
    model: Model,
    certificate: BoundaryOnlyNurbsBooleanCertificate,
}
impl CertifiedBoundaryOnlyNurbsBoolean {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn certificate(&self) -> &BoundaryOnlyNurbsBooleanCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
fn boundary(m: &Model) -> Result<([[f64; 2]; 3], usize)> {
    if m.faces.is_empty() || m.faces.len() > 256 {
        return Err(fail(
            "boundary-only proof requires bounded nonempty original faces",
        ));
    }
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut uses = 0;
    for face in &m.faces {
        let s = &face.surface;
        if s.weights.iter().flatten().any(|&w| w <= 0.) {
            return Err(fail(
                "whole-carrier support hull needs positive original weights",
            ));
        }
        for p in s.control_points.iter().flatten() {
            for k in 0..3 {
                bounds[k][0] = bounds[k][0].min(p[k]);
                bounds[k][1] = bounds[k][1].max(p[k]);
            }
        }
        let natural = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        for w in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for c in &m.loops[w].coedges {
                let curve = &m.edges[c.edge].curve;
                if c.trim().is_some_and(|t| {
                    t.curve_parameter != curve.domain() || t.pcurve_parameter != c.pcurve.domain()
                }) || c.pcurve.weights.iter().any(|&w| w <= 0.)
                    || c.pcurve
                        .control_points
                        .iter()
                        .any(|p| (0..2).any(|k| p[k] < natural[k][0] || p[k] > natural[k][1]))
                {
                    return Err(fail(
                        "boundary-only proof needs complete natural-domain original uses",
                    ));
                }
                let r = nurbs_core::curve_surface_agreement::verify_exact(
                    curve, &c.pcurve, s, c.reversed, 1_000_000,
                )?
                .ok_or_else(|| fail("original boundary exact lift unavailable"))?;
                if r.outcome != cad_predicates::BezierIdentity::Equal {
                    return Err(fail(
                        "original boundary does not have exact carrier identity",
                    ));
                }
                uses += 1;
            }
        }
    }
    Ok((bounds, uses))
}
pub fn certify_nurbs_boundary_only_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
    depth: u8,
) -> Result<CertifiedBoundaryOnlyNurbsBoolean> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "intersection" | "difference") || depth > 3 {
        return Err(fail(
            "boundary-only material proof admits intersection/difference only",
        ));
    }
    let (ab, au) = boundary(a)?;
    let (bb, bu) = boundary(b)?;
    let mut separation = None;
    for axis in 0..3 {
        if ab[axis][1] <= bb[axis][0] {
            separation = Some((axis, 0, [ab[axis][1], bb[axis][0]]));
            break;
        }
        if bb[axis][1] <= ab[axis][0] {
            separation = Some((axis, 1, [bb[axis][1], ab[axis][0]]));
            break;
        }
    }
    let (axis, first_operand, separating_band) =
        separation.ok_or_else(|| fail("no proved separating closed support halfspaces"))?;
    let source_forests = [
        certify_nurbs_shell_forest(a, depth)?,
        certify_nurbs_shell_forest(b, depth)?,
    ];
    // Each bounded original material is inside its complete boundary hull.
    // Disjoint OPEN halfspaces exclude common 3D interior, including when the
    // separating band has zero width. Lower-dimensional contact is removed by
    // regularization; no tangency root classification or coordinate epsilon.
    let model = if operation == "intersection" {
        Model::empty(a.tolerance_mm.max(b.tolerance_mm))?
    } else {
        a.clone()
    };
    let certificate = BoundaryOnlyNurbsBooleanCertificate {
        version: 1,
        sources: [
            source_id(&Source::Brep(a.clone()))?,
            source_id(&Source::Brep(b.clone()))?,
        ],
        operation: operation.into(),
        depth,
        axis,
        first_operand,
        separating_band,
        source_forests,
        exact_uses: [au, bu],
        result: source_id(&Source::Brep(model.clone()))?,
    };
    Ok(CertifiedBoundaryOnlyNurbsBoolean { model, certificate })
}
pub fn verify_nurbs_boundary_only_boolean(
    a: &Model,
    b: &Model,
    result: &Model,
    c: &BoundaryOnlyNurbsBooleanCertificate,
) -> Result<()> {
    if c.version != 1 || c.result != source_id(&Source::Brep(result.clone()))? {
        return Err(fail("boundary-only result/version binding mismatch"));
    }
    let r = certify_nurbs_boundary_only_boolean(a, b, &c.operation, c.depth)?;
    if r.model != *result || r.certificate != *c {
        return Err(fail(
            "boundary-only original support or material certificate changed",
        ));
    }
    Ok(())
}
pub(crate) fn try_nurbs_boundary_only_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<Option<Model>> {
    if !matches!(operation, "intersection" | "difference") {
        return Ok(None);
    }
    match certify_nurbs_boundary_only_boolean(a, b, operation, 1) {
        Ok(r) => Ok(Some(r.into_model())),
        Err(e) if e.code == "MINKOWSKI_3D_REFUSED" => Ok(None),
        Err(e) => Err(e),
    }
}
