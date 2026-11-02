//! Whole-domain SS exclusion and material classification for separated closed
//! source boundaries. Not a transverse-intersection arrangement solver.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NurbsMaterialRelation {
    Disjoint,
    AContainsB,
    BContainsA,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NonintersectingNurbsBooleanCertificate {
    pub version: u32,
    pub sources: [String; 2],
    pub operation: String,
    pub depth: u8,
    pub source_atlases: [Certificate; 2],
    pub source_boundary_uses: [usize; 2],
    pub source_boundary_cells: [usize; 2],
    pub checked_cross_pairs: usize,
    /// [winding of A's boundary in B, winding of B's boundary in A].
    pub winding: [i32; 2],
    pub relation: NurbsMaterialRelation,
    pub result: String,
    pub result_atlas: Option<Certificate>,
}
#[derive(Clone, Debug)]
pub struct CertifiedNonintersectingNurbsBoolean {
    model: Model,
    certificate: NonintersectingNurbsBooleanCertificate,
}
impl CertifiedNonintersectingNurbsBoolean {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn certificate(&self) -> &NonintersectingNurbsBooleanCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
pub(crate) fn source_boundary(m: &Model) -> Result<(usize, usize)> {
    // Atlas trimming uses the full stored pcurves. A diagnostic proof of a
    // requested subrange cannot certify that full boundary. Directed/partial
    // use materialization must be proved separately before this constructor.
    for wire in &m.loops {
        for coedge in &wire.coedges {
            if let Some(trim) = coedge.trim() {
                if trim.curve_parameter != m.edges[coedge.edge].curve.domain()
                    || trim.pcurve_parameter != coedge.pcurve.domain()
                {
                    return Err(fail(
                        "NURBS Boolean atlas requires full forward authored edge/pcurve use intervals",
                    ));
                }
            }
        }
    }
    // A surface atlas proves its lifted trim images. It does not replace the
    // separate obligation for the actual authored 3D edge curves. In particular
    // a curve can agree at all validator samples and disagree between them.
    let proof = crate::boundary_agreement::verify_with_exact(m, 100_000, 1_000_000)?;
    if !proof.agreement.complete {
        return Err(fail(
            "whole-interval original edge/carrier boundary agreement is unproved",
        ));
    }
    Ok((proof.agreement.uses.len(), proof.agreement.cells))
}
pub(super) fn nominal(c: &Certificate) -> (Vec<P>, Vec<Face>) {
    // These are constructor-owned certificates, never caller-supplied data.
    let points = c
        .positions
        .iter()
        .map(|p| {
            p.each_ref()
                .map(|q| Q::new(q[0].parse().unwrap(), q[1].parse().unwrap()))
        })
        .collect::<Vec<_>>();
    let faces = c
        .triangles
        .iter()
        .map(|t| t.iter().map(|i| points[*i as usize].clone()).collect())
        .collect();
    (points, faces)
}
/// No graph, convexity, shared deformation or global projection is required.
/// Each input must have one connected outward shell, embedded by its complete
/// source-atlas proof. Every swept cross-operand chart pair must separate.
/// This proves zero surface intersections over the FULL original trim domains
/// and through the joint source-to-PL homotopy, not just at a seed/sample.
///
/// The connected boundaries then have constant exact winding in each other.
/// The joint disjoint isotopy transfers that membership to the original NURBS
/// solids. Boolean truth tables select entire unchanged source shells; cavity
/// shells reverse orientation. Touching/intersecting/unproved pairs refuse.
pub fn certify_nonintersecting_nurbs_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
    depth: u8,
) -> Result<CertifiedNonintersectingNurbsBoolean> {
    if !matches!(operation, "union" | "difference" | "intersection" | "xor") || depth > 3 {
        return Err(fail(
            "nonintersecting NURBS Boolean operation/depth is unsupported",
        ));
    }
    a.validate()?;
    b.validate()?;
    let ba = source_boundary(a)?;
    let bb = source_boundary(b)?;
    let (ca, pa) = checked_charts(a, depth)?;
    let (cb, pb) = checked_charts(b, depth)?;
    difference::connected_outer_volume(a, &ca)?;
    difference::connected_outer_volume(b, &cb)?;
    let mut work = Work::default();
    let mut pairs = 0;
    for p in &pa {
        for q in &pb {
            if !disjoint(&p.motion, &q.motion, 0, &mut work)? {
                return Err(fail(
                    "complete cross-operand NURBS intersection exclusion is unproved",
                ));
            }
            pairs += 1;
        }
    }
    let (va, fa) = nominal(&ca);
    let (vb, fb) = nominal(&cb);
    let winding = [
        winding(&va[0], &fb, &mut work)?,
        winding(&vb[0], &fa, &mut work)?,
    ];
    let relation = match winding {
        [0, 0] => NurbsMaterialRelation::Disjoint,
        [0, 1] => NurbsMaterialRelation::AContainsB,
        [1, 0] => NurbsMaterialRelation::BContainsA,
        _ => {
            return Err(fail(
                "embedded connected NURBS material membership is inconsistent",
            ));
        }
    };
    let empty = || Model::empty(a.tolerance_mm.max(b.tolerance_mm));
    use NurbsMaterialRelation::*;
    let mut model = match (relation, operation) {
        (Disjoint, "union" | "xor") => crate::boolean_support::separated_union(a, b)?,
        (Disjoint, "difference") | (AContainsB, "union") | (BContainsA, "intersection") => {
            a.clone()
        }
        (Disjoint, "intersection") | (BContainsA, "difference") => empty()?,
        (AContainsB, "intersection") | (BContainsA, "union") => b.clone(),
        (AContainsB, "difference" | "xor") => difference::cavity(a, b)?,
        (BContainsA, "xor") => difference::cavity(b, a)?,
        _ => unreachable!(),
    };
    model.tolerance_mm = a.tolerance_mm.max(b.tolerance_mm);
    model.inherit_topology_ids(&[a, b]);
    model.validate()?;
    let result_atlas = if model.is_empty() {
        None
    } else {
        Some(derive(&model, depth)?)
    };
    let certificate = NonintersectingNurbsBooleanCertificate {
        version: 1,
        sources: [
            source_id(&Source::Brep(a.clone()))?,
            source_id(&Source::Brep(b.clone()))?,
        ],
        operation: operation.into(),
        depth,
        source_atlases: [ca, cb],
        source_boundary_uses: [ba.0, bb.0],
        source_boundary_cells: [ba.1, bb.1],
        checked_cross_pairs: pairs,
        winding,
        relation,
        result: source_id(&Source::Brep(model.clone()))?,
        result_atlas,
    };
    Ok(CertifiedNonintersectingNurbsBoolean { model, certificate })
}
pub fn verify_nonintersecting_nurbs_boolean(
    a: &Model,
    b: &Model,
    result: &Model,
    c: &NonintersectingNurbsBooleanCertificate,
) -> Result<()> {
    if c.version != 1 || c.result != source_id(&Source::Brep(result.clone()))? {
        return Err(fail(
            "nonintersecting NURBS Boolean result/version binding mismatch",
        ));
    }
    let expected = certify_nonintersecting_nurbs_boolean(a, b, &c.operation, c.depth)?;
    if expected.model != *result || expected.certificate != *c {
        return Err(fail(
            "NURBS SS exclusion/material selection certificate mismatch",
        ));
    }
    Ok(())
}
pub(crate) fn try_nonintersecting_nurbs_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<Option<Model>> {
    match certify_nonintersecting_nurbs_boolean(a, b, operation, 0) {
        Ok(r) => Ok(Some(r.into_model())),
        // Other exact families may prove the case. A failed sufficient proof
        // never supplies topology authority to numerical tracing.
        Err(e) if e.code == "MINKOWSKI_3D_REFUSED" => Ok(None),
        Err(e) => Err(e),
    }
}
#[cfg(test)]
mod tests;
