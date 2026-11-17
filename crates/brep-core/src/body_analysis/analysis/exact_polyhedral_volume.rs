use crate::{Error, Result, closed_solid::QualifiedClosedSolid};
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};
use nurbs_core::analysis::exact_affine_hull::{SurfaceAffineHull, classify_surface_affine_hull};

#[derive(Clone, Debug)]
pub struct ExactPolyhedralVolume {
    /// Total material volume, subtracting inward cavity boundaries.
    pub volume_mm3: Q,
    pub exact_work: u64,
}

/// Integrate a qualified solid with planar faces and full piecewise-linear
/// edges. Exact oriented boundary flux counts holes and concavities without
/// triangulating the retained regions or integrating their unused supports.
pub fn exact_polyhedral_volume(
    solid: &QualifiedClosedSolid,
    max_exact_work: u64,
) -> Result<ExactPolyhedralVolume> {
    let (six_volume, exact_work) =
        integrate_polyhedral_boundary(solid, max_exact_work, |_, _, _, _, _| Ok(()))?;
    Ok(ExactPolyhedralVolume {
        volume_mm3: six_volume / Q::from_integer(6.into()),
        exact_work,
    })
}

#[derive(Clone, Debug)]
pub struct ExactPolyhedralCentroid {
    pub volume_mm3: Q,
    /// Center of volume in world coordinates, for uniform density.
    pub centroid_mm: [Q; 3],
    pub exact_work: u64,
}

/// Integrate first moments from the oriented polygonal boundary of a
/// qualified planar solid. Holes and inward cavity shells subtract material.
pub fn exact_polyhedral_centroid(
    solid: &QualifiedClosedSolid,
    max_exact_work: u64,
) -> Result<ExactPolyhedralCentroid> {
    let mut moments = std::array::from_fn(|_| Q::zero());
    let (six_volume, exact_work) =
        integrate_polyhedral_boundary(solid, max_exact_work, |a, b, c, signed, remaining| {
            spend(remaining, 12)?;
            for i in 0..3 {
                moments[i] += signed * (&a[i] + &b[i] + &c[i]);
            }
            Ok(())
        })?;
    // The fourth tetrahedron vertex is the world origin.
    let denominator = &six_volume * Q::from_integer(4.into());
    Ok(ExactPolyhedralCentroid {
        volume_mm3: six_volume / Q::from_integer(6.into()),
        centroid_mm: moments.map(|moment| moment / &denominator),
        exact_work,
    })
}

#[derive(Clone, Debug)]
pub struct ExactPolyhedralInertia {
    pub volume_mm3: Q,
    pub centroid_mm: [Q; 3],
    /// Inertia about the centroid, in world axes at unit volumetric density.
    pub inertia_mm5: [[Q; 3]; 3],
    pub exact_work: u64,
}

/// Integrate the exact centroidal inertia tensor of a qualified planar solid
/// with full piecewise-linear edges, including signed contributions from voids.
pub fn exact_polyhedral_inertia(
    solid: &QualifiedClosedSolid,
    max_exact_work: u64,
) -> Result<ExactPolyhedralInertia> {
    let mut first: [Q; 3] = std::array::from_fn(|_| Q::zero());
    let mut second: [[Q; 3]; 3] = std::array::from_fn(|_| std::array::from_fn(|_| Q::zero()));
    let (six_volume, integration_work) =
        integrate_polyhedral_boundary(solid, max_exact_work, |a, b, c, signed, remaining| {
            spend(remaining, 72)?;
            let sum: [Q; 3] = std::array::from_fn(|i| &a[i] + &b[i] + &c[i]);
            for i in 0..3 {
                first[i] += signed * &sum[i];
                for j in i..3 {
                    let paired = &a[i] * &a[j] + &b[i] * &b[j] + &c[i] * &c[j];
                    second[i][j] += signed * (&sum[i] * &sum[j] + paired);
                }
            }
            Ok(())
        })?;
    let mut remaining = max_exact_work - integration_work;
    spend(&mut remaining, 48)?;
    let exact_work = max_exact_work - remaining;

    let volume_mm3 = &six_volume / Q::from_integer(6.into());
    let first_denominator = six_volume * Q::from_integer(4.into());
    let centroid_mm = first.map(|moment| moment / &first_denominator);
    // The signed determinants are six times the tetrahedron volumes.
    let second_denominator = Q::from_integer(120.into());
    let centered: [[Q; 3]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            &second[i.min(j)][i.max(j)] / &second_denominator
                - &volume_mm3 * &centroid_mm[i] * &centroid_mm[j]
        })
    });
    let trace = &centered[0][0] + &centered[1][1] + &centered[2][2];
    let inertia_mm5 = std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            if i == j {
                &trace - &centered[i][i]
            } else {
                -&centered[i][j]
            }
        })
    });
    Ok(ExactPolyhedralInertia {
        volume_mm3,
        centroid_mm,
        inertia_mm5,
        exact_work,
    })
}

