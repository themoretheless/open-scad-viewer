//! Strict wire adapter for freshly checked native exact constructors.
use super::*;
use crate::primitives::{
    exact_conics as C, exact_frusta as F, exact_planar_regions as P, exact_quadrics as Q,
    exact_tori as T,
};
const OPS: [&str; 5] = [
    "curve_exact_ellipse",
    "surface_exact_ellipsoid",
    "surface_exact_ring_torus",
    "surface_exact_frustum_side",
    "surface_exact_planar_region",
];
pub(super) fn handles(op: &str) -> bool {
    OPS.contains(&op)
}
fn schema(v: &Value, fields: &[&str]) -> Result<()> {
    let object = v
        .as_object()
        .ok_or_else(|| input("Exact primitive request must be an object"))?;
    check(
        object.keys().all(|k| fields.contains(&k.as_str())),
        "Unknown exact primitive request field",
    )
}
fn numeric_leaves(v: &Value) -> Result<()> {
    use value_codec::Number;
    match v {
        Value::Array(a) => {
            for v in a {
                numeric_leaves(v)?;
            }
            Ok(())
        }
        Value::Number(Number::Unsigned(n)) => check(
            (*n as f64) as u128 == *n as u128,
            "Integer coordinate is not exactly representable as binary64",
        ),
        Value::Number(Number::Signed(n)) => check(
            (*n as f64) as i128 == *n as i128,
            "Integer coordinate is not exactly representable as binary64",
        ),
        Value::Number(Number::Float(n)) => {
            check(n.is_finite(), "Exact primitive coordinates must be finite")
        }
        _ => Err(input("Exact primitive coordinate must be numeric")),
    }
}
fn coordinate<T: for<'a> Deserialize<'a>>(v: &Value, k: &str) -> Result<T> {
    // Decode the fixed native type first: malformed nesting fails before the
    // leaf walk. Guard integer->f64 casts before any native arithmetic.
    let value = field(v, k)?;
    numeric_leaves(&v[k])?;
    Ok(value)
}
trait WireRefusal {
    fn code(self) -> &'static str;
}
macro_rules! refusals {
    ($($ty:path),+)=>{$(impl WireRefusal for $ty {fn code(self)->&'static str {match self {
        Self::ResourceLimit=>"resource_limit",Self::NotRepresentable=>"not_representable",Self::DependentAxes=>"dependent_axes",Self::InvalidOutput=>"invalid_output",Self::UnprovedIdentity=>"unproved_identity",
    }}})+};
}
refusals!(C::Refusal, Q::Refusal, T::Refusal, F::Refusal, P::Refusal);
fn finish<T: Serialize>(
    geometry: Option<T>,
    kind: &str,
    work: u64,
    refusal: Option<&str>,
    metadata: Value,
) -> Result<Value> {
    let constructed = geometry.is_some();
    numeric(
        constructed == refusal.is_none(),
        "Inconsistent native exact constructor result",
    )?;
    let geometry = geometry.map(encode).transpose()?;
    Ok(
        json!({"status":if constructed {"constructed"} else {"refused"},"geometryKind":kind,"geometry":geometry,"workUsed":work,"refusal":refusal,"metadata":if constructed {metadata} else {Value::Null},"scope":if kind=="curve" {"literal-curve-geometry"} else {"literal-surface-geometry"},"topologyAuthority":false}),
    )
}
pub(super) fn dispatch(v: &Value, op: &str) -> Result<Value> {
    let fields: &[&str] = match op {
        "curve_exact_ellipse" => &["op", "center", "axisU", "axisV", "maxWork"],
        "surface_exact_ring_torus" => &[
            "op",
            "center",
            "axes",
            "majorRadius",
            "minorRadius",
            "maxWork",
        ],
        "surface_exact_ellipsoid" => &["op", "center", "axes", "maxWork"],
        "surface_exact_frustum_side" | "surface_exact_planar_region" => {
            &["op", "center", "axes", "radii", "maxWork"]
        }
        _ => return Err(input("Unsupported exact primitive operation")),
    };
    schema(v, fields)?;
    match op {
        "curve_exact_ellipse" => {
            let r = C::ellipse(
                coordinate(v, "center")?,
                coordinate(v, "axisU")?,
                coordinate(v, "axisV")?,
                field(v, "maxWork")?,
            )?;
            finish(
                r.curve,
                "curve",
                r.work_used,
                r.refusal.map(WireRefusal::code),
                json!({}),
            )
        }
        "surface_exact_ellipsoid" => {
            let r = Q::ellipsoid(
                coordinate(v, "center")?,
                coordinate(v, "axes")?,
                field(v, "maxWork")?,
            )?;
            finish(
                r.surface,
                "surface",
                r.work_used,
                r.refusal.map(WireRefusal::code),
                json!({}),
            )
        }
        "surface_exact_ring_torus" => {
            let r = T::ring(
                coordinate(v, "center")?,
                coordinate(v, "axes")?,
                coordinate(v, "majorRadius")?,
                coordinate(v, "minorRadius")?,
                field(v, "maxWork")?,
            )?;
            finish(
                r.surface,
                "surface",
                r.work_used,
                r.refusal.map(WireRefusal::code),
                json!({}),
            )
        }
        "surface_exact_frustum_side" => {
            let r = F::side(
                coordinate(v, "center")?,
                coordinate(v, "axes")?,
                coordinate(v, "radii")?,
                field(v, "maxWork")?,
            )?;
            let end = r.collapsed_end.map(|end| match end {
                F::End::Start => "start",
                F::End::End => "end",
            });
            finish(
                r.surface,
                "surface",
                r.work_used,
                r.refusal.map(WireRefusal::code),
                json!({"collapsedEnd":end,"intrinsicApex":end.is_some()}),
            )
        }
        "surface_exact_planar_region" => {
            let r = P::radial(
                coordinate(v, "center")?,
                coordinate(v, "axes")?,
                coordinate(v, "radii")?,
                field(v, "maxWork")?,
            )?;
            let region = r.kind.map(|kind| match kind {
                P::RegionKind::FilledDisk => "filledDisk",
                P::RegionKind::Annulus => "annulus",
            });
            finish(
                r.surface,
                "surface",
                r.work_used,
                r.refusal.map(WireRefusal::code),
                json!({"regionKind":region,"collapsedCenter":r.kind==Some(P::RegionKind::FilledDisk),"regularPlaneSupport":true}),
            )
        }
        _ => Err(input("Unsupported exact primitive operation")),
    }
}
