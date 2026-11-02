//! Nominal periodic sampling only. Exact modulo chooses the original sheet;
//! the final binary64 scalar is a proposal, not an interval geometry proof.
use crate::{Result, check, surface::Surface};
use num_rational::BigRational as Q;
use num_traits::ToPrimitive;
pub fn wrapped_surface_parameters(surface: &Surface, uv: [f64; 2]) -> Result<[f64; 2]> {
    surface.validate()?;
    check(
        uv.iter().all(|v| v.is_finite()),
        "Wrapped surface parameters must be finite",
    )?;
    let d = [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ];
    let mut out = uv;
    for k in 0..2 {
        if [surface.periodic_u, surface.periodic_v][k] {
            let q = |v| Q::from_float(v).unwrap();
            let period = q(d[k][1]) - q(d[k][0]);
            let offset = q(uv[k]) - q(d[k][0]);
            let lift = (&offset / &period).floor();
            let exact = q(d[k][0]) + offset - lift * period;
            out[k] = exact
                .to_f64()
                .ok_or_else(|| crate::input("Wrapped parameter overflow"))?;
            // A rounded near-high interior value may select the upper shore.
            // Native periodic storage has the same endpoint image; diagnostic
            // evaluation retains its usual one-sided derivative semantics.
            check(
                out[k] >= d[k][0] && out[k] <= d[k][1],
                "Wrapped parameter leaves its source chart",
            )?;
        } else {
            check(
                uv[k] >= d[k][0] && uv[k] <= d[k][1],
                "Nonperiodic coordinate leaves original surface domain",
            )?;
        }
    }
    Ok(out)
}
