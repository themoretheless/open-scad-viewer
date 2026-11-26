use super::{
    Coedge, Edge, Face, FaceUse, Loop, Model, Result, Shell, TopologyIds, Vertex, invalid,
    unsupported,
};
use crate::closed_solid::QualifiedClosedSolid;
use crate::model::planar_pcurve_regions::representable;
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};
use nurbs_core::surface_injectivity::certify_affine_parameterization;

mod curved;

type Point = [Q; 3];

fn difference(a: &Point, b: &Point) -> Point {
    std::array::from_fn(|i| &a[i] - &b[i])
}

fn cross(a: &Point, b: &Point) -> Point {
    std::array::from_fn(|i| {
        let j = (i + 1) % 3;
        let k = (i + 2) % 3;
        &a[j] * &b[k] - &a[k] * &b[j]
    })
}

fn dot(a: &Point, b: &Point) -> Q {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn edges<T>(ring: &[T]) -> impl Iterator<Item = (&T, &T)> {
    ring.iter().zip(ring.iter().cycle().skip(1))
}

fn oriented_normal(ring: &[Point]) -> Point {
    let mut normal = std::array::from_fn(|_| Q::zero());
    for (a, b) in edges(ring) {
        for (coordinate, term) in normal.iter_mut().zip(cross(a, b)) {
            *coordinate += term;
        }
    }
    normal
}

fn support_value(a: &Point, direction: &Point, normal: &Point, p: &Point) -> Q {
    dot(&cross(direction, &difference(p, a)), normal)
}

fn exact_point([x, y, z]: [f64; 3]) -> Result<Point> {
    let exact = |v| Q::from_float(v).ok_or_else(|| invalid("Nonfinite polygon vertex"));
    Ok([exact(x)?, exact(y)?, exact(z)?])
}

/// Intersect two convex, planar world-space rings with exact arithmetic.
/// The output follows the winding of the first ring. Point and edge contacts
/// produce an empty ring. Fractional vertices remain rational for BRep sewing.
pub fn exact_convex_coplanar_overlap(
    first: &[[f64; 3]],
    second: &[[f64; 3]],
    max_tests: usize,
) -> Result<Vec<[Q; 3]>> {
    let convert = |ring: &[[f64; 3]]| -> Result<Vec<Point>> {
        if ring.len() < 3 || ring.len() > max_tests {
            return Err(invalid(
                "Need a bounded polygon with at least three vertices",
            ));
        }
        ring.iter().copied().map(exact_point).collect()
    };
    let first = convert(first)?;
    let second = convert(second)?;
    let normal = oriented_normal(&first);
    let second_normal = oriented_normal(&second);
    if normal.iter().all(Zero::is_zero) || second_normal.iter().all(Zero::is_zero) {
        return Err(unsupported("Degenerate overlap polygon"));
    }
    if first
        .iter()
        .chain(&second)
        .any(|p| !dot(&difference(p, &first[0]), &normal).is_zero())
    {
        return Err(unsupported(
            "Overlap polygons are on different exact planes",
        ));
    }
    let mut remaining = max_tests;
    let mut spend = |tests| -> Result<()> {
        remaining = remaining.checked_sub(tests).ok_or_else(|| {
            crate::Error::new(
                "BREP_RESOURCE_LIMIT",
                "Exact polygon overlap exceeded its work budget",
            )
        })?;
        Ok(())
    };
    for (ring, normal) in [(&first, &normal), (&second, &second_normal)] {
        spend(
            ring.len()
                .checked_mul(ring.len())
                .ok_or_else(|| invalid("Invalid polygon size"))?,
        )?;
        for (a, b) in edges(ring) {
            let direction = difference(b, a);
            if a == b
                || ring
                    .iter()
                    .any(|p| support_value(a, &direction, normal, p).is_negative())
            {
                return Err(unsupported(
                    "Need consistently ordered convex overlap polygons",
                ));
            }
        }
        let mut unique: Vec<_> = ring.iter().collect();
        unique.sort_unstable();
        if unique.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(unsupported("A polygon vertex occurs more than once"));
        }
    }
    let mut overlap = first;
    for (a, b) in edges(&second) {
        if overlap.len() < 3 {
            return Ok(Vec::new());
        }
        spend(overlap.len())?;
        let direction = difference(b, a);
        let signed: Vec<_> = overlap
            .iter()
            .map(|p| support_value(a, &direction, &second_normal, p))
            .collect();
        let mut clipped = Vec::new();
        for ((p, q), (sp, sq)) in edges(&overlap).zip(edges(&signed)) {
            if !sp.is_negative() {
                clipped.push(p.clone());
            }
            if (sp.is_positive() && sq.is_negative()) || (sp.is_negative() && sq.is_positive()) {
                let t = sp / (sp - sq);
                clipped.push(std::array::from_fn(|i| &p[i] + &t * (&q[i] - &p[i])));
            }
        }
        overlap = clipped;
    }
    if oriented_normal(&overlap).iter().all(Zero::is_zero) {
        overlap.clear();
    }
    Ok(overlap)
}

