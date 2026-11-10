//! profile certificate algorithm: source_bits.
use super::*;
pub fn source_bits(loops: &[Vec<Curve>], tolerance: f64) -> Result<String> {
    use std::fmt::Write;
    if !tolerance.is_finite()
        || tolerance <= 0.
        || loops.len() > 32
        || loops.iter().any(Vec::is_empty)
        || loops.iter().map(Vec::len).sum::<usize>() > MAX_SPANS
    {
        return Err(refused(
            "Certificate needs 1..32 nonempty closed wires, at most 128 spans and a positive tolerance",
        ));
    }
    let mut out = format!(
        "represented-profile-v1/{:016x}/{}",
        tolerance.to_bits(),
        loops.len()
    );
    for wire in loops {
        write!(out, "/w{}", wire.len()).unwrap();
        for c in wire {
            c.validate()?;
            if !(1..=8).contains(&c.degree)
                || c.control_points
                    .iter()
                    .any(|p| p.len() != 2 || p.iter().any(|x| x.abs() > 1e6))
            {
                return Err(refused(
                    "Certificate requires positive-weight planar curves of degree 1..8",
                ));
            }
            write!(
                out,
                "/c{}/{}/{}/{}",
                c.degree,
                c.control_points.len(),
                c.knots.len(),
                u8::from(c.periodic)
            )
            .unwrap();
            for x in c
                .control_points
                .iter()
                .flatten()
                .chain(&c.weights)
                .chain(&c.knots)
            {
                write!(out, "/{:016x}", x.to_bits()).unwrap();
            }
            if out.len() > 65536 {
                return Err(refused("Certificate exact source exceeds its size budget"));
            }
        }
    }
    Ok(out)
}
