//! lib operations: solve.
use super::*;
impl TwoHandTransform {
/// Solves the two-handed scale, rotation, and translation gesture between two consecutive XR frames.
    pub fn solve(
        prev_left: [f32; 3],
        prev_right: [f32; 3],
        curr_left: [f32; 3],
        curr_right: [f32; 3],
    ) -> Option<Self> {
        if !prev_left
            .iter()
            .chain(prev_right.iter())
            .chain(curr_left.iter())
            .chain(curr_right.iter())
            .all(|v| v.is_finite())
        {
            return None;
        }
        let prev_vec = [
            prev_right[0] - prev_left[0],
            prev_right[1] - prev_left[1],
            prev_right[2] - prev_left[2],
        ];
        let curr_vec = [
            curr_right[0] - curr_left[0],
            curr_right[1] - curr_left[1],
            curr_right[2] - curr_left[2],
        ];
        let prev_len =
            (prev_vec[0] * prev_vec[0] + prev_vec[1] * prev_vec[1] + prev_vec[2] * prev_vec[2])
                .sqrt();
        let curr_len =
            (curr_vec[0] * curr_vec[0] + curr_vec[1] * curr_vec[1] + curr_vec[2] * curr_vec[2])
                .sqrt();
        if prev_len <= 1e-5 || curr_len <= 1e-5 {
            return None;
        }
        let scale = curr_len / prev_len;
        let prev_mid = [
            (prev_left[0] + prev_right[0]) * 0.5,
            (prev_left[1] + prev_right[1]) * 0.5,
            (prev_left[2] + prev_right[2]) * 0.5,
        ];
        let curr_mid = [
            (curr_left[0] + curr_right[0]) * 0.5,
            (curr_left[1] + curr_right[1]) * 0.5,
            (curr_left[2] + curr_right[2]) * 0.5,
        ];
        let translation_m = [
            curr_mid[0] - prev_mid[0],
            curr_mid[1] - prev_mid[1],
            curr_mid[2] - prev_mid[2],
        ];
        let u = [
            prev_vec[0] / prev_len,
            prev_vec[1] / prev_len,
            prev_vec[2] / prev_len,
        ];
        let v = [
            curr_vec[0] / curr_len,
            curr_vec[1] / curr_len,
            curr_vec[2] / curr_len,
        ];
        let dot = (u[0] * v[0] + u[1] * v[1] + u[2] * v[2]).clamp(-1.0, 1.0);
        let rotation_angle_rad = dot.acos();
        let cx = u[1] * v[2] - u[2] * v[1];
        let cy = u[2] * v[0] - u[0] * v[2];
        let cz = u[0] * v[1] - u[1] * v[0];
        let clen = (cx * cx + cy * cy + cz * cz).sqrt();
        let rotation_axis = if clen > 1e-6 {
            [cx / clen, cy / clen, cz / clen]
        } else {
            [0.0, 1.0, 0.0]
        };
        let prev_yaw = prev_vec[2].atan2(prev_vec[0]);
        let curr_yaw = curr_vec[2].atan2(curr_vec[0]);
        let mut yaw_rad = curr_yaw - prev_yaw;
        while yaw_rad > std::f32::consts::PI {
            yaw_rad -= 2.0 * std::f32::consts::PI;
        }
        while yaw_rad < -std::f32::consts::PI {
            yaw_rad += 2.0 * std::f32::consts::PI;
        }
        Some(Self {
            scale,
            translation_m,
            rotation_axis,
            rotation_angle_rad,
            yaw_rad,
        })
    }
/// Applies the gesture transform (anchored at `pivot`) to a 3D point.
    pub fn apply_to_point(&self, point: [f32; 3], pivot: [f32; 3]) -> [f32; 3] {
        let p = [
            (point[0] - pivot[0]) * self.scale,
            (point[1] - pivot[1]) * self.scale,
            (point[2] - pivot[2]) * self.scale,
        ];
        let [kx, ky, kz] = self.rotation_axis;
        let (sin_a, cos_a) = self.rotation_angle_rad.sin_cos();
        let k_dot_p = kx * p[0] + ky * p[1] + kz * p[2];
        let k_cross_p = [
            ky * p[2] - kz * p[1],
            kz * p[0] - kx * p[2],
            kx * p[1] - ky * p[0],
        ];
        [
            pivot[0]
                + self.translation_m[0]
                + p[0] * cos_a
                + k_cross_p[0] * sin_a
                + kx * k_dot_p * (1.0 - cos_a),
            pivot[1]
                + self.translation_m[1]
                + p[1] * cos_a
                + k_cross_p[1] * sin_a
                + ky * k_dot_p * (1.0 - cos_a),
            pivot[2]
                + self.translation_m[2]
                + p[2] * cos_a
                + k_cross_p[2] * sin_a
                + kz * k_dot_p * (1.0 - cos_a),
        ]
    }
}
