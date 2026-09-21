//! Deterministic mesh texture displacement with shared-edge refinement.
use super::{Result, Value, encode, field, input};
use polygon_core::Mesh;
use std::collections::{BTreeMap, BTreeSet};
type V = [f64; 3];
fn sub(a: V, b: V) -> V {
    std::array::from_fn(|k| a[k] - b[k])
}
fn dot(a: V, b: V) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V, b: V) -> V {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: V) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn finite(a: V) -> Result<()> {
    if a.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err(input("Texture exceeds finite numeric range."))
    }
}
fn round(x: f64) -> f64 {
    let f = x.floor();
    if x - f >= 0.5 { f + 1. } else { f }
}
fn uint(x: f64) -> u32 {
    x.trunc().rem_euclid(4294967296.) as u32
}
fn mix(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
fn noise(p: V, seed: u32) -> f64 {
    let cell = p.map(f64::floor);
    let f = std::array::from_fn::<_, 3, _>(|k| {
        let t = p[k] - cell[k];
        t * t * (3. - 2. * t)
    });
    let hash = |x: f64, y: f64, z: f64| {
        let h = uint(x).wrapping_mul(374761393)
            ^ uint(y).wrapping_mul(668265263)
            ^ uint(z).wrapping_mul(2147483647)
            ^ seed;
        let h = (h ^ (h >> 13)).wrapping_mul(1274126177);
        (h ^ (h >> 16)) as f64 / 4294967295.
    };
    let layer = |z| {
        mix(
            mix(
                hash(cell[0], cell[1], z),
                hash(cell[0] + 1., cell[1], z),
                f[0],
            ),
            mix(
                hash(cell[0], cell[1] + 1., z),
                hash(cell[0] + 1., cell[1] + 1., z),
                f[0],
            ),
            f[1],
        )
    };
    mix(layer(cell[2]), layer(cell[2] + 1.), f[2])
}
struct Options {
    pattern: String,
    pitch: f64,
    height: f64,
    angle: f64,
    seed: u32,
    detail: usize,
    invert: bool,
    origin: V,
    u: V,
    v: V,
}
impl Options {
    fn read(o: &Value) -> Result<Self> {
        let pattern: String = field(o, "pattern")?;
        let pitch: f64 = field(o, "pitch")?;
        let height: f64 = field(o, "height")?;
        let angle: f64 = field(o, "angle")?;
        let seed: f64 = field(o, "seed")?;
        let detail: usize = field(o, "detail")?;
        let origin: V = field(o, "origin")?;
        let u: V = field(o, "u")?;
        let v: V = field(o, "v")?;
        let invert: bool = field(o, "invert")?;
        if ![pitch, height, angle, seed].iter().all(|x| x.is_finite())
            || pitch <= 0.
            || height <= 0.
            || height > pitch / 4.
            || !(3..=8).contains(&detail)
            || seed.fract() != 0.
        {
            return Err(input(
                "Use positive pitch/height, height ≤ pitch/4, detail 3–8 and an integer seed.",
            ));
        }
        finite(origin)?;
        finite(u)?;
        finite(v)?;
        if (dot(u, v)).abs() > 1e-6
            || (dot(u, u) - 1.).abs() > 1e-6
            || (dot(v, v) - 1.).abs() > 1e-6
        {
            return Err(input("Invalid texture frame."));
        }
        if !["ribs", "grooves", "knurl", "fuzzy", "dimples", "waves"].contains(&pattern.as_str()) {
            return Err(input("Unknown surface pattern."));
        }
        Ok(Self {
            pattern,
            pitch,
            height,
            angle,
            seed: uint(seed),
            detail,
            invert,
            origin,
            u,
            v,
        })
    }
    fn height(&self, p: V) -> Result<f64> {
        finite(p)?;
        let q = sub(p, self.origin);
        finite(q)?;
        let a = self.angle * std::f64::consts::PI / 180.;
        let x = dot(q, self.u) / self.pitch;
        let y = dot(q, self.v) / self.pitch;
        let s = x * a.cos() - y * a.sin();
        let t = x * a.sin() + y * a.cos();
        finite([a, s, t])?;
        let wave = |v: f64| (1. + (v * 2. * std::f64::consts::PI).cos()) / 2.;
        let h = match self.pattern.as_str() {
            "ribs" => wave(s).powi(2),
            "grooves" => -wave(s).powi(4),
            "knurl" => (wave(s + t) * wave(s - t)).sqrt(),
            "fuzzy" => {
                let p = q.map(|v| v / self.pitch * 2.);
                finite(p)?;
                2. * noise(p, self.seed) - 1.
            }
            "dimples" => {
                let r = (s - round(s)).hypot(t - round(t)) / 0.42;
                if r < 1. { -(1. - r * r).powi(2) } else { 0. }
            }
            "waves" => wave(s + 0.25 * (t * 2. * std::f64::consts::PI).sin()),
            _ => unreachable!(),
        } * self.height
            * if self.invert { -1. } else { 1. };
        if !h.is_finite() {
            return Err(input("Texture exceeds finite numeric range."));
        }
        Ok(h)
    }
}
pub fn height(v: Value) -> Result<Value> {
    let o: Value = field(&v, "options")?;
    encode(Options::read(&o)?.height(field(&v, "point")?)?)
}
pub fn apply(v: Value) -> Result<Value> {
    let body: Value = field(&v, "body")?;
    if body.get("brep").is_some() {
        return Err(input(
            "Texture on retained B-rep is not implemented; geometry was not changed.",
        ));
    }
    let options: Value = field(&v, "options")?;
    let o = Options::read(&options)?;
    let mesh: Mesh = field(&body, "mesh")?;
    let original = mesh.inspect()?;
    if !original.closed || original.signed_volume_mm3 <= 0. {
        return Err(input("Texture requires a closed outward-oriented solid."));
    }
    let mut points = mesh
        .positions
        .as_chunks::<3>().0.iter()
        .map(|p| [p[0], p[1], p[2]])
        .collect::<Vec<_>>();
    let mut faces = mesh
        .indices
        .as_chunks::<3>().0.iter()
        .map(|f| [f[0], f[1], f[2]])
        .collect::<Vec<_>>();
    let selected = if options.get("triangles").is_some() {
        let ids: Vec<usize> = field(&options, "triangles")?;
        if ids.is_empty() || ids.iter().any(|&i| i >= faces.len()) {
            return Err(input("Choose a valid surface."));
        }
        Some(ids.into_iter().collect::<BTreeSet<_>>())
    } else {
        None
    };
    let mut active = (0..faces.len())
        .map(|i| selected.as_ref().is_none_or(|s| s.contains(&i)))
        .collect::<Vec<_>>();
    let mut boundary = Vec::<[V; 2]>::new();
    if selected.is_some() {
        let mut edges = BTreeMap::<(usize, usize), usize>::new();
        for (i, f) in faces.iter().enumerate() {
            if active[i] {
                for k in 0..3 {
                    let (a, b) = (f[k], f[(k + 1) % 3]);
                    *edges.entry((a.min(b), a.max(b))).or_default() += 1;
                }
            }
        }
        for ((a, b), count) in edges {
            if count == 1 {
                boundary.push([points[a], points[b]]);
            }
        }
    }
    let mut longest = 0_f64;
    for f in &faces {
        for k in 0..3 {
            longest = longest.max(norm(sub(points[f[k]], points[f[(k + 1) % 3]])));
        }
    }
    let levels = (longest / (o.pitch / o.detail as f64))
        .log2()
        .ceil()
        .max(0.);
    if !longest.is_finite()
        || !levels.is_finite()
        || levels > 16.
        || faces.len() as f64 * 4_f64.powf(levels) > 40000.
    {
        return Err(input(
            "Texture exceeds 40000 triangles. Increase pitch, lower detail, or simplify the body.",
        ));
    }
    for _ in 0..levels as usize {
        let mut cache = BTreeMap::<(usize, usize), usize>::new();
        let mut next = Vec::with_capacity(faces.len() * 4);
        let mut flags = Vec::with_capacity(faces.len() * 4);
        let mut midpoint = |a: usize, b: usize| -> Result<usize> {
            let key = (a.min(b), a.max(b));
            if let Some(&i) = cache.get(&key) {
                return Ok(i);
            }
            let i = points.len();
            let p = std::array::from_fn(|k| points[a][k] * 0.5 + points[b][k] * 0.5);
            finite(p)?;
            points.push(p);
            cache.insert(key, i);
            Ok(i)
        };
        for (i, &[a, b, c]) in faces.iter().enumerate() {
            let ab = midpoint(a, b)?;
            let bc = midpoint(b, c)?;
            let ca = midpoint(c, a)?;
            next.extend([[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]]);
            flags.extend([active[i]; 4]);
        }
        faces = next;
        active = flags;
    }
    // Bound selection-border distance work separately from triangle refinement.
    if points.len().saturating_mul(boundary.len()) > 20_000_000 {
        return Err(input("Texture boundary distance budget exceeded."));
    }
    let mut normals = vec![[0.; 3]; points.len()];
    let mut enabled = vec![false; points.len()];
    for (i, f) in faces.iter().enumerate() {
        let n = cross(
            sub(points[f[1]], points[f[0]]),
            sub(points[f[2]], points[f[0]]),
        );
        finite(n)?;
        for &id in f {
            for k in 0..3 {
                normals[id][k] += n[k];
            }
            if active[i] {
                enabled[id] = true;
            }
        }
    }
    let mut displaced = points.clone();
    for (i, &p) in points.iter().enumerate() {
        if !enabled[i] {
            continue;
        }
        let mut fade = 1_f64;
        for &[a, b] in &boundary {
            let ab = sub(b, a);
            let denominator = dot(ab, ab);
            if !denominator.is_finite() || denominator <= 0. {
                return Err(input("Degenerate texture boundary."));
            }
            let t = (dot(sub(p, a), ab) / denominator).clamp(0., 1.);
            let d = norm(sub(p, std::array::from_fn(|k| a[k] + t * ab[k])));
            if !d.is_finite() {
                return Err(input("Texture exceeds finite numeric range."));
            }
            fade = fade.min(d / (o.pitch / 2.));
        }
        fade = fade * fade * (3. - 2. * fade);
        let length = norm(normals[i]);
        if !length.is_finite() || length < 1e-9 {
            return Err(input("Zero direction"));
        }
        let h = o.height(p)? * fade;
        if h != 0. {
            let q = std::array::from_fn(|k| round((p[k] + normals[i][k] / length * h) * 1e6) / 1e6);
            finite(q)?;
            displaced[i] = q;
        }
    }
    for f in &faces {
        let old = cross(
            sub(points[f[1]], points[f[0]]),
            sub(points[f[2]], points[f[0]]),
        );
        let next = cross(
            sub(displaced[f[1]], displaced[f[0]]),
            sub(displaced[f[2]], displaced[f[0]]),
        );
        let orientation = dot(old, next);
        if !orientation.is_finite() || orientation <= 0. {
            return Err(input(
                "Texture folds a triangle. Reduce height or increase pitch.",
            ));
        }
    }
    let result = Mesh {
        positions: displaced.into_iter().flatten().collect(),
        indices: faces.into_iter().flatten().collect(),
        uv: None,
    };
    let report = result.inspect()?;
    if !report.closed || !report.signed_volume_mm3.is_finite() || report.signed_volume_mm3 <= 0. {
        return Err(input("Invalid textured surface."));
    }
    let mut object = body
        .as_object()
        .ok_or_else(|| input("Expected body record"))?
        .clone();
    object.insert("mesh".into(), encode(result)?);
    Ok(Value::Object(object))
}
