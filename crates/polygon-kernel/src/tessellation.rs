//! Parametric surface meshing and polygonal UV clipping. The sampler contract
//! supports any surface implementation; this module knows nothing about NURBS.
use crate::{check, norm, sub, BuiltMesh, Construction, Error, Mesh, Result, Seams, MAX_TRIANGLES};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub type UV = [f64; 2];
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trim {
    pub outer: Vec<UV>,
    #[serde(default)]
    pub holes: Vec<Vec<UV>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    pub segments_u: usize,
    pub segments_v: usize,
    #[serde(default)]
    pub trim: Option<Trim>,
    #[serde(default)]
    pub max_triangles: Option<usize>,
}
/// Source-established closure hints. Samples are checked again before welding.
#[derive(Clone, Debug, Default)]
pub struct Boundary {
    pub seams: Seams,
    pub collapsed: [Option<[f64; 3]>; 4],
}
pub trait ParametricSurface {
    fn domain(&self) -> [f64; 4];
    fn point(&self, u: f64, v: f64) -> Result<[f64; 3]>;
    fn boundary(&self) -> Boundary {
        Boundary::default()
    }
}
const EPS: f64 = 1e-11;
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= EPS
}
fn same(a: UV, b: UV) -> bool {
    close(a[0], b[0]) && close(a[1], b[1])
}
fn cross2(a: UV, b: UV, c: UV) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn area(p: &[UV]) -> f64 {
    p.iter()
        .enumerate()
        .map(|(i, a)| {
            let b = p[(i + 1) % p.len()];
            (a[0] - p[0][0]) * (b[1] - p[0][1]) - (b[0] - p[0][0]) * (a[1] - p[0][1])
        })
        .sum::<f64>()
        / 2.
}
fn between(a: f64, low: f64, high: f64) -> bool {
    a >= low.min(high) - EPS && a <= low.max(high) + EPS
}
fn on_segment(p: UV, a: UV, b: UV) -> bool {
    cross2(a, b, p).abs() <= EPS * (b[0] - a[0]).hypot(b[1] - a[1])
        && between(p[0], a[0], b[0])
        && between(p[1], a[1], b[1])
}
fn intersects(a: UV, b: UV, c: UV, d: UV) -> bool {
    on_segment(a, c, d)
        || on_segment(b, c, d)
        || on_segment(c, a, b)
        || on_segment(d, a, b)
        || ((cross2(a, b, c) > 0.) != (cross2(a, b, d) > 0.)
            && (cross2(c, d, a) > 0.) != (cross2(c, d, b) > 0.))
}
fn inside(p: UV, poly: &[UV]) -> bool {
    let mut result = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[j];
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            result = !result;
        }
        j = i;
    }
    result
}
fn clean(poly: Vec<UV>) -> Vec<UV> {
    let result: Vec<_> = poly
        .iter()
        .enumerate()
        .filter(|(i, p)| !same(**p, poly[(i + poly.len() - 1) % poly.len()]))
        .map(|(_, p)| *p)
        .collect();
    if result.len() >= 3 && area(&result).abs() > EPS * EPS {
        result
    } else {
        vec![]
    }
}
pub(crate) fn validate_loops(loops: &[Vec<UV>]) -> Result<()> {
    check(
        loops.len() <= 17 && loops.iter().map(Vec::len).sum::<usize>() <= 512,
        "At most 16 holes and 512 total trim vertices are supported.",
    )?;
    for l in loops {
        check(
            l.len() >= 3,
            "Each trim loop needs at least three vertices.",
        )?;
        for p in l {
            check(
                p.iter().all(|v| v.is_finite()),
                "Trim vertices must be finite UV pairs.",
            )?;
            check(
                between(p[0], 0., 1.) && between(p[1], 0., 1.),
                "Trim lies outside the surface parameter domain.",
            )?;
        }
        check(area(l).abs() > EPS * EPS, "Trim loop has zero area.")?;
        for i in 0..l.len() {
            let a = l[i];
            let b = l[(i + 1) % l.len()];
            check(
                !same(a, b),
                "Trim loop has duplicate adjacent vertices; omit a repeated closing point.",
            )?;
            for j in i + 1..l.len() {
                if j == i + 1 || (i == 0 && j == l.len() - 1) {
                    continue;
                }
                check(
                    !intersects(a, b, l[j], l[(j + 1) % l.len()]),
                    "Trim loops must be simple and cannot self-intersect.",
                )?;
            }
        }
    }
    for i in 0..loops.len() {
        for j in i + 1..loops.len() {
            let a = &loops[i];
            let b = &loops[j];
            for k in 0..a.len() {
                for l in 0..b.len() {
                    check(
                        !intersects(a[k], a[(k + 1) % a.len()], b[l], b[(l + 1) % b.len()]),
                        "Trim loops cannot cross or touch one another.",
                    )?;
                }
            }
            if i > 0 {
                check(
                    !inside(a[0], b) && !inside(b[0], a),
                    "Holes cannot contain or overlap other holes.",
                )?;
            }
        }
    }
    check(
        loops.iter().skip(1).all(|l| inside(l[0], &loops[0])),
        "Every hole must lie strictly inside the outer loop.",
    )
}
fn unique(mut values: Vec<f64>) -> Vec<f64> {
    values.sort_by(f64::total_cmp);
    values
        .iter()
        .enumerate()
        .filter(|(i, v)| *i == 0 || !close(**v, values[i - 1]))
        .map(|(_, v)| *v)
        .collect()
}
fn clip(poly: Vec<UV>, y: f64, above: bool) -> Vec<UV> {
    let accepted = |p: UV| if above { p[1] >= y } else { p[1] <= y };
    let mut out = Vec::new();
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        if accepted(a) {
            out.push(a);
        }
        if accepted(a) != accepted(b) {
            out.push([a[0] + (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]), y]);
        }
    }
    clean(out)
}
fn key(v: f64) -> i64 {
    (v / EPS).round() as i64
}
fn vertex(p: UV, uv: &mut Vec<f64>, map: &mut BTreeMap<(i64, i64), usize>) -> usize {
    *map.entry((key(p[0]), key(p[1]))).or_insert_with(|| {
        let i = uv.len() / 2;
        uv.extend(p);
        i
    })
}
pub fn tessellate(surface: &impl ParametricSurface, options: &Options) -> Result<BuiltMesh> {
    check(
        (1..=128).contains(&options.segments_u) && (1..=128).contains(&options.segments_v),
        "Segment counts must be integers from 1 to 128.",
    )?;
    let budget = options.max_triangles.unwrap_or(MAX_TRIANGLES);
    check(
        (1..=MAX_TRIANGLES).contains(&budget),
        "Triangle budget must be an integer from 1 to 20000.",
    )?;
    let domain = surface.domain();
    check(
        domain.iter().all(|v| v.is_finite()) && domain[0] < domain[1] && domain[2] < domain[3],
        "Surface parameter domain must be finite and increasing.",
    )?;
    let su = domain[1] - domain[0];
    let sv = domain[3] - domain[2];
    check(
        su.is_finite() && sv.is_finite(),
        "Surface parameter span exceeded finite precision.",
    )?;
    let evaluate = |u: f64, v: f64| surface.point(domain[0] + u * su, domain[2] + v * sv);
    let loops = if let Some(trim) = &options.trim {
        std::iter::once(&trim.outer)
            .chain(&trim.holes)
            .map(|l| {
                l.iter()
                    .map(|p| [(p[0] - domain[0]) / su, (p[1] - domain[2]) / sv])
                    .collect()
            })
            .collect::<Vec<Vec<UV>>>()
    } else {
        vec![vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]]
    };
    validate_loops(&loops)?;
    let grid_v: Vec<f64> = (0..=options.segments_v)
        .map(|i| i as f64 / options.segments_v as f64)
        .collect();
    let xs = unique(
        (0..=options.segments_u)
            .map(|i| i as f64 / options.segments_u as f64)
            .chain(loops.iter().flatten().map(|p| p[0]))
            .collect(),
    );
    let edges: Vec<(UV, UV)> = loops
        .iter()
        .flat_map(|l| {
            l.iter()
                .enumerate()
                .map(|(i, a)| (*a, l[(i + 1) % l.len()]))
        })
        .collect();
    let y_at = |edge: &(UV, UV), x: f64| {
        edge.0[1] + (edge.1[1] - edge.0[1]) * (x - edge.0[0]) / (edge.1[0] - edge.0[0])
    };
    let mut cells = Vec::new();
    for pair in xs.windows(2) {
        let left = pair[0];
        let right = pair[1];
        let mid = (left + right) / 2.;
        let mut crossing: Vec<_> = edges
            .iter()
            .filter(|(a, b)| mid > a[0].min(b[0]) && mid < a[0].max(b[0]))
            .collect();
        crossing.sort_by(|a, b| y_at(a, mid).total_cmp(&y_at(b, mid)));
        check(
            crossing.len() % 2 == 0,
            "Trim decomposition has an unmatched crossing.",
        )?;
        for edges in crossing.chunks_exact(2) {
            let low = edges[0];
            let high = edges[1];
            let trapezoid = vec![
                [left, y_at(low, left)],
                [right, y_at(low, right)],
                [right, y_at(high, right)],
                [left, y_at(high, left)],
            ];
            for band in grid_v.windows(2) {
                let clipped = clip(clip(trapezoid.clone(), band[0], true), band[1], false);
                if !clipped.is_empty() {
                    cells.push(clipped);
                }
                check(cells.len()*3<=budget,"Trim tessellation exceeds the triangle budget; reduce segment counts or trim complexity.")?;
            }
        }
    }
    check(!cells.is_empty(), "Trim produced no surface area.")?;
    let mut vertical = BTreeMap::<_, Vec<_>>::new();
    let mut horizontal = BTreeMap::<_, Vec<_>>::new();
    for p in cells.iter().flatten() {
        vertical.entry(key(p[0])).or_default().push(p[1]);
        horizontal.entry(key(p[1])).or_default().push(p[0]);
    }
    for values in vertical.values_mut().chain(horizontal.values_mut()) {
        *values = unique(std::mem::take(values));
    }
    let mut uv = Vec::new();
    let mut indices = Vec::new();
    let mut map = BTreeMap::new();
    for cell in &cells {
        let mut boundary = Vec::new();
        for i in 0..cell.len() {
            let a = cell[i];
            let b = cell[(i + 1) % cell.len()];
            boundary.push(a);
            let mut extra = Vec::new();
            if close(a[0], b[0]) {
                if let Some(values) = vertical.get(&key(a[0])) {
                    extra = values
                        .iter()
                        .filter(|y| **y > a[1].min(b[1]) + EPS && **y < a[1].max(b[1]) - EPS)
                        .map(|y| [a[0], *y])
                        .collect();
                }
            } else if close(a[1], b[1]) {
                if let Some(values) = horizontal.get(&key(a[1])) {
                    extra = values
                        .iter()
                        .filter(|x| **x > a[0].min(b[0]) + EPS && **x < a[0].max(b[0]) - EPS)
                        .map(|x| [*x, a[1]])
                        .collect();
                }
            }
            extra.sort_by(|p, q| {
                ((p[0] - q[0]) * (b[0] - a[0]) + (p[1] - q[1]) * (b[1] - a[1])).total_cmp(&0.)
            });
            boundary.extend(extra);
        }
        let center = [
            cell.iter().map(|p| p[0]).sum::<f64>() / cell.len() as f64,
            cell.iter().map(|p| p[1]).sum::<f64>() / cell.len() as f64,
        ];
        let c = vertex(center, &mut uv, &mut map);
        for i in 0..boundary.len() {
            let a = boundary[i];
            let b = boundary[(i + 1) % boundary.len()];
            check(
                cross2(center, a, b) > EPS * EPS,
                "Trim cell degenerates at the chosen parameter scale.",
            )?;
            indices.extend([
                c,
                vertex(a, &mut uv, &mut map),
                vertex(b, &mut uv, &mut map),
            ]);
            check(indices.len()/3<=budget,"Trim tessellation exceeds the triangle budget; reduce segment counts or trim complexity.")?;
        }
    }
    let mut positions = Vec::new();
    for p in uv.chunks_exact(2) {
        positions.extend(evaluate(p[0], p[1])?);
    }
    let mut deviation = 0_f64;
    for t in indices.chunks_exact(3) {
        let actual = evaluate(
            t.iter().map(|i| uv[2 * i]).sum::<f64>() / 3.,
            t.iter().map(|i| uv[2 * i + 1]).sum::<f64>() / 3.,
        )?;
        let delta: [f64; 3] = std::array::from_fn(|a| {
            actual[a] - t.iter().map(|i| positions[3 * i + a]).sum::<f64>() / 3.
        });
        deviation = deviation.max(norm(&delta));
    }
    let mut mesh = Mesh {
        positions,
        indices,
        uv: Some(uv),
    };
    mesh.validate()?;
    let boundary = if options.trim.is_some() {
        Boundary::default()
    } else {
        surface.boundary()
    };
    weld(&mut mesh, &boundary)?;
    let mut report = mesh.inspect()?;
    if report.closed && report.signed_volume_mm3 < 0. {
        mesh.reverse_winding();
        report = mesh.inspect()?;
    }
    for (i, value) in mesh.uv.as_mut().unwrap().iter_mut().enumerate() {
        *value = if i % 2 == 0 {
            domain[0] + *value * su
        } else {
            domain[2] + *value * sv
        };
    }
    report.construction = Construction::SampledSurface;
    report.uv_area = Some(
        (area(&loops[0]).abs() - loops.iter().skip(1).map(|l| area(l).abs()).sum::<f64>())
            * su
            * sv,
    );
    report.sampled_deviation_mm = Some(deviation);
    report.parameter_seams_welded = Some(boundary.seams);
    report.collapsed_boundary_count =
        Some(boundary.collapsed.iter().filter(|p| p.is_some()).count());
    Ok(BuiltMesh { mesh, report })
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum WeldKey {
    UV(i64, i64),
    Pole([u64; 3]),
}
fn weld(mesh: &mut Mesh, boundary: &Boundary) -> Result<()> {
    if !boundary.seams.u && !boundary.seams.v && boundary.collapsed.iter().all(Option::is_none) {
        return Ok(());
    }
    let uv = mesh
        .uv
        .as_ref()
        .ok_or_else(|| Error::new("Welding requires UV coordinates."))?;
    let tolerance =
        mesh.positions.iter().map(|v| v.abs()).fold(1_f64, f64::max) * f64::EPSILON * 128.;
    let mut positions = Vec::new();
    let mut out_uv = Vec::new();
    let mut aliases = Vec::new();
    let mut by_key = BTreeMap::new();
    for (i, p) in uv.chunks_exact(2).enumerate() {
        let u = p[0];
        let v = p[1];
        let bounds = [close(u, 0.), close(u, 1.), close(v, 0.), close(v, 1.)];
        let pole = boundary
            .collapsed
            .iter()
            .zip(bounds)
            .find_map(|(p, on)| if on { *p } else { None });
        let k = if let Some(p) = pole {
            WeldKey::Pole(p.map(|v| if v == 0. { 0 } else { v.to_bits() }))
        } else {
            WeldKey::UV(
                key(if boundary.seams.u && bounds[1] { 0. } else { u }),
                key(if boundary.seams.v && bounds[3] { 0. } else { v }),
            )
        };
        let mapped = if let Some(&mapped) = by_key.get(&k) {
            let stored = [
                positions[3 * mapped],
                positions[3 * mapped + 1],
                positions[3 * mapped + 2],
            ];
            check(
                norm(&sub(mesh.point(i)?, stored)) <= tolerance,
                "Declared parameter seam does not close within floating-point tolerance.",
            )?;
            mapped
        } else {
            let mapped = positions.len() / 3;
            by_key.insert(k, mapped);
            positions.extend(mesh.point(i)?);
            out_uv.extend([u, v]);
            mapped
        };
        aliases.push(mapped);
    }
    let mut indices = Vec::new();
    for t in mesh.indices.chunks_exact(3) {
        let a = aliases[t[0]];
        let b = aliases[t[1]];
        let c = aliases[t[2]];
        if a != b && b != c && c != a {
            indices.extend([a, b, c]);
        }
    }
    *mesh = Mesh {
        positions,
        indices,
        uv: Some(out_uv),
    };
    Ok(())
}
