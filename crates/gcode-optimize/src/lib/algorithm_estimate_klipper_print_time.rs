//! lib algorithm: estimate_klipper_print_time.
use super::*;
/// Estimates realistic print execution time in seconds using Klipper's trapezoidal velocity profile
/// and centripetal cornering velocity limit (`square_corner_velocity_mm_s`, typically `5.0` mm/s)
/// with peak acceleration `accel_mm_s2`.
pub fn estimate_klipper_print_time(
    moves: &[GcodeMove],
    accel_mm_s2: f64,
    square_corner_velocity_mm_s: f64,
) -> f64 {
    if moves.len() < 2 || !accel_mm_s2.is_finite() || accel_mm_s2 <= 0.0 {
        return 0.0;
    }
    let scv = square_corner_velocity_mm_s.max(0.1);
    let inv_accel = 1.0 / accel_mm_s2;
    let inv_2accel = 0.5 * inv_accel;
    let mut total_time_s = 0.0;
    let mut prev_dir: Option<[f64; 3]> = None;
    let mut prev_exit_v = 0.0_f64;

    // Precompute the first segment direction and length so each loop iteration only computes the next segment once.
    let mut cur_dist = 0.0_f64;
    let mut cur_dir = [0.0_f64; 3];
    {
        let a = &moves[0];
        let b = &moves[1];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let dz = b.z - a.z;
        let d = (dx * dx + dy * dy + dz * dz).sqrt();
        if d > 1e-9 {
            cur_dist = d;
            let inv_d = 1.0 / d;
            cur_dir = [dx * inv_d, dy * inv_d, dz * inv_d];
        }
    }

    for i in 1..moves.len() {
        let b = &moves[i];
        let dist = cur_dist;
        let dir = cur_dir;

        // Compute lookahead segment (i -> i+1) once and cache for next iteration
        let mut next_dist = 0.0_f64;
        let mut next_dir = [0.0_f64; 3];
        if i + 1 < moves.len() {
            let c = &moves[i + 1];
            let ndx = c.x - b.x;
            let ndy = c.y - b.y;
            let ndz = c.z - b.z;
            let nd = (ndx * ndx + ndy * ndy + ndz * ndz).sqrt();
            if nd > 1e-9 {
                next_dist = nd;
                let inv_nd = 1.0 / nd;
                next_dir = [ndx * inv_nd, ndy * inv_nd, ndz * inv_nd];
            }
        }
        cur_dist = next_dist;
        cur_dir = next_dir;

        if dist <= 1e-9 {
            continue;
        }
        let v_cruise = b.feedrate_mm_s.max(0.1);

        let v_entry = if let Some(pd) = prev_dir {
            let cos_theta = (pd[0] * dir[0] + pd[1] * dir[1] + pd[2] * dir[2]).clamp(-1.0, 1.0);
            let half_angle = (0.5 * (1.0 - cos_theta)).max(0.0).sqrt(); // sin(theta / 2)
            if half_angle > 1e-4 {
                let cos_half = (0.5 * (1.0 + cos_theta)).max(0.0).sqrt();
                let factor = (cos_half / half_angle).sqrt();
                (scv * factor).min(v_cruise).min(prev_exit_v)
            } else {
                v_cruise.min(prev_exit_v)
            }
        } else {
            0.0
        };

        let v_exit = if next_dist > 1e-9 {
            let c = &moves[i + 1];
            let cos_theta =
                (dir[0] * next_dir[0] + dir[1] * next_dir[1] + dir[2] * next_dir[2]).clamp(-1.0, 1.0);
            let half_angle = (0.5 * (1.0 - cos_theta)).max(0.0).sqrt();
            if half_angle > 1e-4 {
                let cos_half = (0.5 * (1.0 + cos_theta)).max(0.0).sqrt();
                (scv * (cos_half / half_angle).sqrt())
                    .min(v_cruise)
                    .min(c.feedrate_mm_s.max(0.1))
            } else {
                v_cruise.min(c.feedrate_mm_s.max(0.1))
            }
        } else {
            0.0
        };

        // Reachable peak velocity given segment length and acceleration
        let v_peak_sq = accel_mm_s2 * dist + 0.5 * (v_entry * v_entry + v_exit * v_exit);
        let v_peak = v_peak_sq.max(0.0).sqrt().min(v_cruise);
        let d_accel = ((v_peak * v_peak - v_entry * v_entry).max(0.0)) * inv_2accel;
        let d_decel = ((v_peak * v_peak - v_exit * v_exit).max(0.0)) * inv_2accel;
        let d_cruise = (dist - d_accel - d_decel).max(0.0);

        let t_accel = (v_peak - v_entry).max(0.0) * inv_accel;
        let t_decel = (v_peak - v_exit).max(0.0) * inv_accel;
        let t_cruise = d_cruise / v_peak.max(1e-6);

        total_time_s += t_accel + t_cruise + t_decel;
        prev_dir = Some(dir);
        prev_exit_v = v_peak;
    }
    total_time_s
}
