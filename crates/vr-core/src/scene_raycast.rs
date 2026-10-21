//! Scene raycast: preserves the original arithmetic and proof budgets.
use super::*;

/// Möller–Trumbore XR controller raycast against prepared VR scene meshes.
pub fn raycast_scene(
    scene: &[PreparedMesh],
    origin: [f32; 3],
    direction: [f32; 3],
) -> Option<VrRayHit> {
    let dir_len =
        (direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2])
            .sqrt();
    if !dir_len.is_finite() || dir_len <= 1e-12 {
        return None;
    }
    let dir = [
        direction[0] / dir_len,
        direction[1] / dir_len,
        direction[2] / dir_len,
    ];
    let mut best: Option<VrRayHit> = None;
    for (mi, mesh) in scene.iter().enumerate() {
        for (ti, tri) in mesh.indices.chunks_exact(3).enumerate() {
            let i0 = (tri[0] as usize) * 3;
            let i1 = (tri[1] as usize) * 3;
            let i2 = (tri[2] as usize) * 3;
            if i0 + 2 >= mesh.positions.len()
                || i1 + 2 >= mesh.positions.len()
                || i2 + 2 >= mesh.positions.len()
            {
                continue;
            }
            let v0 = [
                mesh.positions[i0],
                mesh.positions[i0 + 1],
                mesh.positions[i0 + 2],
            ];
            let v1 = [
                mesh.positions[i1],
                mesh.positions[i1 + 1],
                mesh.positions[i1 + 2],
            ];
            let v2 = [
                mesh.positions[i2],
                mesh.positions[i2 + 1],
                mesh.positions[i2 + 2],
            ];
            let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
            let h = [
                dir[1] * e2[2] - dir[2] * e2[1],
                dir[2] * e2[0] - dir[0] * e2[2],
                dir[0] * e2[1] - dir[1] * e2[0],
            ];
            let a = e1[0] * h[0] + e1[1] * h[1] + e1[2] * h[2];
            if a.abs() < 1e-7 {
                continue;
            }
            let f = 1.0 / a;
            let s = [origin[0] - v0[0], origin[1] - v0[1], origin[2] - v0[2]];
            let u = f * (s[0] * h[0] + s[1] * h[1] + s[2] * h[2]);
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let q = [
                s[1] * e1[2] - s[2] * e1[1],
                s[2] * e1[0] - s[0] * e1[2],
                s[0] * e1[1] - s[1] * e1[0],
            ];
            let v = f * (dir[0] * q[0] + dir[1] * q[1] + dir[2] * q[2]);
            if v < 0.0 || u + v > 1.0 {
                continue;
            }
            let t = f * (e2[0] * q[0] + e2[1] * q[1] + e2[2] * q[2]);
            if t > 1e-6 && best.is_none_or(|b| t < b.distance_m) {
                best = Some(VrRayHit {
                    mesh_index: mi,
                    triangle_index: ti,
                    distance_m: t,
                    point: [
                        origin[0] + dir[0] * t,
                        origin[1] + dir[1] * t,
                        origin[2] + dir[2] * t,
                    ],
                });
            }
        }
    }
    best
}
