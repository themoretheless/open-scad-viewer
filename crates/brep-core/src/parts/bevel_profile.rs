//! Actual original 3D bevel outline embedding via exact XY projection.
use super::*;
use nurbs_core::involute::{BevelGearSpec, QualifiedBevelOutline, qualified_bevel_outline};
fn reject(message: &str) -> Error {
    Error::new("BREP_BEVEL_PROFILE_UNPROVEN", message)
}
fn merge(curves: &[Curve]) -> Result<Curve> {
    if curves.is_empty() {
        return Err(reject("Missing original bevel flank chain"));
    }
    let mut points = Vec::new();
    let mut breaks = Vec::new();
    for (j, c) in curves.iter().enumerate() {
        c.validate()?;
        let count = c.control_points.len();
        if c.degree != 3
            || c.periodic
            || count < 4
            || (count - 1) % 3 != 0
            || c.weights.iter().any(|w| *w != 1.)
        {
            return Err(reject("Bevel flank needs original unit-weight cubic spans"));
        }
        let n = (count - 1) / 3;
        let local: Vec<_> = (0..=n).map(|i| c.knots[3 + 3 * i]).collect();
        let mut expected = vec![local[0]; 4];
        for &u in &local[1..n] {
            expected.extend(std::iter::repeat_n(u, 3));
        }
        expected.extend(std::iter::repeat_n(local[n], 4));
        if c.knots != expected
            || (j > 0
                && (breaks.last() != Some(&local[0])
                    || curves[j - 1].control_points.last() != c.control_points.first()))
        {
            return Err(reject("Bevel original Bernstein patch ownership changed"));
        }
        points.extend(c.control_points.iter().skip(usize::from(j > 0)).cloned());
        breaks.extend(local.into_iter().skip(usize::from(j > 0)));
    }
    let mut knots = vec![breaks[0]; 4];
    for &u in &breaks[1..breaks.len() - 1] {
        knots.extend(std::iter::repeat_n(u, 3));
    }
    knots.extend(std::iter::repeat_n(*breaks.last().unwrap(), 4));
    let curve = Curve {
        degree: 3,
        knots,
        weights: vec![1.; points.len()],
        control_points: points,
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}
fn grouped(source: &QualifiedBevelOutline) -> Result<(Vec<Curve>, Vec<Curve>, usize)> {
    source.reverify()?;
    let teeth = source.source().specification().teeth;
    let p = source.source().source().parameters();
    let below = p.root_cone_angle[1] < p.base_cone_angle[0];
    let n = source.source().flanks()[0].patches.len();
    let original = source.curves();
    let mut cursor = 0;
    let mut grouped = Vec::new();
    for _ in 0..teeth {
        if below {
            grouped.push(original[cursor].clone());
            cursor += 1;
        }
        grouped.push(merge(&original[cursor..cursor + n])?);
        cursor += n;
        grouped.push(original[cursor].clone());
        cursor += 1;
        grouped.push(merge(&original[cursor..cursor + n])?);
        cursor += n;
        if below {
            grouped.push(original[cursor].clone());
            cursor += 1;
        }
        grouped.push(original[cursor].clone());
        cursor += 1;
    }
    if cursor != original.len() {
        return Err(reject("Incomplete original bevel outline grouping"));
    }
    let projected = grouped
        .iter()
        .map(|c| {
            let mut c = c.clone();
            for p in &mut c.control_points {
                *p = vec![p[0], p[1]];
            }
            c
        })
        .collect();
    Ok((grouped, projected, if below { 6 } else { 4 }))
}
/// Exact control/weight/knot projection is reconstructed from the original
/// qualified 3D source. A simple XY projection proves the 3D contour is
/// embedded as well. This does not prove cap/wall geometry or a closed solid.
pub fn verify_bevel_profile_projection(
    source: &QualifiedBevelOutline,
    projected: &[Curve],
    certificate: &ProfileEmbeddingCertificate,
    limits: ProfileEmbeddingLimits,
) -> Result<VerifiedGearProfile> {
    let (_, expected, k) = grouped(source)?;
    if projected != expected
        || certificate.teeth != source.source().specification().teeth
        || certificate.curves_per_tooth != k
        || certificate.tooth_wire != 0
    {
        return Err(reject(
            "Bevel embedding certificate is not bound to the original 3D projection",
        ));
    }
    verify_profile_embedding(&[expected], certificate, limits)
}
pub struct VerifiedBevelProfile {
    source: QualifiedBevelOutline,
    grouped: Vec<Curve>,
    projected: Vec<Curve>,
    certificate: ProfileEmbeddingCertificate,
    limits: ProfileEmbeddingLimits,
}
impl VerifiedBevelProfile {
    pub fn source(&self) -> &QualifiedBevelOutline {
        &self.source
    }
    pub fn grouped_profile(&self) -> &[Curve] {
        &self.grouped
    }
    pub fn projected_profile(&self) -> &[Curve] {
        &self.projected
    }
    pub fn certificate(&self) -> &ProfileEmbeddingCertificate {
        &self.certificate
    }
    pub fn reverify(&self) -> Result<()> {
        let (fresh, _, _) = grouped(&self.source)?;
        if fresh != self.grouped {
            return Err(reject("Owned original 3D grouped profile changed"));
        }
        verify_bevel_profile_projection(
            &self.source,
            &self.projected,
            &self.certificate,
            self.limits,
        )?;
        Ok(())
    }
}
pub fn certify_bevel_profile(
    spec: BevelGearSpec,
    tolerance: f64,
    max_patches_per_flank: usize,
    max_surfaces: usize,
    max_curves: usize,
    limits: ProfileEmbeddingLimits,
) -> Result<VerifiedBevelProfile> {
    let source = qualified_bevel_outline(
        spec,
        tolerance,
        max_patches_per_flank,
        max_surfaces,
        max_curves,
    )?;
    let (grouped, projected, k) = grouped(&source)?;
    let (certificate, _) =
        certify_grouped_profile_embedding(&[projected.clone()], spec.teeth, k, 0, 1e-7, limits)?;
    verify_bevel_profile_projection(&source, &projected, &certificate, limits)?;
    Ok(VerifiedBevelProfile {
        source,
        grouped,
        projected,
        certificate,
        limits,
    })
}
