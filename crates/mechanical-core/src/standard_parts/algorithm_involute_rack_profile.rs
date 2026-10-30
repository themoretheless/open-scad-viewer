//! standard parts algorithm: involute_rack_profile.
use super::*;
/// Generates a closed 2D profile loop for an involute linear gear rack along the X axis.
pub fn involute_rack_profile(o: &RackOptions) -> Result<Vec<Point>> {
    if !(1..=256).contains(&o.teeth) {
        return Err(err("rack", "Rack teeth must be from 1 to 256."));
    }
    if !o.module.is_finite() || !(0.1..=100.0).contains(&o.module) {
        return Err(err("rack", "Rack module must be from 0.1 to 100 mm."));
    }
    if !o.pressure_angle.is_finite() || !(14.5..=30.0).contains(&o.pressure_angle) {
        return Err(err(
            "rack",
            "Rack pressure_angle must be from 14.5 to 30 degrees.",
        ));
    }
    if !o.base_height.is_finite() || !(0.5..=1000.0).contains(&o.base_height) {
        return Err(err("rack", "Rack base_height must be from 0.5 to 1000 mm."));
    }
    if !o.clearance.is_finite() || !(0.0..=o.module).contains(&o.clearance) {
        return Err(err("rack", "Rack clearance must be from 0 to one module."));
    }
    let pitch = PI * o.module;
    let alpha = o.pressure_angle * PI / 180.0;
    let tan_a = alpha.tan();
    let addendum = o.module;
    let dedendum = o.module + o.clearance;
    let half_tooth_pitch = pitch * 0.25;
    let dx_add = addendum * tan_a;
    let dx_ded = dedendum * tan_a;
    if half_tooth_pitch <= dx_add + 1e-6 {
        return Err(err("rack", "Rack tooth tip collapses."));
    }
    let total_length = o.teeth as f64 * pitch;
    let y_bottom = -dedendum - o.base_height;
    let mut loop_pts = Vec::with_capacity(o.teeth * 4 + 4);
    loop_pts.push([0.0, y_bottom]);
    loop_pts.push([total_length, y_bottom]);
    for i in (0..o.teeth).rev() {
        let x0 = i as f64 * pitch;
        let center = x0 + pitch * 0.5;
        let x_right_root = center + half_tooth_pitch + dx_ded;
        let x_right_tip = center + half_tooth_pitch - dx_add;
        let x_left_tip = center - half_tooth_pitch + dx_add;
        let x_left_root = center - half_tooth_pitch - dx_ded;
        loop_pts.push([x_right_root.min(x0 + pitch), -dedendum]);
        loop_pts.push([x_right_tip, addendum]);
        loop_pts.push([x_left_tip, addendum]);
        loop_pts.push([x_left_root.max(x0), -dedendum]);
    }
    Ok(loop_pts)
}
