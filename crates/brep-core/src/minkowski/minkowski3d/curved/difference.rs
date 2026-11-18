//! Source-exact curved subtraction under the common ambient map
//! F(x,y,z)=(x,y,z+k*x*x). Its global inverse preserves all set operations.
//! No claim is made for general implicit atlas regions.
use super::*;
mod boolean;
mod model;
pub use boolean::{
    CertifiedQuadraticBoolean, QuadraticBooleanCertificate, certify_quadratic_boolean,
    verify_quadratic_boolean,
};
#[derive(Clone, Debug, PartialEq)]
pub struct QuadraticDifferenceCertificate {
    pub version: u32,
    pub sources: [String; 2],
    pub coefficient: Ratio,
    pub depth: u8,
    pub reference_vertices: Vec<ExactPoint>,
    pub triangles: Vec<[u32; 3]>,
    pub source_atlases: [SourceAtlasCertificate; 2],
    pub result_atlas: Option<SourceAtlasCertificate>,
}
#[derive(Clone, Debug)]
pub struct CertifiedQuadraticDifference {
    model: Model,
    certificate: QuadraticDifferenceCertificate,
}
impl CertifiedQuadraticDifference {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn certificate(&self) -> &QuadraticDifferenceCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
fn ratio(q: &Q) -> Ratio {
    [q.numer().to_string(), q.denom().to_string()]
}
/// Both input Models must have an exact affine inverse chart on every complete
/// source knot cell under the specified common quadratic deformation. Input
/// trimming loops must have straight inverse images, including hole boundaries.
pub fn certify_quadratic_difference(
    container: &Model,
    tool: &Model,
    coefficient: f64,
    depth: u8,
) -> Result<CertifiedQuadraticDifference> {
    construction(container, tool, coefficient, depth, "difference")
}
fn construction(
    container: &Model,
    tool: &Model,
    coefficient: f64,
    depth: u8,
    operation: &str,
) -> Result<CertifiedQuadraticDifference> {
    if !matches!(operation, "union" | "difference" | "intersection" | "xor") {
        return Err(fail("unknown exact quadratic Boolean operation"));
    }
    if !coefficient.is_finite() || coefficient.abs() > 1e6 || depth > 3 {
        return Err(fail(
            "quadratic difference coefficient/depth budget exceeded",
        ));
    }
    container.validate()?;
    tool.validate()?;
    let source_atlases = [
        certify_source_atlas(container, depth)?,
        certify_source_atlas(tool, depth)?,
    ];
    let k = Q::from_float(coefficient).unwrap();
    let polynomial = vec![vec![Q::zero()], vec![Q::zero()], vec![k.clone()]];
    let mut work = Work::default();
    let inverse = |m: &Model, work: &mut Work| -> Result<Input> {
        let (charts, _) = shear::reference_charts(m, &polynomial, 2, work)?;
        shear::reference_input_generic(m, &charts)
    };
    let a = inverse(container, &mut work)?;
    let b = inverse(tool, &mut work)?;
    let planes = a
        .planes
        .iter()
        .chain(&b.planes)
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut material = Vec::new();
    // Union and xor must retain material outside the first operand's box.
    // Every source plane participates, including seams of nonconvex inputs.
    let bounds = if matches!(operation, "union" | "xor") {
        a.points
            .iter()
            .chain(&b.points)
            .cloned()
            .collect::<Vec<_>>()
    } else {
        a.points.clone()
    };
    for cell in partition(vec![bounding_box(&bounds)?], &planes, &mut work)? {
        let p = centroid(&cell.vertices());
        let inside_a = winding(&p, &a.faces, &mut work)? > 0;
        let inside_b = winding(&p, &b.faces, &mut work)? > 0;
        let keep = match operation {
            "union" => inside_a || inside_b,
            "intersection" => inside_a && inside_b,
            "difference" => inside_a && !inside_b,
            "xor" => inside_a != inside_b,
            _ => unreachable!(),
        };
        if keep {
            material.push(cell);
        }
    }
    let (points, triangles) = boundary_dyadic(&material, &mut work)?;
    super::super::difference::manifold(&triangles)?;
    let model = model::build(&points, &triangles, &k, &mut work)?;
    let result_atlas = if model.bodies.is_empty() {
        None
    } else {
        Some(certify_source_atlas(&model, depth)?)
    };
    let certificate = QuadraticDifferenceCertificate {
        version: 1,
        sources: [
            source_id(&Source::Brep(container.clone()))?,
            source_id(&Source::Brep(tool.clone()))?,
        ],
        coefficient: ratio(&k),
        depth,
        reference_vertices: points.iter().map(|p| p.each_ref().map(ratio)).collect(),
        triangles,
        source_atlases,
        result_atlas,
    };
    Ok(CertifiedQuadraticDifference { model, certificate })
}
pub fn verify_quadratic_difference(
    container: &Model,
    tool: &Model,
    model: &Model,
    cert: &QuadraticDifferenceCertificate,
) -> Result<()> {
    if cert.version != 1 {
        return Err(fail("quadratic difference certificate version mismatch"));
    }
    if cert.coefficient.iter().any(|s| s.len() > 2500) {
        return Err(fail("quadratic coefficient bit budget exceeded"));
    }
    let parse = |s: &String| s.parse().map_err(|_| fail("invalid quadratic coefficient"));
    let numerator = parse(&cert.coefficient[0])?;
    let denominator: num_bigint::BigInt = parse(&cert.coefficient[1])?;
    if denominator <= 0.into() {
        return Err(fail("invalid quadratic denominator"));
    }
    let k = Q::new(numerator, denominator);
    let coefficient = k
        .to_f64()
        .ok_or_else(|| fail("quadratic coefficient overflow"))?;
    if Q::from_float(coefficient).as_ref() != Some(&k) {
        return Err(fail("quadratic coefficient is not binary64 exact"));
    }
    let expected = certify_quadratic_difference(container, tool, coefficient, cert.depth)?;
    if *cert != expected.certificate || *model != expected.model {
        return Err(fail("quadratic difference source/result proof mismatch"));
    }
    Ok(())
}
#[cfg(test)]
mod tests;

pub(crate) use boolean::try_common_quadratic_boolean;
