use super::*;
use crate::closed_solid::{
    BooleanFace, BooleanFaceBoxLimits, BooleanFaceDisposition, BooleanFaceSeedLimits,
    BooleanSourceSide, qualify_boolean_face_seed,
};
use crate::face_domain::{FaceDomain, affine_support};
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};

impl QualifiedOpenSSNetwork<'_, '_, '_, '_, '_> {
    /// Classify all descendants on the selected source face, with a fresh
    /// certified interior witness for each region. Refuses the whole selection
    /// if any region cannot be proved.
    pub fn discover_boolean_descendants(
        &self,
        query: BooleanFace<'_>,
        directions: &[[f64; 3]],
        limits: BooleanFaceSeedLimits,
    ) -> Result<Vec<(usize, BooleanFaceDisposition)>> {
        self.faces[usize::from(query.side == BooleanSourceSide::Second)]
            .iter()
            .map(|&descendant| {
                let (model, tolerance) = self.boolean_descendant(query, descendant)?;
                let disposition =
                    prove_face_material(query, model, descendant, tolerance, directions, limits)?;
                Ok((descendant, disposition))
            })
            .collect()
    }

    /// Select the whole relative interior of an affine descendant when
    /// every opposing face is proved disjoint from that interior.
    pub fn qualify_boolean_descendant(
        &self,
        query: BooleanFace<'_>,
        descendant: usize,
        seed: [[f64; 2]; 2],
        directions: &[[f64; 3]],
        limits: BooleanFaceBoxLimits,
    ) -> Result<BooleanFaceDisposition> {
        let (model, _) = self.boolean_descendant(query, descendant)?;
        let other = query.solids[usize::from(query.side == BooleanSourceSide::First)].model();
        separate_interior(
            model,
            descendant,
            other,
            limits.material.source.distance_cells,
        )?;
        let seed = self.qualify_boolean_rectangle(query, descendant, seed, directions, limits)?;
        Ok(query
            .operation
            .face_disposition(query.side, seed.material_box().inside()))
    }
}

pub(super) fn prove_face_material(
    query: BooleanFace<'_>,
    model: &Model,
    face: usize,
    tolerance: f64,
    directions: &[[f64; 3]],
    limits: BooleanFaceSeedLimits,
) -> Result<BooleanFaceDisposition> {
    let other = query.solids[usize::from(query.side == BooleanSourceSide::First)].model();
    separate_interior(model, face, other, limits.material.distance_cells)?;
    let domain = FaceDomain::new(model, face, tolerance)?;
    let native = crate::boolean_robustness::surface_uv_domains(&model.faces[face].surface);
    let (uv, remaining) = interior_witness(&domain, native, limits.domain_cells)?;
    let witness = qualify_boolean_face_seed(
        query,
        uv,
        directions,
        BooleanFaceSeedLimits {
            domain_cells: remaining.min(100_000),
            ..limits
        },
    )?;
    Ok(query
        .operation
        .face_disposition(query.side, witness.material_seed().inside()))
}

fn interior_witness(
    domain: &FaceDomain,
    native: [[f64; 2]; 2],
    limit: usize,
) -> Result<([f64; 2], usize)> {
    if !(1..=1_000_000).contains(&limit) {
        return Err(invalid("Need bounded SS interior witness work"));
    }
    let mut pending = std::collections::VecDeque::from([(native, 0)]);
    let mut used = 0;
    while let Some((mut cell, axis)) = pending.pop_front() {
        spend(&mut used, limit, 1)?;
        if used == limit {
            break;
        }
        let uv = cell.map(|[lo, hi]| lo.midpoint(hi));
        let proof = domain.classify(uv.map(|x| [x; 2]), (limit - used).min(100_000))?;
        spend(&mut used, limit, proof.cells)?;
        if proof.location == Location::Inside && used < limit {
            return Ok((uv, limit - used));
        }
        if cell[axis][0] < uv[axis] && uv[axis] < cell[axis][1] {
            let mut right = cell;
            cell[axis][1] = uv[axis];
            right[axis][0] = uv[axis];
            pending.extend([(cell, 1 - axis), (right, 1 - axis)]);
        }
    }
    Err(unproven("No interior SS descendant witness was proved"))
}

fn exact(point: &[f64], axis: usize) -> Q {
    Q::from_float(point[axis]).expect("validated finite BRep coordinate")
}

type Point = [Q; 3];

fn difference(a: &Point, b: &Point) -> Point {
    std::array::from_fn(|k| &a[k] - &b[k])
}

