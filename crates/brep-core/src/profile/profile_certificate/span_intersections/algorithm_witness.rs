//! span intersections algorithm: witness.
use super::*;
pub fn witness(
    a: &Cell,
    b: &Cell,
    i: usize,
    j: usize,
    diagonal: bool,
    w: f64,
) -> Result<SpanPairCertificate> {
    let mut p = SpanPairCertificate {
        first: i,
        second: j,
        kind: String::new(),
        axis: 0,
        shear: 0,
        factors: Vec::new(),
        relation: Vec::new(),
        branches: None,
    };
    for axis in 0..2 {
        if separated(a, b, axis) {
            p.kind = "separate".into();
            p.axis = axis as u8;
            return Ok(p);
        }
        if diagonal && arithmetic::monotone(&a.net, axis)? {
            p.kind = "stationary".into();
            p.axis = axis as u8;
            p.factors = encode_factors(&engine::stationary(&a.net, &width(a, b, w))?);
            return Ok(p);
        }
        if !diagonal && let Ok((lo, hi, _)) = range(a, b, axis) {
            if lo >= hi {
                p.kind = "range".into();
                p.axis = axis as u8;
                return Ok(p);
            }
            if let Ok(h) = arithmetic::bivariate::partial_relation(&a.net, &b.net, axis) {
                p.kind = "overlap".into();
                p.axis = axis as u8;
                p.relation = h
                    .iter()
                    .map(|row| row.iter().map(ratio).collect())
                    .collect();
                return Ok(p);
            }
        }
    }
    if let Some((h, residual)) = arithmetic::bivariate::component_system(&a.net, &b.net, diagonal)?
    {
        if h.is_empty() {
            p.kind = "whole-square".into();
            return Ok(p);
        }
        let (k, f) = engine::produce_system(&residual, diagonal, &width(a, b, w))?;
        p.kind = "locus".into();
        p.shear = k;
        p.factors = encode_factors(&f);
        p.relation = h
            .iter()
            .map(|row| row.iter().map(ratio).collect())
            .collect();
        return Ok(p);
    }
    let (k, f) = engine::produce(&a.net, &b.net, diagonal, &width(a, b, w))?;
    p.kind = "events".into();
    p.shear = k;
    p.factors = encode_factors(&f);
    Ok(p)
}
