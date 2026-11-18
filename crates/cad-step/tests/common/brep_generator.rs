//! Deterministic seeded generator of valid B-rep solids and the oracles
//! that check them without reusing kernel bookkeeping. Shared by the
//! generator oracle test and the exchange round-trip corpus.
use brep_core::Model;
pub type Result<T> = std::result::Result<T, nurbs_core::Error>;
use std::collections::BTreeMap;
use std::f64::consts::PI;
pub struct Lcg(pub u64);
impl Lcg {
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 11
    }
    /// Uniform in [lo, hi), quantized to 1/64 so cases print exactly.
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        let steps = ((hi - lo) * 64.).max(1.) as u64;
        lo + (self.next() % steps) as f64 / 64.
    }
    pub fn pick(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

#[derive(Debug, Clone)]
pub enum Case {
    Box { min: [f64; 3], max: [f64; 3] },
    Cylinder { radius: f64, height: f64, offset: [f64; 3] },
    /// Full revolution of the rectangle r in [r0, r1], z in [z0, z1].
    Revolve { r0: f64, r1: f64, z0: f64, z1: f64 },
    /// Union of two boxes that overlap in a box.
    BoxUnion { a: ([f64; 3], [f64; 3]), b: ([f64; 3], [f64; 3]) },
    /// Box minus a strictly interior smaller box (a cavity-free pocket that
    /// opens through the top face).
    BoxPocket { outer: ([f64; 3], [f64; 3]), cut: ([f64; 3], [f64; 3]) },
    /// Box [0,a]x[0,b]x[0,c] drilled through along z by a centred cylinder:
    /// a curved Boolean with genus 1.
    DrilledBox { size: [f64; 3], radius: f64 },
}

pub struct Expect {
    pub volume: f64,
    pub area: Option<f64>,
    pub centroid: Option<[f64; 3]>,
    pub shells: i64,
    pub genus: i64,
}

pub fn box_volume((min, max): ([f64; 3], [f64; 3])) -> f64 {
    (0..3).map(|i| max[i] - min[i]).product()
}
pub fn box_area((min, max): ([f64; 3], [f64; 3])) -> f64 {
    let d: Vec<f64> = (0..3).map(|i| max[i] - min[i]).collect();
    2. * (d[0] * d[1] + d[1] * d[2] + d[2] * d[0])
}
pub fn box_center((min, max): ([f64; 3], [f64; 3])) -> [f64; 3] {
    std::array::from_fn(|i| (min[i] + max[i]) / 2.)
}

pub fn generate(rng: &mut Lcg) -> Case {
    match rng.pick(6) {
        0 => {
            let min = [rng.range(-5., 5.), rng.range(-5., 5.), rng.range(-5., 5.)];
            let max = std::array::from_fn(|i| min[i] + rng.range(0.25, 6.));
            Case::Box { min, max }
        }
        1 => Case::Cylinder {
            radius: rng.range(0.25, 4.),
            height: rng.range(0.25, 6.),
            offset: [rng.range(-5., 5.), rng.range(-5., 5.), rng.range(-5., 5.)],
        },
        2 => {
            let r0 = rng.range(0.25, 3.);
            let z0 = rng.range(-3., 3.);
            Case::Revolve { r0, r1: r0 + rng.range(0.25, 3.), z0, z1: z0 + rng.range(0.25, 4.) }
        }
        3 => {
            let a_min = [rng.range(-3., 0.), rng.range(-3., 0.), rng.range(-3., 0.)];
            let a_max: [f64; 3] = std::array::from_fn(|i| a_min[i] + rng.range(1., 4.));
            // b starts strictly inside a on every axis and ends strictly past
            // it, so the overlap is a box and no faces are coplanar.
            let b_min: [f64; 3] = std::array::from_fn(|i| {
                a_min[i] + (a_max[i] - a_min[i]) * rng.range(0.25, 0.75)
            });
            let b_max = std::array::from_fn(|i| a_max[i] + rng.range(0.25, 2.));
            Case::BoxUnion { a: (a_min, a_max), b: (b_min, b_max) }
        }
        5 => {
            let size = [rng.range(1., 5.), rng.range(1., 5.), rng.range(0.5, 4.)];
            let radius = rng.range(0.125, size[0].min(size[1]) * 0.4);
            Case::DrilledBox { size, radius }
        }
        _ => {
            let min = [rng.range(-3., 0.), rng.range(-3., 0.), rng.range(-3., 0.)];
            let max: [f64; 3] = std::array::from_fn(|i| min[i] + rng.range(2., 5.));
            let cut_min = [
                min[0] + rng.range(0.25, 0.75),
                min[1] + rng.range(0.25, 0.75),
                min[2] + rng.range(0.25, 0.75),
            ];
            let cut_max = [
                max[0] - rng.range(0.25, 0.75),
                max[1] - rng.range(0.25, 0.75),
                max[2] + rng.range(0.25, 1.),
            ];
            Case::BoxPocket { outer: (min, max), cut: (cut_min, cut_max) }
        }
    }
}

pub fn translate(model: &Model, t: [f64; 3]) -> Result<Model> {
    brep_core::transform::affine(
        model,
        [[1., 0., 0., t[0]], [0., 1., 0., t[1]], [0., 0., 1., t[2]], [0., 0., 0., 1.]],
    )
}

pub fn build(case: &Case) -> Result<(Model, Expect)> {
    Ok(match case {
        Case::Box { min, max } => (
            brep_core::cuboid(*min, *max)?,
            Expect {
                volume: box_volume((*min, *max)),
                area: Some(box_area((*min, *max))),
                centroid: Some(box_center((*min, *max))),
                shells: 1,
                genus: 0,
            },
        ),
        Case::Cylinder { radius, height, offset } => {
            let model = translate(&brep_core::analytic::cylinder(*radius, *height)?, *offset)?;
            (
                model,
                Expect {
                    volume: PI * radius * radius * height,
                    area: Some(2. * PI * radius * (radius + height)),
                    centroid: Some([offset[0], offset[1], offset[2] + height / 2.]),
                    shells: 1,
                    genus: 0,
                },
            )
        }
        Case::Revolve { r0, r1, z0, z1 } => {
            let profile = [[*r0, *z0], [*r1, *z0], [*r1, *z1], [*r0, *z1]];
            let h = z1 - z0;
            (
                brep_core::analytic::revolve(&profile)?,
                Expect {
                    volume: PI * (r1 * r1 - r0 * r0) * h,
                    area: Some(2. * PI * (r0 + r1) * h + 2. * PI * (r1 * r1 - r0 * r0)),
                    centroid: Some([0., 0., (z0 + z1) / 2.]),
                    shells: 1,
                    genus: 1,
                },
            )
        }
        Case::BoxUnion { a, b } => {
            let overlap = (
                std::array::from_fn(|i| a.0[i].max(b.0[i])),
                std::array::from_fn(|i| a.1[i].min(b.1[i])),
            );
            let model = brep_core::operations::boolean(
                &brep_core::cuboid(a.0, a.1)?,
                &brep_core::cuboid(b.0, b.1)?,
                "union",
            )?;
            // b's min corner lies inside a and its max corner outside, so
            // exactly the half of each operand's overlap boundary that lies
            // inside the other operand is hidden: area(A) + area(B) - area(O).
            let area = box_area(*a) + box_area(*b) - box_area(overlap);
            (
                model,
                Expect {
                    volume: box_volume(*a) + box_volume(*b) - box_volume(overlap),
                    area: Some(area),
                    centroid: None,
                    shells: 1,
                    genus: 0,
                },
            )
        }
        Case::DrilledBox { size, radius } => {
            let [a, b, c] = *size;
            let drill = translate(
                &brep_core::analytic::cylinder(*radius, c + 2.)?,
                [a / 2., b / 2., -1.],
            )?;
            let model = brep_core::operations::boolean(
                &brep_core::cuboid([0.; 3], *size)?,
                &drill,
                "difference",
            )?;
            (
                model,
                Expect {
                    volume: a * b * c - PI * radius * radius * c,
                    area: Some(box_area(([0.; 3], *size)) - 2. * PI * radius * radius + 2. * PI * radius * c),
                    centroid: Some([a / 2., b / 2., c / 2.]),
                    shells: 1,
                    genus: 1,
                },
            )
        }
        Case::BoxPocket { outer, cut } => {
            let clipped = (cut.0, std::array::from_fn(|i| cut.1[i].min(outer.1[i])));
            let model = brep_core::operations::boolean(
                &brep_core::cuboid(outer.0, outer.1)?,
                &brep_core::cuboid(cut.0, cut.1)?,
                "difference",
            )?;
            let d: Vec<f64> = (0..3).map(|i| clipped.1[i] - clipped.0[i]).collect();
            // Outer box area, minus the top opening, plus the pocket walls
            // and floor.
            let area = box_area(*outer) - d[0] * d[1] + 2. * (d[0] + d[1]) * d[2] + d[0] * d[1];
            (
                model,
                Expect {
                    volume: box_volume(*outer) - box_volume(clipped),
                    area: Some(area),
                    centroid: None,
                    shells: 1,
                    genus: 0,
                },
            )
        }
    })
}

/// Topological oracles computed straight from the entity tables.
pub fn topology(model: &Model, expect: &Expect) -> Vec<String> {
    let mut issues = Vec::new();
    if model.bodies.is_empty() {
        issues.push("no bodies".into());
    }
    if let Some(open) = model.shells.iter().position(|s| !s.closed) {
        issues.push(format!("shell {open} is not closed"));
    }
    // A shell may use a face reversed, which flips every coedge of that face.
    let mut uses: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
    for face_use in model.shells.iter().flat_map(|s| &s.faces) {
        let face = &model.faces[face_use.face];
        for &l in std::iter::once(&face.outer).chain(&face.holes) {
            for coedge in &model.loops[l].coedges {
                let entry = uses.entry(coedge.edge).or_default();
                if coedge.reversed != face_use.reversed {
                    entry.1 += 1;
                } else {
                    entry.0 += 1;
                }
            }
        }
    }
    for (edge, (forward, reverse)) in &uses {
        if (*forward, *reverse) != (1, 1) {
            issues.push(format!("edge {edge} used {forward} forward / {reverse} reversed"));
        }
    }
    let v = model.vertices.len() as i64;
    let e = model.edges.len() as i64;
    let f = model.faces.len() as i64;
    let l = model.loops.len() as i64;
    let s = model.shells.len() as i64;
    if s != expect.shells {
        issues.push(format!("expected {} shells, found {s}", expect.shells));
    }
    let chi = v - e + 2 * f - l;
    if chi != 2 * (expect.shells - expect.genus) {
        issues.push(format!(
            "Euler-Poincare: V={v} E={e} F={f} L={l} gives V-E+2F-L={chi}, expected 2(S-G)={}",
            2 * (expect.shells - expect.genus)
        ));
    }
    issues
}

pub fn metric(model: &Model, expect: &Expect) -> Vec<String> {
    let mut issues = Vec::new();
    let props = match brep_core::analysis::mass_properties(model, 1e-8, 400_000) {
        Ok(props) => props,
        Err(error) => return vec![format!("mass properties refused: {error:?}")],
    };
    let close = |actual: f64, expected: f64| (actual - expected).abs() <= 1e-5 * expected.abs().max(1.);
    if !close(props.signed_volume_mm3, expect.volume) {
        issues.push(format!("volume {} != analytic {}", props.signed_volume_mm3, expect.volume));
    }
    if let Some(area) = expect.area
        && !close(props.surface_area_mm2, area)
    {
        issues.push(format!("area {} != analytic {area}", props.surface_area_mm2));
    }
    if let Some(c) = expect.centroid
        && (0..3).any(|i| (props.centroid[i] - c[i]).abs() > 1e-5 * (1. + c[i].abs()))
    {
        issues.push(format!("centroid {:?} != analytic {c:?}", props.centroid));
    }
    issues
}

impl Case {
    pub fn kind(&self) -> &'static str {
        match self {
            Case::Box { .. } => "box",
            Case::Cylinder { .. } => "cylinder",
            Case::Revolve { .. } => "revolve",
            Case::BoxUnion { .. } => "box-union",
            Case::BoxPocket { .. } => "box-pocket",
            Case::DrilledBox { .. } => "drilled-box",
        }
    }
}
