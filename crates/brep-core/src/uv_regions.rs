//! Planar UV region arrangement shared by the curved imprint families: a
//! small DCEL over trimmed-face boundary pieces and section pcurves.
//!
//! Input is a set of undirected pieces between chart vertices, each with its
//! forward pcurve over [0, 1] and an opaque key naming the 3D edge it maps
//! to. Half-edges are ordered around every vertex by tangent angle; walking
//! `next` keeps the bounded region on the left, so counter-clockwise cycles
//! bound regions and clockwise cycles are holes of the smallest region
//! containing a point just to their left (or the chart exterior). Tangential
//! branches at a vertex, dangling vertices and non-simple walks refuse.
use nurbs_core::{Error, Result, curve::Curve};
use std::collections::BTreeMap;

const TAU: f64 = std::f64::consts::TAU;
/// Offsets tried when sampling a UV region interior.
const SAMPLE_DELTAS: [f64; 4] = [1e-2, 1e-3, 1e-4, 1e-5];

fn unsupported(message: &str) -> Error {
    Error::new("BREP_UNSUPPORTED_OPERATION", message)
}

/// One undirected UV piece of a chart arrangement.
pub(crate) struct Piece<K> {
    pub(crate) v: [usize; 2],
    /// Forward pcurve over [0, 1], from v[0] to v[1].
    pub(crate) pcurve: Curve,
    pub(crate) key: K,
    /// Forward direction runs against the 3D edge.
    pub(crate) reversed: bool,
}

struct Half {
    piece: usize,
    forward: bool,
    from: usize,
    to: usize,
    /// Outgoing tangent angle at `from`.
    angle: f64,
}

pub(crate) struct Region {
    pub(crate) outer: usize,
    pub(crate) holes: Vec<usize>,
}

pub(crate) struct Arrangement<K> {
    pieces: Vec<Piece<K>>,
    halves: Vec<Half>,
    /// Half-edge ids of each cycle, in traversal order.
    cycles: Vec<Vec<usize>>,
    /// Sampled polygon of each cycle.
    polygons: Vec<Vec<[f64; 2]>>,
    pub(crate) regions: Vec<Region>,
}

impl<K> Arrangement<K> {
    /// Pieces of a cycle in traversal order, with whether each is walked
    /// along its forward pcurve.
    pub(crate) fn cycle_pieces(&self, cycle: usize) -> impl Iterator<Item = (&Piece<K>, bool)> {
        self.cycles[cycle].iter().map(move |&h| {
            let half = &self.halves[h];
            (&self.pieces[half.piece], half.forward)
        })
    }
}

pub(crate) fn pcurve_point(curve: &Curve, t: f64) -> Result<[f64; 2]> {
    let p = curve.evaluate(t)?.point;
    Ok([p[0], p[1]])
}

/// The same curve over the knot domain [0, 1]; `Curve::trim` keeps the
/// sub-domain of its source, and every chart routine here samples in [0, 1].
pub(crate) fn normalized(curve: Curve) -> Curve {
    let [a, b] = curve.domain();
    if a == 0. && b == 1. {
        return curve;
    }
    let mut out = curve;
    out.knots = out.knots.iter().map(|k| (k - a) / (b - a)).collect();
    out
}

fn tangent_angle(curve: &Curve, at_start: bool) -> Result<f64> {
    let n = curve.control_points.len();
    let (p, q) = if at_start {
        (&curve.control_points[0], &curve.control_points[1])
    } else {
        (&curve.control_points[n - 1], &curve.control_points[n - 2])
    };
    let d = [q[0] - p[0], q[1] - p[1]];
    let len = d[0].hypot(d[1]);
    if !(len > 1e-12) {
        return Err(unsupported(
            "UV regions: degenerate pcurve tangent in the UV arrangement",
        ));
    }
    Ok(d[1].atan2(d[0]))
}

fn signed_area(polygon: &[[f64; 2]]) -> f64 {
    let mut area = 0.;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        area += a[0] * b[1] - b[0] * a[1];
    }
    area / 2.
}

fn point_in_polygon(polygon: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut winding = 0i32;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[(i + 1) % polygon.len()];
        let side = (b[0] - a[0]) * (p[1] - a[1]) - (p[0] - a[0]) * (b[1] - a[1]);
        if a[1] <= p[1] {
            if b[1] > p[1] && side > 0. {
                winding += 1;
            }
        } else if b[1] <= p[1] && side < 0. {
            winding -= 1;
        }
    }
    winding != 0
}

