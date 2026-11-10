//! Regularized material Booleans separated by exact polynomial graph barriers.
//! Contact may have singular algebraic curves; no SSI branch fit authorizes it.
use super::*;
use nurbs_core::ss_intersection as ss;
#[derive(Clone, Debug, PartialEq)]
pub struct GraphOrderBooleanCertificate {
    pub version: u32,
    pub sources: [String; 2],
    pub operation: String,
    pub faces: [usize; 2],
    pub depth: u8,
    pub limits: ss::GraphOrderLimits,
    pub source_forests: [NurbsShellForestCertificate; 2],
    pub exact_uses: [usize; 2],
    pub order_cells: usize,
    pub boundary_cells: usize,
    pub square_identity: bool,
    pub result: String,
}
pub struct CertifiedGraphOrderBoolean {
    model: Model,
    certificate: GraphOrderBooleanCertificate,
}
impl CertifiedGraphOrderBoolean {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn certificate(&self) -> &GraphOrderBooleanCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
fn native<T>(r: nurbs_core::Result<T>) -> Result<T> {
    r.map_err(|e| {
        if e.code == "NURBS_RESOURCE_LIMIT" {
            crate::Error::new("BREP_RESOURCE_LIMIT", e.message)
        } else {
            fail(&e.message)
        }
    })
}
/// Each source material is bounded by its original oriented shell forest.
/// Its boundary lies on one side of a continuous graph barrier. A bounded
/// material cannot cross that barrier without crossing its boundary: z-F(x,y)
/// has nonzero z derivative, so a positive maximum cannot be in its interior.
/// The ordered barriers exclude common 3D interior. A nonzero polynomial gap
/// has no open XY zero patch, so retained body boundaries share no face area.
/// Union/XOR keep separate input bodies, including lower-dimensional contacts;
/// they do not claim a single globally embedded result boundary atlas.
pub fn certify_graph_order_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
    faces: [usize; 2],
    depth: u8,
    limits: ss::GraphOrderLimits,
) -> Result<CertifiedGraphOrderBoolean> {
    a.validate()?;
    b.validate()?;
    if !matches!(operation, "intersection" | "difference" | "union" | "xor")
        || depth > 3
        || faces[0] >= a.faces.len()
        || faces[1] >= b.faces.len()
    {
        return Err(fail("Invalid graph-order material request"));
    }
    let sa = &a.faces[faces[0]].surface;
    let sb = &b.faces[faces[1]].surface;
    let order = native(ss::certify_polynomial_graph_order(
        &ss::FiberSources::new(sa, sb)?,
        limits,
    ))?;
    let mut boundary_cells = 0;
    for (operand, m) in [a, b].into_iter().enumerate() {
        for f in &m.faces {
            // The order certificate is on the first graph's whole rectangle.
            native(ss::prove_graph_projection_inside(sa, &f.surface))?;
            boundary_cells += native(ss::prove_polynomial_graph_boundary_side(
                if operand == 0 { sa } else { sb },
                &f.surface,
                operand == 0,
                limits,
            ))?;
        }
    }
    let (_, au) = super::boundary_only::boundary(a)?;
    let (_, bu) = super::boundary_only::boundary(b)?;
    let source_forests = [
        certify_nurbs_shell_forest(a, depth)?,
        certify_nurbs_shell_forest(b, depth)?,
    ];
    let model = match operation {
        "intersection" => Model::empty(a.tolerance_mm.max(b.tolerance_mm))?,
        "difference" => a.clone(),
        _ => crate::boolean_support::concatenate_bodies(a, b)?,
    };
    model.validate()?;
    let certificate = GraphOrderBooleanCertificate {
        version: 1,
        sources: [
            source_id(&Source::Brep(a.clone()))?,
            source_id(&Source::Brep(b.clone()))?,
        ],
        operation: operation.into(),
        faces,
        depth,
        limits,
        source_forests,
        exact_uses: [au, bu],
        order_cells: order.cells(),
        boundary_cells,
        square_identity: order.has_global_square_identity(),
        result: source_id(&Source::Brep(model.clone()))?,
    };
    Ok(CertifiedGraphOrderBoolean { model, certificate })
}
pub fn verify_graph_order_boolean(
    a: &Model,
    b: &Model,
    result: &Model,
    c: &GraphOrderBooleanCertificate,
) -> Result<()> {
    if c.version != 1 || c.result != source_id(&Source::Brep(result.clone()))? {
        return Err(fail("Graph-order Boolean result binding changed"));
    }
    let fresh = certify_graph_order_boolean(a, b, &c.operation, c.faces, c.depth, c.limits)?;
    if fresh.model != *result || fresh.certificate != *c {
        return Err(fail("Original graph/material Boolean certificate changed"));
    }
    Ok(())
}
pub(crate) fn try_graph_order_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<Option<Model>> {
    if a.faces.is_empty() || b.faces.is_empty() || a.faces.len() > 64 || b.faces.len() > 64 {
        return Ok(None);
    }
    for reverse in [false, true] {
        let (lower, upper) = if reverse { (b, a) } else { (a, b) };
        for i in 0..lower.faces.len() {
            for j in 0..upper.faces.len() {
                // The regularized intersection proof is symmetric. If B is below
                // A, the same proved absence of common interior leaves A\B=A.
                let proof_operation = if reverse && operation == "difference" {
                    "intersection"
                } else {
                    operation
                };
                match certify_graph_order_boolean(
                    lower,
                    upper,
                    proof_operation,
                    [i, j],
                    1,
                    ss::GraphOrderLimits::default(),
                ) {
                    Ok(r) => {
                        return Ok(Some(if reverse && operation == "difference" {
                            a.clone()
                        } else {
                            r.into_model()
                        }));
                    }
                    Err(e) if e.code == "MINKOWSKI_3D_REFUSED" => {}
                    Err(e) => return Err(e),
                }
            }
        }
    }
    Ok(None)
}
