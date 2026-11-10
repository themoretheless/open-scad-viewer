//! Witness search, deliberately separate from the search-free verifier.
use super::*;

/// Produce and independently verify a certificate, or explicitly refuse.
/// Resource/separator failures never become negative intersection decisions.
pub fn certify(loops: &[Vec<Curve>], tolerance: f64) -> Result<ProfileCertificate> {
    certify_impl(loops, tolerance, true)
}
pub(super) fn certify_geometry(loops: &[Vec<Curve>], tolerance: f64) -> Result<ProfileCertificate> {
    certify_impl(loops, tolerance, false)
}
fn certify_impl(
    loops: &[Vec<Curve>],
    tolerance: f64,
    material_left: bool,
) -> Result<ProfileCertificate> {
    let source = source_bits(loops, tolerance)?;
    for depth in 0..=MAX_DEPTH {
        let p = pieces(loops, depth)?;
        let attempt = (|| -> Result<ProfileCertificate> {
            let mut axes = Vec::new();
            for piece in &p {
                let axis = (0..2)
                    .find_map(|axis| match arithmetic::monotone(&piece.net, axis) {
                        Ok(true) => Some(Ok(axis as u8)),
                        Ok(false) => None,
                        Err(e) => Some(Err(e)),
                    })
                    .ok_or_else(|| refused("No exact monotone coordinate found"))??;
                axes.push(axis);
            }
            let mut normals = Vec::new();
            for i in 0..p.len() {
                for j in i + 1..p.len() {
                    let witness = normal_candidates(&p[i], &p[j])
                        .find(|w| {
                            normal(&p[i], &p[j], *w)
                                .is_some_and(|n| separated(&p[i], &p[j], &n, adjacent(&p, i, j)))
                        })
                        .or_else(|| {
                            (p[i].wire == p[j].wire
                                && arithmetic::endpoint_only(
                                    &p[i].net,
                                    &p[j].net,
                                    adjacent(&p, i, j),
                                )
                                .unwrap_or(false))
                            .then_some(u16::MAX)
                        })
                        .or_else(|| {
                            (p[i].wire == p[j].wire
                                && adjacent(&p, i, j)
                                && (p[i].net.len() > 4 || p[j].net.len() > 4)
                                && arithmetic::endpoint_census(&p[i].net, &p[j].net, true)
                                    .unwrap_or(false))
                            .then_some(u16::MAX - 1)
                        })
                        .ok_or_else(|| refused("No exact hull separation found"))?;
                    normals.push(witness);
                }
            }
            let (mut orientation, nesting_depth) = polygon_claims(&p, loops.len())?;
            let orientation_depth = if normals.contains(&u16::MAX)
                || normals.contains(&(u16::MAX - 1))
                || orientation.contains(&0)
            {
                let (d, signs) = (0..=12)
                    .find_map(|d| orientation::check(loops, d).ok().map(|s| (d, s)))
                    .ok_or_else(|| refused("Exact Green orientation remains unresolved"))?;
                orientation = signs;
                Some(d)
            } else {
                None
            };
            Ok(ProfileCertificate {
                version: VERSION,
                source: source.clone(),
                subdivision_depth: depth,
                monotone_axes: axes,
                separating_normals: normals,
                orientation,
                nesting_depth,
                orientation_depth,
            })
        })();
        if let Ok(c) = attempt {
            if material_left {
                verify(loops, tolerance, &c)?;
            } else {
                verify_geometry(loops, tolerance, &c)?;
            }
            return Ok(c);
        }
    }
    Err(refused(
        "No complete profile certificate within the exact subdivision budget",
    ))
}