fn integrate_polyhedral_boundary(
    solid: &QualifiedClosedSolid,
    max_exact_work: u64,
    mut integrate: impl FnMut(&[Q; 3], &[Q; 3], &[Q; 3], &Q, &mut u64) -> Result<()>,
) -> Result<(Q, u64)> {
    let mut six_volume = Q::zero();
    let mut remaining = max_exact_work;
    visit_polyhedral_boundary(solid, &mut remaining, |_, a, b, c, reversed, remaining| {
        spend(remaining, 21)?;
        let contribution = determinant(a, b, c);
        let signed = if reversed {
            -contribution
        } else {
            contribution
        };
        integrate(a, b, c, &signed, remaining)?;
        six_volume += signed;
        Ok(())
    })?;
    if !six_volume.is_positive() {
        return Err(refuse("Exact material volume must be positive"));
    }
    Ok((six_volume, max_exact_work - remaining))
}

#[derive(Clone, Debug)]
pub struct ExactPolyhedralArea {
    /// Indexed by B-rep face. Each value is the square of that face's
    /// retained area. The total area is the sum of the positive square roots.
    pub face_area_squared_mm4: Vec<Q>,
    pub exact_work: u64,
}

/// Measure retained planar faces with linear boundaries, including hole
/// subtraction. Irrational areas retain an exact square-root representation.
pub fn exact_polyhedral_area(
    solid: &QualifiedClosedSolid,
    max_exact_work: u64,
) -> Result<ExactPolyhedralArea> {
    let mut remaining = max_exact_work;
    let mut vectors =
        vec![std::array::from_fn::<Q, 3, _>(|_| Q::zero()); solid.model().faces.len()];
    visit_polyhedral_boundary(
        solid,
        &mut remaining,
        |face, a, b, c, reversed, remaining| {
            spend(remaining, 18)?;
            let u: [Q; 3] = std::array::from_fn(|i| &b[i] - &a[i]);
            let v: [Q; 3] = std::array::from_fn(|i| &c[i] - &a[i]);
            for (i, j, k) in [(0, 1, 2), (1, 2, 0), (2, 0, 1)] {
                let cross = &u[j] * &v[k] - &u[k] * &v[j];
                if reversed {
                    vectors[face][i] -= cross;
                } else {
                    vectors[face][i] += cross;
                }
            }
            Ok(())
        },
    )?;
    let four = Q::from_integer(4.into());
    let mut face_area_squared_mm4 = Vec::with_capacity(vectors.len());
    for vector in vectors {
        spend(&mut remaining, 7)?;
        let squared: Q = vector.iter().map(|x| x * x).sum();
        if squared.is_zero() {
            return Err(refuse(
                "Exact polyhedral area requires a positive area for each face",
            ));
        }
        face_area_squared_mm4.push(squared / &four);
    }
    Ok(ExactPolyhedralArea {
        face_area_squared_mm4,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug)]
pub struct BoundedPolyhedralArea {
    /// Lower and upper rational bounds for the total retained surface area.
    pub bounds_mm2: [Q; 2],
    pub exact_work: u64,
}

/// Bound the sum of exact planar face areas within a requested absolute
/// width. Perfect rational squares contribute exactly to both endpoints.
pub fn bounded_polyhedral_area(
    solid: &QualifiedClosedSolid,
    max_width_mm2: f64,
    max_exact_work: u64,
) -> Result<BoundedPolyhedralArea> {
    let tolerance = enclosure_width(max_width_mm2)?;
    let report = exact_polyhedral_area(solid, max_exact_work)?;
    let mut remaining = max_exact_work - report.exact_work;
    let bounds_mm2 = bound_root_sum(report.face_area_squared_mm4, tolerance, &mut remaining)?;
    Ok(BoundedPolyhedralArea {
        bounds_mm2,
        exact_work: max_exact_work - remaining,
    })
}

#[derive(Clone, Debug)]
pub struct BoundedPolyhedralEdgeLength {
    /// Lower and upper bounds for the length of the unique material edges.
    pub bounds_mm: [Q; 2],
    pub edge_count: usize,
    pub exact_work: u64,
}

/// Enclose the total length of full clamped piecewise-linear material edges.
/// Each topological edge contributes once, regardless of its coedge count.
pub fn bounded_polyhedral_edge_length(
    solid: &QualifiedClosedSolid,
    max_width_mm: f64,
    max_exact_work: u64,
) -> Result<BoundedPolyhedralEdgeLength> {
    let tolerance = enclosure_width(max_width_mm)?;
    validate_work(max_exact_work)?;
    let agreement = &solid.geometry().boundary.agreement;
    if !agreement.all_equal || !agreement.all_joins_exact {
        return Err(refuse(
            "Exact edge length requires exact boundary identity and joins",
        ));
    }
    let model = solid.model();
    let mut remaining = max_exact_work;
    let mut visited = std::collections::BTreeSet::new();
    let mut squares = Vec::new();
    for body in &model.bodies {
        for shell in std::iter::once(&body.outer_shell).chain(&body.inner_shells) {
            for face_use in &model.shells[*shell].faces {
                let face = &model.faces[face_use.face];
                for wire in std::iter::once(&face.outer).chain(&face.holes) {
                    for coedge in &model.loops[*wire].coedges {
                        spend(&mut remaining, 1)?;
                        let edge = &model.edges[coedge.edge];
                        require_linear_edge(edge, coedge)?;
                        if !visited.insert(coedge.edge) {
                            continue;
                        }
                        let curve = &edge.curve;
                        spend(&mut remaining, 3)?;
                        let mut previous = lift(&curve.control_points[0]);
                        for (i, pole) in curve.control_points.iter().enumerate().skip(1) {
                            spend(&mut remaining, 12)?;
                            require_linear_span(&curve.knots, i)?;
                            let current = lift(pole);
                            squares.push(
                                previous
                                    .iter()
                                    .zip(&current)
                                    .map(|(a, b)| (a - b).pow(2))
                                    .sum(),
                            );
                            previous = current;
                        }
                    }
                }
            }
        }
    }
    let bounds_mm = bound_root_sum(squares, tolerance, &mut remaining)?;
    Ok(BoundedPolyhedralEdgeLength {
        bounds_mm,
        edge_count: visited.len(),
        exact_work: max_exact_work - remaining,
    })
}

fn enclosure_width(width: f64) -> Result<Q> {
    Q::from_float(width)
        .filter(|value| value.is_positive())
        .ok_or_else(|| {
            Error::new(
                "BREP_INVALID_INPUT",
                "Enclosure width must be finite and positive",
            )
        })
}

fn bound_root_sum(squares: Vec<Q>, tolerance: Q, remaining: &mut u64) -> Result<[Q; 2]> {
    if squares.is_empty() {
        return Ok([Q::zero(), Q::zero()]);
    }
    spend(remaining, 8)?;
    // A common grid gives each irrational term an interval no wider than
    // tolerance / face count, so their widths sum to at most tolerance.
    let count = Q::from_integer(squares.len().into());
    let denominator = (count / tolerance).ceil().to_integer();
    let denominator_squared = &denominator * &denominator;
    let mut lower = Q::zero();
    let mut upper = Q::zero();
    for square in squares {
        spend(remaining, 16)?;
        let numerator_root = square.numer().sqrt();
        let denominator_root = square.denom().sqrt();
        if &numerator_root * &numerator_root == *square.numer()
            && &denominator_root * &denominator_root == *square.denom()
        {
            let area = Q::new(numerator_root, denominator_root);
            lower += &area;
            upper += area;
        } else {
            let scaled_floor = (square.numer() * &denominator_squared) / square.denom();
            let grid_floor = scaled_floor.sqrt();
            lower += Q::new(grid_floor.clone(), denominator.clone());
            upper += Q::new(grid_floor + 1, denominator.clone());
        }
    }
    Ok([lower, upper])
}

#[derive(Clone, Debug)]
pub struct ExactPolyhedralProjection {
    /// Minimum and maximum of the unnormalized dot product with the direction.
    pub dot_bounds: [Q; 2],
    /// Points on the material boundary attaining the corresponding extrema.
    pub witnesses_mm: [[Q; 3]; 2],
    pub exact_work: u64,
}

/// Find the exact directional range of a qualified planar solid with full
/// piecewise-linear edges. A linear functional on each retained planar
/// region attains its extrema on the polygonal boundary.
pub fn exact_polyhedral_projection(
    solid: &QualifiedClosedSolid,
    direction: [f64; 3],
    max_exact_work: u64,
) -> Result<ExactPolyhedralProjection> {
    if !direction.iter().all(|value| value.is_finite()) || direction == [0.; 3] {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Projection needs a finite nonzero direction",
        ));
    }
    let ([axis], exact_work) = project_polyhedral_boundary(
        solid,
        [lift(&direction)],
        std::array::from_fn(|_| Q::zero()),
        max_exact_work,
    )?;
    Ok(ExactPolyhedralProjection {
        dot_bounds: axis.bounds,
        witnesses_mm: axis.witnesses_mm,
        exact_work,
    })
}