struct AffineChart {
    origin: Point,
    u: Point,
    v: Point,
    axis: usize,
    determinant: Q,
    domain: [[f64; 2]; 2],
}

impl AffineChart {
    fn inverse(&self, point: &Point) -> Result<[Q; 2]> {
        let offset = difference(point, &self.origin);
        let s = &cross(&offset, &self.v)[self.axis] / &self.determinant;
        let t = &cross(&self.u, &offset)[self.axis] / &self.determinant;
        if (0..3).any(|i| offset[i] != &s * &self.u[i] + &t * &self.v[i]) {
            return Err(unsupported(
                "Boundary point lies outside the exact support plane",
            ));
        }
        let [u, v] = self
            .domain
            .map(|range| range.map(|x| Q::from_float(x).unwrap()));
        Ok([&u[0] + s * (&u[1] - &u[0]), &v[0] + t * (&v[1] - &v[0])])
    }

    fn lift(&self, [u, v]: &[Q; 2]) -> Point {
        let [s, t] = std::array::from_fn(|axis| {
            let x = [u, v][axis];
            let [lo, hi] = self.domain[axis];
            let [lo, hi] = [lo, hi].map(|value| Q::from_float(value).unwrap());
            (x - &lo) / (hi - lo)
        });
        std::array::from_fn(|k| &self.origin[k] + &s * &self.u[k] + &t * &self.v[k])
    }

    fn restrict(&self, surface: &crate::Surface, uv: &[Vec<f64>]) -> Result<crate::Surface> {
        let bounds = [0, 1].map(|k| {
            uv.iter()
                .fold([f64::INFINITY, f64::NEG_INFINITY], |[lo, hi], p| {
                    [lo.min(p[k]), hi.max(p[k])]
                })
        });
        if bounds == self.domain {
            return Ok(surface.clone());
        }
        let fraction = |x, [lo, hi]: [f64; 2]| {
            let [x, lo, hi] = [x, lo, hi].map(|v| Q::from_float(v).unwrap());
            (x - &lo) / (hi - lo)
        };
        let mut control_points = Vec::with_capacity(2);
        for x in bounds[0] {
            let s = fraction(x, self.domain[0]);
            let mut row = Vec::with_capacity(2);
            for y in bounds[1] {
                let t = fraction(y, self.domain[1]);
                let point = (0..3)
                    .map(|k| representable(&(&self.origin[k] + &s * &self.u[k] + &t * &self.v[k])))
                    .collect::<Result<Vec<_>>>()?;
                row.push(point);
            }
            control_points.push(row);
        }
        let knots = |[a, b]: [f64; 2]| vec![a, a, b, b];
        Ok(crate::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: knots(bounds[0]),
            knots_v: knots(bounds[1]),
            control_points,
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        })
    }
}

struct SourceFace {
    chart: AffineChart,
    reversed: bool,
}

