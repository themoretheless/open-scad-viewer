use super::Contour;

/// Configuration for textured "Fuzzy Skin" outer perimeter wall displacement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FuzzySkinConfig {
    /// Maximum outward normal jitter amplitude in millimeters (typical `0.3` mm).
    pub thickness_mm: f64,
    /// Target subdivision spacing along outer contour segments in millimeters (typical `0.8` mm).
    pub point_dist_mm: f64,
    /// Deterministic PRNG seed for reproducible surface texture.
    pub seed: u64,
}

impl Default for FuzzySkinConfig {
    fn default() -> Self {
        Self {
            thickness_mm: 0.3,
            point_dist_mm: 0.8,
            seed: 0x517C_C1B7_2722_0A95,
        }
    }
}

/// Subdivides a closed outer contour at `point_dist_mm` intervals and displaces vertices
/// outward along the 2D segment normal by deterministic pseudo-random offsets in `[0, thickness_mm]`
/// to produce a matte / textured 3D-printed exterior ("Fuzzy Skin").
pub fn apply_fuzzy_skin_to_contour(
    contour: &Contour,
    thickness_mm: f64,
    point_dist_mm: f64,
    seed: u64,
) -> Contour {
    if contour.len() < 3
        || !thickness_mm.is_finite()
        || thickness_mm <= 0.0
        || !point_dist_mm.is_finite()
        || point_dist_mm <= 1e-4
    {
        return contour.clone();
    }
    let n = contour.len();
    // Determine contour signed area so outward normal points away from interior regardless of winding
    let mut area2 = 0.0;
    for i in 0..n {
        let a = contour[i];
        let b = contour[(i + 1) % n];
        area2 += a[0] * b[1] - b[0] * a[1];
    }
    // For CCW contour (area2 > 0), outward unit normal of (dx, dy) is (dy, -dx) / len
    let out_sign = if area2 >= 0.0 { 1.0 } else { -1.0 };

    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    let mut next_u01 = || -> f64 {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        ((z >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
    };

    let mut out = Vec::with_capacity(n * 4);
    for i in 0..n {
        let a = contour[i];
        let b = contour[(i + 1) % n];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let seg_len = dx.hypot(dy);
        if seg_len <= 1e-9 {
            continue;
        }
        let nx = out_sign * (dy / seg_len);
        let ny = out_sign * (-dx / seg_len);
        let steps = ((seg_len / point_dist_mm).round() as usize).clamp(1, 2048);
        let inv_steps = 1.0 / steps as f64;
        for s in 0..steps {
            let t = (s as f64) * inv_steps;
            let disp = next_u01() * thickness_mm;
            out.push([
                a[0] + t * dx + disp * nx,
                a[1] + t * dy + disp * ny,
            ]);
        }
    }
    out
}
