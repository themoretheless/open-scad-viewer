//! Offline diagnostic surface; fresh inspection required, no inherited receipts.
use brep_core::region_replacement as r;
use nurbs_core::surface::Surface;
pub(crate) fn candidate(collars: &r::CornerCollars) -> r::CentralGraph {
    let mut bytes = &include_bytes!("trihedral-central-candidate-v1.bin")[..];
    assert_eq!(&bytes[..8], b"OSVCGF01");
    bytes = &bytes[8..];
    let mut u32_ = || {
        let (head, tail) = bytes.split_at(4);
        bytes = tail;
        u32::from_le_bytes(head.try_into().unwrap()) as usize
    };
    let [du, dv, nu, nv, ku, kv] = std::array::from_fn(|_| u32_());
    assert!(du <= 12 && dv <= 12 && nu * nv <= 4096 && ku == nu + du + 1 && kv == nv + dv + 1);
    let mut number = || {
        let (head, tail) = bytes.split_at(8);
        bytes = tail;
        f64::from_le_bytes(head.try_into().unwrap())
    };
    let knots_u = (0..ku).map(|_| number()).collect::<Vec<_>>();
    let knots_v = (0..kv).map(|_| number()).collect::<Vec<_>>();
    let affine: [f64; 6] = std::array::from_fn(|_| number());
    let greville = |k: &[f64], p: usize, n: usize| {
        (0..n)
            .map(|i| k[i + 1..=i + p].iter().sum::<f64>() / p as f64)
            .collect::<Vec<_>>()
    };
    let u = greville(&knots_u, du, nu);
    let v = greville(&knots_v, dv, nv);
    // Lossless stored Z poles plus a rounded affine reconstruction of XY.
    // This is a NEW candidate, so the test verifies every stored edge lift;
    // it does not inherit the source artifact's boundary G2 qualification.
    let control_points = (0..nu)
        .map(|i| {
            (0..nv)
                .map(|j| {
                    let z = number();
                    vec![
                        z + affine[0] + affine[1] * u[i] + affine[2] * v[j],
                        z + affine[3] + affine[4] * u[i] + affine[5] * v[j],
                        z,
                    ]
                })
                .collect()
        })
        .collect();
    assert!(bytes.is_empty());
    let surface = Surface {
        degree_u: du,
        degree_v: dv,
        knots_u,
        knots_v,
        control_points,
        weights: vec![vec![1.; nv]; nu],
        periodic_u: false,
        periodic_v: false,
    };
    let determinant = affine[1] * affine[5] - affine[2] * affine[4];
    let pcurves = collars
        .boundaries
        .iter()
        .map(|c| {
            let mut pc = c.curve.clone();
            pc.control_points = pc
                .control_points
                .iter()
                .map(|p| {
                    let x = p[0] - p[2] - affine[0];
                    let y = p[1] - p[2] - affine[3];
                    vec![
                        (x * affine[5] - y * affine[2]) / determinant,
                        (affine[1] * y - affine[4] * x) / determinant,
                    ]
                })
                .collect();
            pc
        })
        .collect();
    r::CentralGraph {
        surface,
        pcurves,
        sampled_jet_residual: [f64::INFINITY; 6],
    }
}