#[derive(Clone, Debug)]
pub struct ExactPolyhedralFrameBounds {
    /// Min/max coordinates along each basis axis in the supplied frame.
    pub bounds: [[Q; 2]; 3],
    /// World-space points attaining each axis's two extrema.
    pub witnesses_mm: [[[Q; 3]; 2]; 3],
    pub exact_work: u64,
}

/// Get the smallest coordinate box in a caller-supplied affine frame.
/// The three world-space basis vectors may be skewed or scaled.
pub fn exact_polyhedral_frame_bounds(
    solid: &QualifiedClosedSolid,
    origin_mm: [f64; 3],
    basis: [[f64; 3]; 3],
    max_exact_work: u64,
) -> Result<ExactPolyhedralFrameBounds> {
    validate_work(max_exact_work)?;
    if !origin_mm
        .iter()
        .chain(basis.iter().flatten())
        .all(|value| value.is_finite())
    {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Projection frame must be finite",
        ));
    }
    let mut frame_work = max_exact_work;
    spend(&mut frame_work, 64)?;
    let [a, b, c] = basis.map(|axis| lift(&axis));
    let determinant = determinant(&a, &b, &c);
    if determinant.is_zero() {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Projection frame must be invertible",
        ));
    }
    let covectors = [(&b, &c), (&c, &a), (&a, &b)].map(|(u, v)| {
        [(1, 2), (2, 0), (0, 1)].map(|(j, k)| (&u[j] * &v[k] - &u[k] * &v[j]) / &determinant)
    });
    let (axes, work) = project_polyhedral_boundary(solid, covectors, lift(&origin_mm), frame_work)?;
    Ok(ExactPolyhedralFrameBounds {
        bounds: axes.each_ref().map(|axis| axis.bounds.clone()),
        witnesses_mm: axes.map(|axis| axis.witnesses_mm),
        exact_work: max_exact_work - frame_work + work,
    })
}