fn source_face(model: &Model, face: usize, max_tests: usize) -> Result<SourceFace> {
    let f = model
        .faces
        .get(face)
        .ok_or_else(|| invalid("Unknown overlap face"))?;
    let use_ = model
        .shells
        .iter()
        .flat_map(|s| &s.faces)
        .find(|u| u.face == face)
        .ok_or_else(|| invalid("Overlap face has no shell use"))?;
    let affine = certify_affine_parameterization(&f.surface)
        .ok_or_else(|| unsupported("Exact convex overlap needs an affine UV chart"))?;
    let corner =
        |i: usize, j: usize| exact_point(std::array::from_fn(|k| affine.control_points[i][j][k]));
    let origin = corner(0, 0)?;
    let u = difference(&corner(1, 0)?, &origin);
    let v = difference(&corner(0, 1)?, &origin);
    let normal = cross(&u, &v);
    let axis = normal
        .iter()
        .position(|v| !v.is_zero())
        .ok_or_else(|| unsupported("Singular overlap chart"))?;
    let domain = [
        [affine.knots_u[1], affine.knots_u[2]],
        [affine.knots_v[1], affine.knots_v[2]],
    ];
    let chart = AffineChart {
        origin,
        u,
        v,
        axis,
        determinant: normal[axis].clone(),
        domain,
    };
    let loops = || std::iter::once(f.outer).chain(f.holes.iter().copied());
    if loops()
        .map(|id| model.loops[id].coedges.len())
        .sum::<usize>()
        > max_tests
    {
        return Err(crate::Error::new(
            "BREP_RESOURCE_LIMIT",
            "Exact overlap exceeded its source edge budget",
        ));
    }
    Ok(SourceFace {
        chart,
        reversed: use_.reversed,
    })
}

fn checked_ring(model: &Model, ring: &Loop, chart: &AffineChart) -> Result<Vec<[f64; 3]>> {
    let ring = &ring.coedges;
    let mut points = Vec::with_capacity(ring.len());
    for coedge in ring {
        let edge = &model.edges[coedge.edge];
        if coedge.trim().is_some()
            || edge.degenerate
            || edge.curve.degree != 1
            || coedge.pcurve.degree != 1
            || edge.curve.periodic
            || coedge.pcurve.periodic
            || edge.curve.control_points.len() != 2
            || coedge.pcurve.control_points.len() != 2
        {
            return Err(unsupported("Exact overlap needs full straight coedges"));
        }
        let world = edge.vertices.map(|v| model.vertices[v].point);
        for (i, p) in world.iter().enumerate() {
            let uv = chart.inverse(&exact_point(*p)?)?;
            let authored = &coedge.pcurve.control_points[i ^ usize::from(coedge.reversed)];
            if edge.curve.control_points[i].as_slice() != p
                || (0..2).any(|k| Q::from_float(authored[k]).as_ref() != Some(&uv[k]))
            {
                return Err(unsupported(
                    "Source coedge endpoints disagree with the exact affine chart",
                ));
            }
        }
        points.push(world[usize::from(coedge.reversed)]);
    }
    Ok(points)
}