fn cross(a: &Point, b: &Point) -> Point {
    std::array::from_fn(|k| {
        let i = (k + 1) % 3;
        let j = (k + 2) % 3;
        &a[i] * &b[j] - &a[j] * &b[i]
    })
}

fn signed_distance(point: &Point, origin: &Point, normal: &Point) -> Q {
    (0..3).map(|k| (&point[k] - &origin[k]) * &normal[k]).sum()
}

fn hull_ranges(points: &[Point]) -> [[&Q; 2]; 3] {
    std::array::from_fn(|k| {
        let lo = points.iter().map(|p| &p[k]).min().unwrap();
        let hi = points.iter().map(|p| &p[k]).max().unwrap();
        [lo, hi]
    })
}

fn opposing_plane(points: &[Point]) -> Option<(&Point, Point)> {
    let origin = &points[0];
    let u = difference(points.iter().find(|p| *p != origin)?, origin);
    let normal = points
        .iter()
        .map(|p| cross(&u, &difference(p, origin)))
        .find(|normal| normal.iter().any(|value| !value.is_zero()))?;
    points
        .iter()
        .all(|p| signed_distance(p, origin, &normal).is_zero())
        .then_some((origin, normal))
}

fn separate_interior(
    model: &Model,
    descendant: usize,
    other: &Model,
    max_work: usize,
) -> Result<()> {
    let face = &model.faces[descendant];
    let original = &face.surface;
    if !(1..=1_000_000).contains(&max_work) {
        return Err(invalid("Need bounded SS descendant separation work"));
    }
    let affine;
    let surface = if !original.periodic_u && !original.periodic_v && affine_support(original) {
        original
    } else {
        affine = nurbs_core::surface_injectivity::certify_affine_parameterization(original)
            .ok_or_else(|| unproven("Whole SS descendant needs a proved affine chart"))?;
        &affine
    };
    let origin: [Q; 3] = std::array::from_fn(|k| exact(&surface.control_points[0][0], k));
    let u: [Q; 3] = std::array::from_fn(|k| exact(&surface.control_points[1][0], k) - &origin[k]);
    let v: [Q; 3] = std::array::from_fn(|k| exact(&surface.control_points[0][1], k) - &origin[k]);
    let knots = [&surface.knots_u, &surface.knots_v];
    let starts = knots.map(|k| exact(k, 1));
    let spans: [Q; 2] = std::array::from_fn(|k| exact(knots[k], 2) - &starts[k]);
    let mut hull: Vec<[Q; 3]> = Vec::new();
    let mut used = 0;
    for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
        for coedge in &model.loops[wire].coedges {
            let curve = &coedge.pcurve;
            if curve.weights.iter().any(|&w| w <= 0.) {
                return Err(unproven("SS descendant trim lacks positive weights"));
            }
            spend(&mut used, max_work, curve.control_points.len())?;
            for point in &curve.control_points {
                let s = (exact(point, 0) - &starts[0]) / &spans[0];
                let t = (exact(point, 1) - &starts[1]) / &spans[1];
                hull.push(std::array::from_fn(|k| {
                    &origin[k] + &u[k] * &s + &v[k] * &t
                }));
            }
        }
    }
    let ranges = hull_ranges(&hull);
    for face in &other.faces {
        let surface = &face.surface;
        if surface.weights.iter().flatten().any(|&weight| weight <= 0.) {
            return Err(unproven(
                "Opposing face lacks a positive rational control hull",
            ));
        }
        let mut points = Vec::new();
        for point in surface.control_points.iter().flatten() {
            spend(&mut used, max_work, 1)?;
            points.push(std::array::from_fn(|k| exact(point, k)));
        }
        let opposing = hull_ranges(&points);
        // At a shared range endpoint, only a nonconstant source coordinate
        // proves strict separation throughout the open descendant.
        if ranges
            .into_iter()
            .zip(opposing)
            .any(|([lo, hi], [a, b])| hi < a || b < lo || (lo < hi && (hi == a || b == lo)))
        {
            continue;
        }
        spend(&mut used, max_work, points.len())?;
        let (origin, normal) = opposing_plane(&points)
            .ok_or_else(|| unproven("Curved opposing face may reach the SS descendant"))?;
        spend(&mut used, max_work, hull.len())?;
        let mut positive = false;
        let mut negative = false;
        for point in &hull {
            let signed = signed_distance(point, origin, &normal);
            positive |= signed.is_positive();
            negative |= signed.is_negative();
        }
        // A halfspace containing all mapped trim poles contains the filled UV
        // region. A nonconstant affine sign is strict on its open interior.
        if positive == negative {
            return Err(unproven(
                "Opposing plane may meet the interior of the SS descendant",
            ));
        }
    }
    Ok(())
}
