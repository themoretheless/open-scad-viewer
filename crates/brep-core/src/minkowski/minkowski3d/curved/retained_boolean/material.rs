use super::*;
use nurbs_core::trim_domain::Location;
pub(super) fn classify(
    models: [&Model; 2],
    p: &[NurbsFacePartition; 2],
    op: &str,
    depth: u8,
) -> Result<Vec<RetainedRegionMaterial>> {
    let oracles = [
        atlas::point_material::MaterialOracle::new(models[0], depth)?,
        atlas::point_material::MaterialOracle::new(models[1], depth)?,
    ];
    let mut regions = Vec::new();
    let mut work = Work::default();
    for side in 0..2 {
        let m = &p[side].model;
        for f in 0..m.faces.len() {
            let face = &m.faces[f];
            let domain = crate::face_domain::FaceDomain::new(m, f, 1e-10)?;
            let poles = exact::uses(m, f)
                .flat_map(|c| c.pcurve.control_points.iter())
                .collect::<Vec<_>>();
            let lo: [f64; 2] =
                std::array::from_fn(|k| poles.iter().map(|p| p[k]).min_by(f64::total_cmp).unwrap());
            let hi: [f64; 2] =
                std::array::from_fn(|k| poles.iter().map(|p| p[k]).max_by(f64::total_cmp).unwrap());
            let mut accepted = None;
            'search: for n in [2, 4, 8, 16] {
                for i in 1..n {
                    for j in 1..n {
                        work.charge(1)?;
                        let uv = [
                            lo[0] + (hi[0] - lo[0]) * i as f64 / n as f64,
                            lo[1] + (hi[1] - lo[1]) * j as f64 / n as f64,
                        ];
                        let c = domain.classify(uv.map(|x| [x, x]), 4096)?;
                        if c.location != Location::Inside || c.winding != Some(1) {
                            continue;
                        }
                        let u = exact::q(uv[0]);
                        let v = exact::q(uv[1]);
                        let net = sb::cell(&face.surface, [u.clone(), u], [v.clone(), v])?;
                        let point = sb::point(&net[0][0]);
                        // Failure is inconclusive, not Outside. The same native
                        // Work ledger is retained while another seed is proposed.
                        if let Ok(w) = oracles[1 - side].classify(&point, &mut work) {
                            accepted = Some((uv, w));
                            break 'search;
                        }
                    }
                }
            }
            let (uv, w) = accepted
                .ok_or_else(|| fail("no strict region seed with original-solid material proof"))?;
            let inside = w == 1;
            let keep = match op {
                "union" => !inside,
                "intersection" => inside,
                "difference" => {
                    if side == 0 {
                        !inside
                    } else {
                        inside
                    }
                }
                "xor" => true,
                _ => unreachable!(),
            };
            let reverse = match op {
                "difference" => side == 1,
                "xor" => inside,
                _ => false,
            };
            regions.push(RetainedRegionMaterial {
                operand: side,
                face: f,
                original_face: p[side].face_sources[f],
                seed_uv: uv,
                winding_in_other: w,
                keep,
                reverse,
            });
        }
    }
    Ok(regions)
}
