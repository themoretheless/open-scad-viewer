//! Document-model appearance: paints, gradients, recolor, shadows, styles.
//! Sampling math only — no renderer.
use crate::{check, Result};

/// Non-premultiplied sRGB with alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }
    pub const fn black() -> Self {
        Self::rgb(0, 0, 0)
    }
    pub const fn transparent() -> Self {
        Self::rgba(0, 0, 0, 0)
    }
    pub const fn to_rgba(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
    pub const fn from_rgba(c: [u8; 4]) -> Self {
        Self::rgba(c[0], c[1], c[2], c[3])
    }
    pub fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillRule {
    #[default]
    Nonzero,
    EvenOdd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GradientSpread {
    #[default]
    Pad,
    Repeat,
    Reflect,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GradientKind {
    Linear { p1: [f64; 2], p2: [f64; 2] },
    Radial { center: [f64; 2], radius: f64 },
    Conic { center: [f64; 2], angle: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientStop {
    pub offset: f64,
    pub color: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    pub kind: GradientKind,
    pub stops: Vec<GradientStop>,
    pub spread: GradientSpread,
}

impl Gradient {
    pub fn linear(p1: [f64; 2], p2: [f64; 2], stops: Vec<GradientStop>) -> Self {
        Self {
            kind: GradientKind::Linear { p1, p2 },
            stops,
            spread: GradientSpread::Pad,
        }
    }

    pub fn sample(&self, t: f64) -> Color {
        sample_stops(&self.stops, apply_spread(t, self.spread))
    }

    /// Sample at a point in bbox-normalized `[0,1]²` space.
    pub fn sample_at(&self, p: [f64; 2]) -> Color {
        let t = match self.kind {
            GradientKind::Linear { p1, p2 } => {
                let d = [p2[0] - p1[0], p2[1] - p1[1]];
                let len2 = d[0] * d[0] + d[1] * d[1];
                if len2 < 1e-18 {
                    0.0
                } else {
                    ((p[0] - p1[0]) * d[0] + (p[1] - p1[1]) * d[1]) / len2
                }
            }
            GradientKind::Radial { center, radius } => {
                let r = if radius.abs() < 1e-12 { 1.0 } else { radius };
                let dx = p[0] - center[0];
                let dy = p[1] - center[1];
                dx.hypot(dy) / r
            }
            GradientKind::Conic { center, angle } => {
                let a = (p[1] - center[1]).atan2(p[0] - center[0]) - angle;
                let frac = a / std::f64::consts::TAU;
                frac - frac.floor()
            }
        };
        self.sample(t)
    }

    pub fn shift_hsl(&self, hue_deg: f64, sat_pct: f64, light_pct: f64) -> Self {
        let mut g = self.clone();
        for s in &mut g.stops {
            s.color = shift_color(s.color, hue_deg, sat_pct, light_pct);
        }
        g
    }
}

/// Map a raw parameter into `[0,1]` per SVG `spreadMethod`.
pub fn apply_spread(t: f64, spread: GradientSpread) -> f64 {
    if !t.is_finite() {
        return 0.0;
    }
    match spread {
        GradientSpread::Pad => t.clamp(0.0, 1.0),
        GradientSpread::Repeat => t.rem_euclid(1.0),
        GradientSpread::Reflect => {
            let m = t.rem_euclid(2.0);
            if m > 1.0 {
                2.0 - m
            } else {
                m
            }
        }
    }
}

fn sample_stops(stops: &[GradientStop], t: f64) -> Color {
    if stops.is_empty() {
        return Color::black();
    }
    let mut owned: Vec<GradientStop> = stops.to_vec();
    owned.sort_by(|a, b| a.offset.total_cmp(&b.offset));
    if t <= owned[0].offset {
        return owned[0].color;
    }
    let last = owned[owned.len() - 1];
    if t >= last.offset {
        return last.color;
    }
    for w in owned.windows(2) {
        let (a, b) = (w[0], w[1]);
        if t >= a.offset && t <= b.offset {
            let span = b.offset - a.offset;
            let f = if span > 1e-12 { (t - a.offset) / span } else { 0.0 };
            let lerp = |x: u8, y: u8| (f64::from(x) + (f64::from(y) - f64::from(x)) * f).round() as u8;
            return Color::rgba(
                lerp(a.color.r, b.color.r),
                lerp(a.color.g, b.color.g),
                lerp(a.color.b, b.color.b),
                lerp(a.color.a, b.color.a),
            );
        }
    }
    last.color
}

#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    Solid(Color),
    Gradient(Gradient),
}

impl Paint {
    pub fn representative_color(&self) -> Color {
        match self {
            Paint::Solid(c) => *c,
            Paint::Gradient(g) => g.sample(0.5),
        }
    }

    pub fn shift_hsl(&self, hue_deg: f64, sat_pct: f64, light_pct: f64) -> Self {
        match self {
            Paint::Solid(c) => Paint::Solid(shift_color(*c, hue_deg, sat_pct, light_pct)),
            Paint::Gradient(g) => Paint::Gradient(g.shift_hsl(hue_deg, sat_pct, light_pct)),
        }
    }

    pub fn with_alpha(&self, a: u8) -> Self {
        match self {
            Paint::Solid(c) => Paint::Solid(c.with_alpha(a)),
            Paint::Gradient(g) => {
                let mut g = g.clone();
                for s in &mut g.stops {
                    s.color.a = a;
                }
                Paint::Gradient(g)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropShadow {
    pub dx: f64,
    pub dy: f64,
    pub color: Color,
    pub blur: f64,
}

impl DropShadow {
    pub fn new(dx: f64, dy: f64, color: Color) -> Self {
        Self {
            dx,
            dy,
            color,
            blur: 0.0,
        }
    }
    pub fn effective_blur(self) -> f64 {
        if self.blur.is_finite() {
            self.blur.max(0.0)
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InnerShadow {
    pub dx: f64,
    pub dy: f64,
    pub color: Color,
    pub blur: f64,
}

impl InnerShadow {
    pub fn new(dx: f64, dy: f64, color: Color) -> Self {
        Self {
            dx,
            dy,
            color,
            blur: 0.0,
        }
    }
    pub fn effective_blur(self) -> f64 {
        if self.blur.is_finite() {
            self.blur.max(0.0)
        } else {
            0.0
        }
    }
}

/// Snapshot of fill / stroke / opacity / markers / shadows (copy appearance).
#[derive(Debug, Clone, PartialEq)]
pub struct Appearance {
    pub fill: Option<Paint>,
    pub stroke: Option<Paint>,
    pub stroke_width: f64,
    pub opacity: f64,
    pub fill_rule: FillRule,
    pub start_marker: crate::planar::stroke::ArrowMarker,
    pub end_marker: crate::planar::stroke::ArrowMarker,
    pub shadows: Vec<DropShadow>,
    pub inner_shadows: Vec<InnerShadow>,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            fill: None,
            stroke: None,
            stroke_width: 0.0,
            opacity: 1.0,
            fill_rule: FillRule::Nonzero,
            start_marker: crate::planar::stroke::ArrowMarker::None,
            end_marker: crate::planar::stroke::ArrowMarker::None,
            shadows: Vec::new(),
            inner_shadows: Vec::new(),
        }
    }
}

impl Appearance {
    pub fn copy_from(&self) -> Self {
        self.clone()
    }

    pub fn apply_to(&self, target: &mut Self) {
        *target = self.clone();
    }

    pub fn recolor(&self, hue_deg: f64, sat_pct: f64, light_pct: f64) -> Self {
        let mut a = self.clone();
        if let Some(p) = &a.fill {
            a.fill = Some(p.shift_hsl(hue_deg, sat_pct, light_pct));
        }
        if let Some(p) = &a.stroke {
            a.stroke = Some(p.shift_hsl(hue_deg, sat_pct, light_pct));
        }
        a
    }
}

/// Shift Hue (degrees), Saturation (%), Lightness (%). Alpha is unchanged.
pub fn shift_color(c: Color, hue_deg: f64, sat_pct: f64, light_pct: f64) -> Color {
    let (h, s, l) = rgb_to_hsl(c);
    let h = (h + hue_deg).rem_euclid(360.0);
    let s = (s * (1.0 + sat_pct / 100.0)).clamp(0.0, 1.0);
    let l = (l + light_pct / 100.0).clamp(0.0, 1.0);
    hsl_to_rgb(h, s, l, c.a)
}

fn rgb_to_hsl(c: Color) -> (f64, f64, f64) {
    let r = f64::from(c.r) / 255.0;
    let g = f64::from(c.g) / 255.0;
    let b = f64::from(c.b) / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let l = (max + min) / 2.0;
    if delta <= 1.5 / 255.0 {
        return (0.0, 0.0, l);
    }
    let s = if l > 0.5 {
        delta / (2.0 - max - min)
    } else {
        delta / (max + min)
    };
    let h = if (max - r).abs() < 1e-15 {
        ((g - b) / delta).rem_euclid(6.0)
    } else if (max - g).abs() < 1e-15 {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    ((h * 60.0).rem_euclid(360.0), s, l)
}

fn hue_to_channel(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

fn hsl_to_rgb(h: f64, s: f64, l: f64, alpha: u8) -> Color {
    if s <= 1e-12 {
        let v = (l.clamp(0.0, 1.0) * 255.0).round() as u8;
        return Color::rgba(v, v, v, alpha);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let hk = h.rem_euclid(360.0) / 360.0;
    let to_u8 = |x: f64| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::rgba(
        to_u8(hue_to_channel(p, q, hk + 1.0 / 3.0)),
        to_u8(hue_to_channel(p, q, hk)),
        to_u8(hue_to_channel(p, q, hk - 1.0 / 3.0)),
        alpha,
    )
}

/// Validate a style snapshot (finite widths / offsets, opacity in range).
pub fn validate_appearance(a: &Appearance) -> Result<()> {
    check(a.stroke_width.is_finite() && a.stroke_width >= 0.0, "Invalid stroke width")?;
    check(a.opacity.is_finite() && (0.0..=1.0).contains(&a.opacity), "Invalid opacity")?;
    for s in &a.shadows {
        check(s.dx.is_finite() && s.dy.is_finite(), "Invalid shadow offset")?;
    }
    for s in &a.inner_shadows {
        check(s.dx.is_finite() && s.dy.is_finite(), "Invalid inner-shadow offset")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spread_modes() {
        assert!((apply_spread(-0.2, GradientSpread::Pad) - 0.0).abs() < 1e-12);
        assert!((apply_spread(1.25, GradientSpread::Repeat) - 0.25).abs() < 1e-12);
        assert!((apply_spread(1.25, GradientSpread::Reflect) - 0.75).abs() < 1e-12);
    }

    #[test]
    fn linear_midpoint() {
        let g = Gradient::linear(
            [0., 0.5],
            [1., 0.5],
            vec![
                GradientStop {
                    offset: 0.,
                    color: Color::rgb(0, 0, 0),
                },
                GradientStop {
                    offset: 1.,
                    color: Color::rgb(255, 255, 255),
                },
            ],
        );
        let mid = g.sample_at([0.5, 0.5]);
        assert!((mid.r as i32 - 128).abs() <= 1);
    }

    #[test]
    fn recolor_preserves_alpha() {
        let c = shift_color(Color::rgba(200, 40, 40, 90), 0., -100., 0.);
        assert_eq!(c.a, 90);
        assert_eq!(c.r, c.g);
        assert_eq!(c.g, c.b);
    }

    #[test]
    fn copy_appearance_clones() {
        let src = Appearance {
            fill: Some(Paint::Solid(Color::rgb(1, 2, 3))),
            opacity: 0.5,
            ..Default::default()
        };
        let dst = src.copy_from();
        assert_eq!(src, dst);
    }
}