pub(crate) fn arrange<K>(pieces: Vec<Piece<K>>, vertex_count: usize) -> Result<Arrangement<K>> {
    let mut halves = Vec::with_capacity(pieces.len() * 2);
    for (i, piece) in pieces.iter().enumerate() {
        halves.push(Half {
            piece: i,
            forward: true,
            from: piece.v[0],
            to: piece.v[1],
            angle: tangent_angle(&piece.pcurve, true)?,
        });
        halves.push(Half {
            piece: i,
            forward: false,
            from: piece.v[1],
            to: piece.v[0],
            angle: tangent_angle(&piece.pcurve, false)?,
        });
    }
    let mut outgoing: Vec<Vec<(f64, usize)>> = vec![Vec::new(); vertex_count];
    for (h, half) in halves.iter().enumerate() {
        outgoing[half.from].push((half.angle, h));
    }
    for list in outgoing.iter_mut() {
        if list.len() < 2 {
            return Err(unsupported(
                "UV regions: dangling vertex in the UV arrangement",
            ));
        }
        list.sort_by(|a, b| a.0.total_cmp(&b.0));
        for w in 0..list.len() {
            let a = list[w].0;
            let b = if w + 1 == list.len() {
                list[0].0 + TAU
            } else {
                list[w + 1].0
            };
            if b - a < 1e-9 {
                return Err(unsupported(
                    "UV regions: two UV branches leave a vertex tangentially",
                ));
            }
        }
    }
    // next(h): at v = to(h), the outgoing half-edge clockwise-adjacent to
    // twin(h), which keeps the bounded region on the left.
    let mut next = vec![usize::MAX; halves.len()];
    for h in 0..halves.len() {
        let twin = h ^ 1;
        let v = halves[h].to;
        let list = &outgoing[v];
        let idx = list
            .iter()
            .position(|&(_, id)| id == twin)
            .ok_or_else(|| unsupported("UV regions: twin half-edge missing"))?;
        next[h] = list[(idx + list.len() - 1) % list.len()].1;
    }
    let mut cycles: Vec<Vec<usize>> = Vec::new();
    let mut visited = vec![false; halves.len()];
    for start in 0..halves.len() {
        if visited[start] {
            continue;
        }
        let mut cycle = Vec::new();
        let mut h = start;
        loop {
            if visited[h] {
                return Err(unsupported(
                    "UV regions: UV arrangement walk is not a simple cycle",
                ));
            }
            visited[h] = true;
            cycle.push(h);
            h = next[h];
            if h == start {
                break;
            }
        }
        cycles.push(cycle);
    }
    let mut polygons = Vec::with_capacity(cycles.len());
    let mut areas = Vec::with_capacity(cycles.len());
    for cycle in &cycles {
        let mut polygon = Vec::new();
        for &h in cycle {
            let piece = &pieces[halves[h].piece];
            for s in 0..16 {
                let t = s as f64 / 16.;
                let t = if halves[h].forward { t } else { 1. - t };
                polygon.push(pcurve_point(&piece.pcurve, t)?);
            }
        }
        areas.push(signed_area(&polygon));
        polygons.push(polygon);
    }
    // CCW cycles bound regions; CW cycles are holes of the smallest CCW
    // cycle containing a point just to their left, or the chart exterior.
    let mut regions: Vec<Region> = Vec::new();
    let mut region_of_cycle: BTreeMap<usize, usize> = BTreeMap::new();
    for (c, &area) in areas.iter().enumerate() {
        if area > 0. {
            region_of_cycle.insert(c, regions.len());
            regions.push(Region {
                outer: c,
                holes: Vec::new(),
            });
        }
    }
    for (c, &area) in areas.iter().enumerate() {
        if area > 0. {
            continue;
        }
        let probe = left_probe(&pieces, &halves, &cycles[c], &polygons, None)?;
        let mut best: Option<(f64, usize)> = None;
        for (&outer, &r) in &region_of_cycle {
            if point_in_polygon(&polygons[outer], probe) {
                let a = areas[outer];
                if best.map(|(ba, _)| a < ba).unwrap_or(true) {
                    best = Some((a, r));
                }
            }
        }
        if let Some((_, r)) = best {
            regions[r].holes.push(c);
        }
    }
    Ok(Arrangement {
        pieces,
        halves,
        cycles,
        polygons,
        regions,
    })
}

/// A point just left of some half-edge of a cycle; with a constraint it must
/// lie inside that outer polygon and outside the listed holes.
fn left_probe<K>(
    pieces: &[Piece<K>],
    halves: &[Half],
    cycle: &[usize],
    polygons: &[Vec<[f64; 2]>],
    constraint: Option<(usize, &[usize])>,
) -> Result<[f64; 2]> {
    for &h in cycle {
        let half = &halves[h];
        let piece = &pieces[half.piece];
        let (t0, t1) = if half.forward {
            (0.5, 0.5 + 1e-4)
        } else {
            (0.5, 0.5 - 1e-4)
        };
        let m = pcurve_point(&piece.pcurve, t0)?;
        let q = pcurve_point(&piece.pcurve, t1)?;
        let d = [q[0] - m[0], q[1] - m[1]];
        let len = d[0].hypot(d[1]);
        if !(len > 0.) {
            continue;
        }
        let left = [-d[1] / len, d[0] / len];
        for delta in SAMPLE_DELTAS {
            let p = [m[0] + left[0] * delta, m[1] + left[1] * delta];
            match constraint {
                None => return Ok(p),
                Some((outer, holes)) => {
                    if point_in_polygon(&polygons[outer], p)
                        && holes
                            .iter()
                            .all(|&hole| !point_in_polygon(&polygons[hole], p))
                    {
                        return Ok(p);
                    }
                }
            }
        }
    }
    Err(unsupported(
        "UV regions: could not sample a UV region interior",
    ))
}

pub(crate) fn region_sample<K>(arrangement: &Arrangement<K>, region: &Region) -> Result<[f64; 2]> {
    left_probe(
        &arrangement.pieces,
        &arrangement.halves,
        &arrangement.cycles[region.outer],
        &arrangement.polygons,
        Some((region.outer, &region.holes)),
    )
}
