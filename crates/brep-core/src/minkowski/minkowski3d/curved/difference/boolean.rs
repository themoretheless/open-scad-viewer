//! Exact independent-operand Booleans in the admitted common ambient chart.
//! This is not an arbitrary NURBS/NURBS intersection solver.
use super::*;
#[derive(Clone, Debug, PartialEq)]
pub struct QuadraticBooleanCertificate {
    pub version: u32,
    pub operation: String,
    pub geometry: QuadraticDifferenceCertificate,
}
#[derive(Clone, Debug)]
pub struct CertifiedQuadraticBoolean {
    model: Model,
    certificate: QuadraticBooleanCertificate,
}
impl CertifiedQuadraticBoolean {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn certificate(&self) -> &QuadraticBooleanCertificate {
        &self.certificate
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
/// All four regularized operations on independent admitted closed Models.
/// Complete carriers and trim inverse images must pass the common-map proof
/// F(x,y,z)=(x,y,z+k*x*x). The operands may intersect multiple original faces,
/// be nonconvex, have holes or contain multiple components. Nonmanifold
/// regularized boundaries, nonrepresentable controls and exhausted budgets
/// refuse. New caps and all shared subdivisions are authored in ordinary Model.
pub fn certify_quadratic_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
    coefficient: f64,
    depth: u8,
) -> Result<CertifiedQuadraticBoolean> {
    let result = construction(a, b, coefficient, depth, operation)?;
    Ok(CertifiedQuadraticBoolean {
        model: result.model,
        certificate: QuadraticBooleanCertificate {
            version: 1,
            operation: operation.into(),
            geometry: result.certificate,
        },
    })
}
pub fn verify_quadratic_boolean(
    a: &Model,
    b: &Model,
    model: &Model,
    cert: &QuadraticBooleanCertificate,
) -> Result<()> {
    if cert.version != 1 || cert.geometry.coefficient.iter().any(|s| s.len() > 2500) {
        return Err(fail(
            "quadratic Boolean version/coefficient budget mismatch",
        ));
    }
    let parse = |s: &String| {
        s.parse()
            .map_err(|_| fail("invalid quadratic Boolean coefficient"))
    };
    let n = parse(&cert.geometry.coefficient[0])?;
    let d: num_bigint::BigInt = parse(&cert.geometry.coefficient[1])?;
    if d <= 0.into() {
        return Err(fail("invalid quadratic Boolean denominator"));
    }
    let k = Q::new(n, d);
    let coefficient = k
        .to_f64()
        .ok_or_else(|| fail("quadratic Boolean coefficient overflow"))?;
    if Q::from_float(coefficient).as_ref() != Some(&k) {
        return Err(fail("quadratic Boolean coefficient is not binary64 exact"));
    }
    let expected =
        certify_quadratic_boolean(a, b, &cert.operation, coefficient, cert.geometry.depth)?;
    if *cert != expected.certificate || *model != expected.model {
        return Err(fail(
            "quadratic Boolean source/operation/result proof mismatch",
        ));
    }
    Ok(())
}

/// Internal exact dispatch. None means that the whole-source common chart
/// was not proved; no partially fitted coefficient is used to author a result.
pub(crate) fn try_common_quadratic_boolean(
    a: &Model,
    b: &Model,
    operation: &str,
) -> Result<Option<Model>> {
    let Some(k) = shear::common_quadratic_coefficient(a, b, &mut Work::default()) else {
        return Ok(None);
    };
    let Some(coefficient) = k
        .to_f64()
        .filter(|v| Q::from_float(*v).as_ref() == Some(&k))
    else {
        return Ok(None);
    };
    certify_quadratic_boolean(a, b, operation, coefficient, 1).map(|r| Some(r.into_model()))
}
