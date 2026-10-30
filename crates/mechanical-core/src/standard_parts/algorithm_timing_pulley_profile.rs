//! standard parts algorithm: timing_pulley_profile.
use super::*;
/// Generates outer and optional inner bore 2D profile loops for a synchronous timing pulley (e.g. GT2/HTD).
pub fn timing_pulley_profile(o: &TimingPulleyOptions) -> Result<Vec<Vec<Point>>> {
    if !(8..=256).contains(&o.teeth) {
        return Err(err("pulley", "Pulley teeth must be from 8 to 256."));
    }
    if !o.pitch_mm.is_finite() || !(1.0..=50.0).contains(&o.pitch_mm) {
        return Err(err("pulley", "Pulley pitch_mm must be from 1 to 50 mm."));
    }
    if !o.tooth_depth_mm.is_finite()
        || o.tooth_depth_mm <= 0.05
        || o.tooth_depth_mm >= o.pitch_mm * 0.8
    {
        return Err(err(
            "pulley",
            "Pulley tooth_depth_mm must be positive and less than 0.8 * pitch.",
        ));
    }
    if !(4..=24).contains(&o.segments_per_tooth) {
        return Err(err("pulley", "segments_per_tooth must be from 4 to 24."));
    }
    let outer_radius = (o.teeth as f64 * o.pitch_mm) / TAU;
    let root_radius = outer_radius - o.tooth_depth_mm;
    if !o.bore_mm.is_finite() || o.bore_mm < 0.0 || o.bore_mm >= 2.0 * root_radius - 0.5 {
        return Err(err(
            "pulley",
            "Pulley bore_mm must leave at least 0.5 mm wall inside root circle.",
        ));
    }
    let total_steps = o.teeth * o.segments_per_tooth;
    let mut outer = Vec::with_capacity(total_steps);
    for i in 0..total_steps {
        let theta = TAU * (i as f64) / (total_steps as f64);
        let phase = (i % o.segments_per_tooth) as f64 / (o.segments_per_tooth as f64);
        // Smooth cosine groove profile matching GT2/HTD rounded tooth pockets
        let groove = if phase < 0.6 {
            let u = phase / 0.6;
            0.5 * (1.0 - (TAU * u).cos())
        } else {
            0.0
        };
        let r = outer_radius - o.tooth_depth_mm * groove;
        outer.push([r * theta.cos(), r * theta.sin()]);
    }
    let mut loops = vec![outer];
    if o.bore_mm > 0.0 {
        let r_bore = o.bore_mm * 0.5;
        let bore_steps = 32;
        let bore: Vec<Point> = (0..bore_steps)
            .rev()
            .map(|i| {
                let a = TAU * (i as f64) / (bore_steps as f64);
                [r_bore * a.cos(), r_bore * a.sin()]
            })
            .collect();
        loops.push(bore);
    }
    Ok(loops)
}