struct ProjectionAxisBounds {
    bounds: [Q; 2],
    witnesses_mm: [[Q; 3]; 2],
}

fn project_polyhedral_boundary<const N: usize>(
    solid: &QualifiedClosedSolid,
    directions: [[Q; 3]; N],
    origin: [Q; 3],
    max_exact_work: u64,
) -> Result<([ProjectionAxisBounds; N], u64)> {
    let mut ranges: [Option<ProjectionAxisBounds>; N] = std::array::from_fn(|_| None);
    let mut remaining = max_exact_work;
    visit_polyhedral_boundary(solid, &mut remaining, |_, _, a, b, _, remaining| {
        for point in [a, b] {
            spend(remaining, 3 + 7 * N as u64)?;
            let local: [Q; 3] = std::array::from_fn(|i| &point[i] - &origin[i]);
            for (direction, range) in directions.iter().zip(&mut ranges) {
                let value: Q = direction.iter().zip(&local).map(|(d, p)| d * p).sum();
                if let Some(range) = range {
                    if value < range.bounds[0] {
                        range.bounds[0] = value;
                        range.witnesses_mm[0] = point.clone();
                    } else if value > range.bounds[1] {
                        range.bounds[1] = value;
                        range.witnesses_mm[1] = point.clone();
                    }
                } else {
                    *range = Some(ProjectionAxisBounds {
                        bounds: [value.clone(), value],
                        witnesses_mm: [point.clone(), point.clone()],
                    });
                }
            }
        }
        Ok(())
    })?;
    if ranges.iter().any(Option::is_none) {
        return Err(refuse("Exact projection requires material boundary edges"));
    }
    Ok((ranges.map(Option::unwrap), max_exact_work - remaining))
}

