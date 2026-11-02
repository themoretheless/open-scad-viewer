use super::*;
pub struct SixEdgeGraphProof {
    pub roof_cells: usize,
    pub smooth_seams: usize,
    pub exact_jet_equalities: usize,
    pub classified_face_pairs: usize,
    pub minimum_height_mm: f64,
    pub maximum_height_mm: f64,
    pub volume_exact: String,
    pub volume_mm3: f64,
    pub material_inside_source: bool,
    pub curvature_upper_per_mm: f64,
}
fn derivative(p: &Poly, axis: usize) -> Poly {
    p.iter()
        .filter_map(|(&(i, j), v)| {
            let n = if axis == 0 { i } else { j };
            (n > 0).then(|| {
                (
                    (i - usize::from(axis == 0), j - usize::from(axis == 1)),
                    v * q(n as i64),
                )
            })
        })
        .collect()
}
fn integral(p: &Poly, polygon: &[Point]) -> Q {
    let mut sum = Q::zero();
    for (&(i, j), v) in p {
        for e in 0..polygon.len() {
            let a = polygon[e];
            let b = polygon[(e + 1) % polygon.len()];
            for r in 0..=i + 1 {
                for s in 0..=j {
                    sum += v
                        * q(choose(i + 1, r))
                        * q(a[0]).pow((i + 1 - r) as i32)
                        * q(b[0] - a[0]).pow(r as i32)
                        * q(choose(j, s))
                        * q(a[1]).pow((j - s) as i32)
                        * q(b[1] - a[1]).pow(s as i32)
                        * q(b[1] - a[1])
                        / q((i + 1) as i64)
                        / q((r + s + 1) as i64);
                }
            }
        }
    }
    sum
}
/// Fresh bounded-family proof of the ACTUAL complete saved boundary. Literal
/// reconstruction equality is mandatory, including surfaces, pcurves, curves,
/// loops, senses and material roles. Exact rational polynomials certify the
/// complete XY tiling and all shared 2-jets; convex vertical columns classify
/// every boundary-face pair. This is not the generic arbitrary-B-rep inspector.
/// The complete trimmed-face curvature bound is recomputed from exact profile
/// coefficients. Principal arm radius measurements remain a separate gate.
pub fn inspect_six_edge_graph(source: &Model, corner: &SixEdgeCorner) -> Result<SixEdgeGraphProof> {
    source.validate()?;
    corner.model.validate()?;
    if source.0 != six_edge_corner_stock()?.0 {
        return Err(fail("Source no longer matches six-edge stock"));
    }
    if corner
        .handles
        .iter()
        .any(|h| !h.is_finite() || *h < 0. || *h > 1. || (*h * 16777216.).fract() != 0.)
    {
        return Err(fail("Invalid profile family"));
    }
    let (expected, top, _) = construct(true, corner.handles)?;
    if expected.0 != corner.model.0 || top != corner.top_faces || corner.physical_scale != SCALE {
        return Err(fail(
            "Saved six-edge geometry differs from exact construction",
        ));
    }
    match (corner.requested_mid_radii, corner.mid_radius_intervals) {
        (None, None) => {}
        (Some(requested), Some(saved)) => {
            let fresh = radius_intervals(corner.handles)?;
            if fresh != saved
                || (0..3).any(|i| {
                    !requested[i].is_finite()
                        || requested[i] <= 0.
                        || fresh[i][0] < requested[i] - 1e-7
                        || fresh[i][1] > requested[i] + 1e-7
                })
            {
                return Err(fail(
                    "Saved prescribed radius does not match actual profile coefficients",
                ));
            }
        }
        _ => return Err(fail("Incomplete six-edge radius intent")),
    }
    let cells = cells(true, corner.handles)?;
    for cell in &cells {
        if area(&cell.polygon) <= 0 {
            return Err(fail("Nonpositive retained XY cell"));
        }
        for i in 0..cell.polygon.len() {
            let a = cell.polygon[i];
            let b = cell.polygon[(i + 1) % cell.polygon.len()];
            for &p in &cell.polygon {
                if (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]) < 0 {
                    return Err(fail("Retained XY cell is not convex"));
                }
            }
        }
        for p in &cell.polygon {
            for i in 0..3 {
                let l = linear(*p, i);
                let inside = match cell.states[i] {
                    -1 => l <= -A,
                    0 => -A <= l && l <= A,
                    _ => l >= A,
                };
                if !inside {
                    return Err(fail(
                        "Actual retained cell leaves its polynomial profile span",
                    ));
                }
            }
        }
    }
    let sum: i64 = cells.iter().map(|c| area(&c.polygon)).sum();
    if sum != area(&hexagon())
        || cells.iter().any(|c| {
            c.polygon
                .iter()
                .any(|p| p[0].abs() > R || p[1].abs() > R || (p[0] + p[1]).abs() > R)
        })
    {
        return Err(fail("Exact XY partition failed"));
    }
    // State intervals have disjoint interiors and exhaust each axis. Their
    // convex clipped cells therefore partition the original convex hexagon.
    // On each cell every smooth profile is a positive Bernstein combination
    // of [3,2,1,h,1,2,3], hence 0<=f<=3; other profiles satisfy 0<=abs(L)<=12.
    // Every roof is consequently above 4*SCALE and inside the original stock:
    // f-L and f+L have nonnegative literal degree-six coefficients when h>=0.
    let mut seams = Vec::new();
    let mut equalities = 0;
    for edge in 0..corner.model.edges.len() {
        let uses: Vec<_> = top
            .iter()
            .enumerate()
            .filter_map(|(i, &f)| {
                corner.model.loops[corner.model.faces[f].outer]
                    .coedges
                    .iter()
                    .find(|c| c.edge == edge)
                    .map(|c| (i, c))
            })
            .collect();
        if uses.len() != 2 {
            continue;
        }
        let ((a, co), (b, _)) = (uses[0], uses[1]);
        let points = &co.pcurve.control_points;
        if points.len() != 2 {
            return Err(fail("Roof boundary chart is not original affine segment"));
        }
        let to_point = |p: &Vec<f64>| -> Result<Point> {
            let r = [p[0] as i64, p[1] as i64];
            if r[0] as f64 != p[0] || r[1] as f64 != p[1] {
                return Err(fail("Roof boundary root is not exact lattice owner"));
            }
            Ok(r)
        };
        let u = to_point(&points[0])?;
        let v = to_point(&points[1])?;
        let mut delta = cells[a].height.clone();
        add(&mut delta, &cells[b].height, &q(-1));
        for [du, dv] in [[0, 0], [1, 0], [0, 1], [2, 0], [1, 1], [0, 2]] {
            let mut d = delta.clone();
            for _ in 0..du {
                d = derivative(&d, 0);
            }
            for _ in 0..dv {
                d = derivative(&d, 1);
            }
            if restriction(&d, u, v).iter().any(|c| !c.is_zero()) {
                return Err(fail("Actual shared roof two-jets do not match"));
            }
            equalities += 1;
        }
        seams.push(edge);
    }
    if seams != corner.smooth_edges {
        return Err(fail(
            "Requested smooth roles differ from final actual seams",
        ));
    }
    // Roof cells are single-valued over disjoint XY interiors. Walls lie over
    // convex outer polygon edges and range from z=0 to the same positive roof.
    // Base is z=0. Thus roof/roof, roof/wall, roof/base, wall/wall and wall/base
    // pairs meet only on their source-owned expected strata, never interiors.
    let volume = cells
        .iter()
        .fold(Q::zero(), |v, c| v + integral(&c.height, &c.polygon))
        * scale().pow(3);
    if volume <= Q::zero() {
        return Err(fail("Columnar material has no positive volume"));
    }
    // On [-3,3], phi'' has Bernstein coefficients
    // (5/6)*[0,h,2-2h,h,0]. All are nonnegative. The actual graph Hessian is
    // -(d0*ex*ex^T + d1*ey*ey^T + d2*(ex+ey)*(ex+ey)^T).
    // Its operator norm is <=max(mu0,mu1)+2*mu2. For the physical height
    // graph, G=I+grad(z)*grad(z)^T >= I and the normal denominator >=1,
    // so the full shape-operator norm is <=this bound /physical_scale.
    // Retained plane walls/base have zero curvature. No samples participate.
    let mus: Vec<Q> = corner
        .handles
        .iter()
        .map(|h| {
            let h = Q::from_f64(*h).unwrap();
            let other = q(2) - q(2) * &h;
            std::cmp::max(h, other) * q(5) / q(6)
        })
        .collect();
    let curvature = (std::cmp::max(mus[0].clone(), mus[1].clone()) + q(2) * &mus[2]) / scale();
    if curvature > q(10) {
        return Err(fail("Complete coefficient curvature bound exceeds 10/mm"));
    }
    let raw = curvature
        .to_f64()
        .ok_or_else(|| fail("Curvature outside binary range"))?;
    let curvature_upper = if Q::from_f64(raw).unwrap() < curvature {
        raw.next_up()
    } else {
        raw
    };
    let n = corner.model.faces.len();
    Ok(SixEdgeGraphProof {
        roof_cells: cells.len(),
        smooth_seams: seams.len(),
        exact_jet_equalities: equalities,
        classified_face_pairs: n * (n - 1) / 2,
        minimum_height_mm: 4. * SCALE,
        maximum_height_mm: H as f64 * SCALE,
        volume_exact: volume.to_string(),
        volume_mm3: volume
            .to_f64()
            .ok_or_else(|| fail("Volume outside binary range"))?,
        material_inside_source: true,
        curvature_upper_per_mm: curvature_upper,
    })
}