/// Build the common outward patches of two qualified bodies on a
/// co-oriented affine plane. Straight trims can produce holes and multiple
/// components in the first UV chart. Strictly separated curved boundaries
/// retain the outer loop's original chart and preserve the spatial curves.
/// Isolated contacts between equally oriented charts can be split and sewn
/// when their parameters and retained fragments are exactly representable.
pub fn exact_coplanar_intersection_patches(
    solids: [&QualifiedClosedSolid; 2],
    faces: [usize; 2],
    max_tests: usize,
) -> Result<Vec<Model>> {
    let [first, second] = solids.map(QualifiedClosedSolid::model);
    let a = source_face(first, faces[0], max_tests)?;
    let b = source_face(second, faces[1], max_tests)?;
    let opposite_charts = a.reversed != b.reversed;
    let normal = dot(
        &cross(&a.chart.u, &a.chart.v),
        &cross(&b.chart.u, &b.chart.v),
    );
    if (opposite_charts && !normal.is_negative()) || (!opposite_charts && !normal.is_positive()) {
        return Err(unsupported(
            "Exact intersection patch needs co-oriented material",
        ));
    }

    let rings = |model: &Model, face: usize, chart: &AffineChart| {
        let f = &model.faces[face];
        std::iter::once(f.outer)
            .chain(f.holes.iter().copied())
            .map(|id| checked_ring(model, &model.loops[id], chart))
            .collect::<Result<Vec<_>>>()
    };
    let (first_rings, second_rings) = match (
        rings(first, faces[0], &a.chart),
        rings(second, faces[1], &b.chart),
    ) {
        (Ok(first), Ok(second)) => (first, second),
        (Err(error), _) | (_, Err(error)) if error.code != "BREP_UNSUPPORTED_OPERATION" => {
            return Err(error);
        }
        _ => return curved::strict_intersection([first, second], faces, [&a, &b], max_tests),
    };
    let regions = if first_rings.len() == 1 && second_rings.len() == 1 {
        match exact_convex_coplanar_overlap(&first_rings[0], &second_rings[0], max_tests) {
            Ok(ring) if ring.is_empty() => return Ok(Vec::new()),
            Ok(ring) => Some(vec![vec![ring]]),
            Err(error) if error.code == "BREP_UNSUPPORTED_OPERATION" => None,
            Err(error) => return Err(error),
        }
    } else {
        None
    };
    let regions = match regions {
        Some(regions) => regions,
        None => {
            let map = |rings: Vec<Vec<[f64; 3]>>, reverse: bool| -> Result<Vec<Vec<[Q; 2]>>> {
                rings
                    .into_iter()
                    .map(|ring| {
                        let mut uv = ring
                            .into_iter()
                            .map(|p| a.chart.inverse(&exact_point(p)?))
                            .collect::<Result<Vec<_>>>()?;
                        if reverse {
                            uv.reverse();
                        }
                        Ok(uv)
                    })
                    .collect()
            };
            crate::profile::profile_certificate::intersection_rings(
                map(first_rings, false)?,
                map(second_rings, opposite_charts)?,
                max_tests,
            )?
            .into_iter()
            .map(|region| {
                region
                    .into_iter()
                    .map(|ring| ring.iter().map(|p| a.chart.lift(p)).collect())
                    .collect()
            })
            .collect()
        }
    };
    regions
        .into_iter()
        .map(|region| {
            build_patch(
                &a.chart,
                &first.faces[faces[0]].surface,
                a.reversed,
                first.tolerance_mm.min(second.tolerance_mm),
                region,
            )
        })
        .collect()
}

/// Convenience adapter for callers expecting no more than one patch.
pub fn exact_coplanar_intersection_patch(
    solids: [&QualifiedClosedSolid; 2],
    faces: [usize; 2],
    max_tests: usize,
) -> Result<Option<Model>> {
    let mut patches = exact_coplanar_intersection_patches(solids, faces, max_tests)?;
    if patches.len() > 1 {
        return Err(unsupported(
            "Exact coplanar intersection has multiple patches",
        ));
    }
    Ok(patches.pop())
}

fn build_patch(
    chart: &AffineChart,
    source: &crate::Surface,
    reversed: bool,
    tolerance_mm: f64,
    region: Vec<Vec<Point>>,
) -> Result<Model> {
    let mut vertices = Vec::new();
    let mut uv = Vec::new();
    let mut edges = Vec::new();
    let mut loops = Vec::new();
    for ring in region {
        let start = vertices.len();
        for p in ring {
            vertices.push(Vertex {
                point: [
                    representable(&p[0])?,
                    representable(&p[1])?,
                    representable(&p[2])?,
                ],
            });
            let [u, v] = chart.inverse(&p)?;
            uv.push(vec![representable(&u)?, representable(&v)?]);
        }
        let mut coedges = Vec::new();
        for i in start..vertices.len() {
            let j = if i + 1 == vertices.len() {
                start
            } else {
                i + 1
            };
            coedges.push(Coedge::new(
                edges.len(),
                false,
                crate::line(uv[i].clone(), uv[j].clone()),
            ));
            edges.push(Edge {
                degenerate: false,
                vertices: [i, j],
                curve: crate::line(vertices[i].point.to_vec(), vertices[j].point.to_vec()),
            });
        }
        loops.push(Loop { coedges });
    }
    let mut patch = Model(
        brep_topology::Model {
            vertices,
            edges,
            faces: vec![Face {
                surface: chart.restrict(source, &uv)?,
                outer: 0,
                holes: (1..loops.len()).collect(),
            }],
            loops,
            shells: vec![Shell {
                faces: vec![FaceUse { face: 0, reversed }],
                closed: false,
            }],
            bodies: vec![],
            tolerance_mm,
        },
        TopologyIds::default(),
    );
    patch.rebuild_topology_ids();
    patch.validate()?;
    Ok(patch)
}