fn visit_polyhedral_boundary(
    solid: &QualifiedClosedSolid,
    remaining: &mut u64,
    mut visit: impl FnMut(usize, &[Q; 3], &[Q; 3], &[Q; 3], bool, &mut u64) -> Result<()>,
) -> Result<()> {
    validate_work(*remaining)?;
    let agreement = &solid.geometry().boundary.agreement;
    if !agreement.all_equal || !agreement.all_joins_exact {
        return Err(refuse(
            "Exact polyhedral integration requires exact boundary identity and joins",
        ));
    }
    let model = solid.model();
    for body in &model.bodies {
        for shell in std::iter::once(&body.outer_shell).chain(&body.inner_shells) {
            for face_use in &model.shells[*shell].faces {
                let face = &model.faces[face_use.face];
                spend(remaining, 1)?;
                if *remaining == 0 {
                    return Err(work_exhausted());
                }
                let affine = classify_surface_affine_hull(&face.surface, *remaining)?;
                *remaining -= affine.work_used;
                match affine.hull {
                    SurfaceAffineHull::Plane(_) => {}
                    SurfaceAffineHull::ResourceLimit => return Err(work_exhausted()),
                    _ => {
                        return Err(refuse(
                            "Exact polyhedral integration requires planar supports",
                        ));
                    }
                }

                let outer = &model.loops[face.outer];
                let first = &outer.coedges[0];
                let origin = lift(&model.edges[first.edge].curve.control_points[0]);
                for wire in std::iter::once(&face.outer).chain(&face.holes) {
                    for coedge in &model.loops[*wire].coedges {
                        let edge = &model.edges[coedge.edge];
                        let curve = &edge.curve;
                        let knots = &curve.knots;
                        require_linear_edge(edge, coedge)?;
                        let reversed = coedge.reversed ^ face_use.reversed;
                        spend(remaining, 3)?;
                        let mut previous = lift(&curve.control_points[0]);
                        for (i, pole) in curve.control_points.iter().enumerate().skip(1) {
                            spend(remaining, 3)?;
                            require_linear_span(knots, i)?;
                            let current = lift(pole);
                            visit(
                                face_use.face,
                                &origin,
                                &previous,
                                &current,
                                reversed,
                                remaining,
                            )?;
                            previous = current;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_work(work: u64) -> Result<()> {
    if !(1..=1_000_000).contains(&work) {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Exact polyhedral work must be in 1..1000000",
        ));
    }
    Ok(())
}

fn require_linear_edge(edge: &crate::Edge, coedge: &crate::Coedge) -> Result<()> {
    let curve = &edge.curve;
    let knots = &curve.knots;
    let n = curve.control_points.len();
    if edge.degenerate
        || coedge.trim().is_some()
        || curve.degree != 1
        || curve.periodic
        || knots[0] != knots[1]
        || knots[n] != knots[n + 1]
    {
        return Err(refuse(
            "Exact polyhedral integration requires full clamped degree-one edges",
        ));
    }

    Ok(())
}

fn require_linear_span(knots: &[f64], i: usize) -> Result<()> {
    if knots[i] >= knots[i + 1] {
        return Err(refuse(
            "Exact polyhedral integration requires nonzero linear spans",
        ));
    }
    Ok(())
}

fn lift(p: &[f64]) -> [Q; 3] {
    std::array::from_fn(|i| Q::from_float(p[i]).unwrap())
}

fn determinant(a: &[Q; 3], b: &[Q; 3], c: &[Q; 3]) -> Q {
    [(0, 1, 2), (1, 2, 0), (2, 0, 1)]
        .into_iter()
        .map(|(i, j, k)| &a[i] * (&b[j] * &c[k] - &b[k] * &c[j]))
        .sum()
}

fn spend(remaining: &mut u64, cost: u64) -> Result<()> {
    *remaining = remaining.checked_sub(cost).ok_or_else(work_exhausted)?;
    Ok(())
}

fn work_exhausted() -> Error {
    Error::new(
        "BREP_RESOURCE_LIMIT",
        "Exact material volume work exhausted",
    )
}

fn refuse(message: &str) -> Error {
    Error::new("BREP_CERTIFIED_MASS_REFUSED", message)
}
