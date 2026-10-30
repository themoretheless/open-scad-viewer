//! standard parts algorithm: roller_chain_sprocket.
use super::*;
/// Generates outer and optional inner bore 2D profile loops for an ISO 606 roller chain sprocket.
pub fn roller_chain_sprocket(o: &SprocketOptions) -> Result<Vec<Vec<Point>>> {
    if !(6..=150).contains(&o.teeth) {
        return Err(err("sprocket", "Sprocket teeth must be from 6 to 150."));
    }
    if !o.chain_pitch_mm.is_finite() || !(4.0..=100.0).contains(&o.chain_pitch_mm) {
        return Err(err(
            "sprocket",
            "Sprocket chain_pitch_mm must be from 4 to 100 mm.",
        ));
    }
    if !o.roller_diameter_mm.is_finite()
        || o.roller_diameter_mm <= 1.0
        || o.roller_diameter_mm >= o.chain_pitch_mm * 0.85
    {
        return Err(err(
            "sprocket",
            "Sprocket roller_diameter_mm must be between 1.0 mm and 0.85 * chain_pitch_mm.",
        ));
    }
    if !(4..=24).contains(&o.segments_per_tooth) {
        return Err(err("sprocket", "segments_per_tooth must be from 4 to 24."));
    }
    let z = o.teeth as f64;
    let pitch_radius = o.chain_pitch_mm / (2.0 * (PI / z).sin());
    let seating_radius = 0.5025 * o.roller_diameter_mm + 0.025;
    let root_radius = pitch_radius - seating_radius;
    let tip_radius =
        o.chain_pitch_mm * (0.6 + 1.0 / (PI / z).tan()) * 0.5 - 0.15 * o.roller_diameter_mm;
    if !o.bore_mm.is_finite() || o.bore_mm < 0.0 || o.bore_mm >= 2.0 * root_radius - 1.0 {
        return Err(err(
            "sprocket",
            "Sprocket bore_mm must leave at least 0.5 mm radial wall inside root circle.",
        ));
    }
    let total_steps = o.teeth * o.segments_per_tooth;
    let mut outer = Vec::with_capacity(total_steps);
    for i in 0..total_steps {
        let theta = TAU * (i as f64) / (total_steps as f64);
        let phase = (i % o.segments_per_tooth) as f64 / (o.segments_per_tooth as f64);
        // Smooth roller seating pocket + tooth crest transition
        let u = (phase - 0.5).abs() * 2.0; // 0 at roller center, 1 at tooth tip
        let blend = u * u * (3.0 - 2.0 * u);
        let r = root_radius + (tip_radius - root_radius) * blend;
        outer.push([r * theta.cos(), r * theta.sin()]);
    }
    let mut loops = vec![outer];
    if o.bore_mm > 0.0 {
        let r_bore = o.bore_mm * 0.5;
        let bore: Vec<Point> = (0..32)
            .rev()
            .map(|i| {
                let a = TAU * (i as f64) / 32.0;
                [r_bore * a.cos(), r_bore * a.sin()]
            })
            .collect();
        loops.push(bore);
    }
    Ok(loops)
}
