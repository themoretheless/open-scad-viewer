//! Faceted multi-start helical thread mesh generation, with no source emission.
use crate::{Point, Result, err};
use std::collections::HashMap;
use std::f64::consts::TAU;
#[derive(Debug, Clone)]
pub struct ThreadOptions {
    pub diameter: f64,
    pub pitch: f64,
    pub length: f64,
    pub wall: f64,
    pub clearance: f64,
    pub starts: usize,
    pub segments_per_turn: usize,
    pub internal: bool,
    pub left_handed: bool,
}
impl Default for ThreadOptions {
    fn default() -> Self {
        Self {
            diameter: 6.,
            pitch: 1.,
            length: 2.,
            wall: 1.,
            clearance: 0.1,
            starts: 1,
            segments_per_turn: 24,
            internal: false,
            left_handed: false,
        }
    }
}
impl ThreadOptions {
    fn validate_finite(&self) -> Result<()> {
        for (key, v) in [
            ("diameter", self.diameter),
            ("pitch", self.pitch),
            ("length", self.length),
            ("wall", self.wall),
            ("clearance", self.clearance),
        ] {
            if !v.is_finite() {
                return Err(err("thread", format!("Thread {key} must be finite.")));
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct ThreadGeometry {
    pub positions: Vec<[f64; 3]>,
    pub indices: Vec<usize>,
    pub options: ThreadOptions,
    pub angular_columns: usize,
}
const BREAKS: [f64; 4] = [1. / 16., 3. / 8., 5. / 8., 15. / 16.];
fn clip(poly: &[Point], slope: f64, bound: f64, lower: bool) -> Vec<Point> {
    let mut out = Vec::with_capacity(poly.len() + 2);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let sign = if lower { 1. } else { -1. };
        let da = (a[1] - slope * a[0] - bound) * sign;
        let db = (b[1] - slope * b[0] - bound) * sign;
        let ia = da >= -1e-11;
        let ib = db >= -1e-11;
        if ia {
            out.push(a);
        }
        if ia != ib {
            let f = da / (da - db);
            out.push([a[0] + f * (b[0] - a[0]), a[1] + f * (b[1] - a[1])]);
        }
    }
    out
}
// Coordinates are nonnegative and bounded by 64. Compute the exact rounded
// decimal key used by JS toFixed(11), without allocating decimal strings. The
// 53-bit significand times 5^11 fits u128; this avoids floating-point rounding
// before the final decimal rounding (including exact halfway cases).
fn fixed11_key(value: f64) -> u64 {
    debug_assert!((0.0..=64.0).contains(&value));
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    if exponent == 0 {
        return 0;
    }
    let significand = ((bits & ((1u64 << 52) - 1)) | (1u64 << 52)) as u128;
    let scaled = significand * 48_828_125u128; // 5^11
    let shift = (1064 - exponent) as u32;
    if shift >= 128 {
        return 0;
    }
    let whole = scaled >> shift;
    let remainder = scaled & ((1u128 << shift) - 1);
    (whole + u128::from(remainder >= (1u128 << (shift - 1)))) as u64
}

struct ThreadMesh<'a> {
    o: &'a ThreadOptions,
    turns: f64,
    points: Vec<[f64; 3]>,
    faces: Vec<[usize; 3]>,
    ids: HashMap<(bool, u64, u64), usize>,
    path: &'a str,
}
impl ThreadMesh<'_> {
    fn point(&mut self, mut t: f64, mut y: f64, outer: bool) -> usize {
        if (t - 1.).abs() < 1e-10 || t.abs() < 1e-10 {
            t = 0.;
        }
        if y.abs() < 1e-10 {
            y = 0.;
        }
        if (y - self.turns).abs() < 1e-10 {
            y = self.turns;
        }
        let key = (outer, fixed11_key(t), fixed11_key(y));
        if let Some(id) = self.ids.get(&key) {
            return *id;
        }
        let angle = TAU * t;
        let radius = if outer {
            self.o.diameter / 2. + self.o.clearance / 2. + self.o.wall
        } else {
            let phase = y
                - (if self.o.left_handed { -1. } else { 1. }) * self.o.starts as f64 * angle / TAU;
            let wrapped = phase - phase.floor();
            let distance = wrapped.min(1. - wrapped);
            let depth = 3f64.sqrt() * self.o.pitch * (distance - 1. / 16.).clamp(0., 5. / 16.);
            self.o.diameter / 2. - depth
                + (if self.o.internal { 1. } else { -1. }) * self.o.clearance / 2.
        };
        let round = |v: f64| format!("{v:.12e}").parse::<f64>().unwrap();
        let id = self.points.len();
        self.points.push([
            round(radius * angle.cos()),
            round(radius * angle.sin()),
            round(y * self.o.pitch),
        ]);
        self.ids.insert(key, id);
        id
    }
    fn triangle(&mut self, a: usize, b: usize, c: usize, reverse: bool) -> Result<()> {
        if a == b || b == c || c == a {
            return Ok(());
        }
        self.faces.push(if reverse { [a, c, b] } else { [a, b, c] });
        if self.faces.len() > 3500 {
            return Err(err(
                self.path,
                "Thread mesh exceeds 3500 triangles; shorten length, increase pitch, or reduce segments_per_turn.",
            ));
        }
        Ok(())
    }
}
pub fn thread_radius_at(o: &ThreadOptions, angle: f64, z: f64) -> Result<f64> {
    o.validate_finite()?;
    if !angle.is_finite() || !z.is_finite() || o.pitch <= 0. {
        return Err(err("thread", "Invalid thread radius query."));
    }
    let phase =
        z / o.pitch - (if o.left_handed { -1. } else { 1. }) * o.starts as f64 * angle / TAU;
    let wrapped = phase - phase.floor();
    let distance = wrapped.min(1. - wrapped);
    let depth = 3f64.sqrt() * o.pitch * (distance - 1. / 16.).clamp(0., 5. / 16.);
    let radius = o.diameter / 2. - depth + (if o.internal { 1. } else { -1. }) * o.clearance / 2.;
    if !phase.is_finite() || !radius.is_finite() {
        return Err(err("thread", "Thread radius exceeds finite numeric range."));
    }
    Ok(radius)
}
pub fn thread_mesh(o: &ThreadOptions) -> Result<ThreadGeometry> {
    let path = "thread";
    o.validate_finite()?;
    let e = |m: &str| err(path, m);
    for (name, v) in [
        ("diameter", o.diameter),
        ("pitch", o.pitch),
        ("length", o.length),
        ("wall", o.wall),
    ] {
        if !(0.01..=10000.).contains(&v) {
            return Err(err(
                path,
                format!("Thread {name} must be between 0.01 and 10000 mm."),
            ));
        }
    }
    let diameter = o.diameter;
    let pitch = o.pitch;
    let length = o.length;
    let clearance = o.clearance;
    let starts = o.starts as f64;
    let segments = o.segments_per_turn as f64;
    let internal = o.internal;
    if diameter < 2. * pitch {
        return Err(e("Thread diameter must be at least twice the pitch."));
    }
    if length < pitch / 4. || length > pitch * 64. {
        return Err(e("Thread length must be between 0.25 and 64 pitches."));
    }
    if clearance < 0. || clearance >= 5. * 3f64.sqrt() * pitch / 16. {
        return Err(e(
            "Thread radial clearance must be nonnegative and smaller than the thread depth.",
        ));
    }
    if starts.fract() != 0. || !(1.0..=4.).contains(&starts) {
        return Err(e("Thread starts must be an integer from 1 to 4."));
    }
    if segments.fract() != 0. || !(16.0..=96.).contains(&segments) || segments < 8. * starts {
        return Err(e(
            "Thread segments_per_turn must be an integer from 16 to 96, and at least eight times starts.",
        ));
    }
    let turns = length / pitch;
    let slope = (if o.left_handed { -1. } else { 1. }) * starts;
    let mut angles: Vec<f64> = (0..=segments as usize)
        .map(|i| i as f64 / segments)
        .collect();
    for y in [0., turns] {
        for k in (y.floor() as i32 - starts as i32 - 1)..=(y.ceil() as i32 + starts as i32 + 1) {
            for corner in BREAKS {
                let t = (y - k as f64 - corner) / slope;
                if t > 1e-10 && t < 1. - 1e-10 {
                    angles.push(t);
                }
            }
        }
    }
    angles.sort_by(f64::total_cmp);
    let columns: Vec<f64> = angles
        .iter()
        .enumerate()
        .filter(|(i, t)| *i == 0 || **t - angles[i - 1] > 1e-10)
        .map(|(_, t)| *t)
        .collect();
    let mut mesh = ThreadMesh {
        o,
        turns,
        points: Vec::new(),
        faces: Vec::new(),
        ids: HashMap::new(),
        path,
    };
    for col in 0..columns.len() - 1 {
        let (a, b) = (columns[col], columns[col + 1]);
        let min = (-slope * a).min(-slope * b);
        let max = (turns - slope * a).max(turns - slope * b);
        let mut breaks = Vec::new();
        for k in min.floor() as i32 - 1..=max.ceil() as i32 {
            for corner in BREAKS {
                breaks.push(k as f64 + corner);
            }
        }
        for j in 0..breaks.len() - 1 {
            let (lo, hi) = (breaks[j], breaks[j + 1]);
            if hi <= min + 1e-11 || lo >= max - 1e-11 {
                continue;
            }
            let poly = clip(
                &clip(&[[a, 0.], [b, 0.], [b, turns], [a, turns]], slope, lo, true),
                slope,
                hi,
                false,
            );
            let all: Vec<usize> = poly
                .iter()
                .map(|[t, y]| {
                    mesh.point(
                        if (*t - a).abs() < 1e-10 {
                            a
                        } else if (*t - b).abs() < 1e-10 {
                            b
                        } else {
                            *t
                        },
                        *y,
                        false,
                    )
                })
                .collect();
            let ids: Vec<usize> = all
                .iter()
                .enumerate()
                .filter(|(i, id)| **id != all[(i + all.len() - 1) % all.len()])
                .map(|(_, id)| *id)
                .collect();
            for k in 1..ids.len().saturating_sub(1) {
                mesh.triangle(ids[0], ids[k], ids[k + 1], !internal)?;
            }
        }
    }
    let bottom = mesh.points.len();
    let top = bottom + 1;
    if !internal {
        mesh.points.push([0., 0., 0.]);
        mesh.points.push([0., 0., length]);
    }
    for i in 0..columns.len() - 1 {
        let (a, b) = (columns[i], columns[i + 1]);
        let a0 = mesh.point(a, 0., false);
        let b0 = mesh.point(b, 0., false);
        let a1 = mesh.point(a, turns, false);
        let b1 = mesh.point(b, turns, false);
        if !internal {
            mesh.triangle(bottom, a0, b0, false)?;
            mesh.triangle(top, b1, a1, false)?;
        } else {
            let ao = mesh.point(a, 0., true);
            let bo = mesh.point(b, 0., true);
            let at = mesh.point(a, turns, true);
            let bt = mesh.point(b, turns, true);
            for [a, b, c] in [
                [ao, b0, a0],
                [ao, bo, b0],
                [at, a1, b1],
                [at, b1, bt],
                [ao, bt, bo],
                [ao, at, bt],
            ] {
                mesh.triangle(a, b, c, false)?;
            }
        }
    }
    Ok(ThreadGeometry {
        positions: mesh.points,
        indices: mesh
            .faces
            .into_iter()
            .flat_map(|f| f.into_iter().rev())
            .collect(),
        options: o.clone(),
        angular_columns: columns.len() - 1,
    })
}

#[cfg(test)]
mod key_tests {
    use super::*;
    #[test]
    fn decimal_vertex_keys_round_exact_halfway_like_javascript() {
        assert_eq!(fixed11_key(0.0), 0);
        assert_eq!(fixed11_key(1.0), 100_000_000_000);
        assert_eq!(fixed11_key(64.0), 6_400_000_000_000);
        assert_eq!(fixed11_key(0.000244140625), 24_414_063);
        assert_eq!(fixed11_key(1.0 / 3.0), 33_333_333_333);
        assert_eq!(fixed11_key(2.0 / 3.0), 66_666_666_667);
    }
}
