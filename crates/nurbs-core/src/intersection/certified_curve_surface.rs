//! Original-coefficient curve/surface roots with strict interval Krawczyk proofs.
//! The generic Krawczyk path leaves original-boundary/tangent roots unresolved;
//! artificial split seams use overlapping original-source window proofs. Specialized
//! original plane and quadric graph owners below prove supported families.
//! Monotone affine-plane proofs can cross original interior curve knots.
use crate::{
    check, curve::Curve, distance_bounds::Interval as I, resource, surface::Surface, Result,
};
type Vector = [I; 3];
type Box3 = [[f64; 2]; 3];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootProof {
    InteriorKrawczyk,
    MonotoneAffinePlane,
}
#[derive(Debug)]
pub struct Root {
    pub proof: RootProof,
    pub spans: [usize; 3],
    /// Outward normalized reference bounds. For Krawczyk roots, use the
    /// explicit natural existence_domain as the uniqueness authority.
    /// Krawczyk cells are dyadic subdivisions; affine-plane enclosures can
    /// touch an interior C1 knot and use the closed adjacent span.
    pub normalized_cell: Box3,
    /// Exact binary64 Krawczyk existence/uniqueness domain, potentially
    /// crossing artificial subdivision seams but never original knot spans.
    /// None for the separately proved monotone affine-plane family.
    pub existence_domain: Option<Box3>,
    /// Outward natural t/u/v enclosure of that unique root.
    pub parameter_box: Box3,
    pub position_bounds: [[f64; 2]; 3],
    /// Sign of C_t dot (S_u cross S_v) at this certified transverse root.
    pub crossing_sign: i8,
}
#[derive(Debug)]
pub struct Unresolved {
    pub spans: [usize; 3],
    pub normalized_cell: Box3,
    pub parameter_box: Box3,
}
#[derive(Debug)]
pub struct Report {
    pub roots: Vec<Root>,
    pub unresolved: Vec<Unresolved>,
    pub cells: usize,
    pub excluded: usize,
    /// Complete original parameter-product coverage, not a solid/trim proof.
    pub complete: bool,
}
#[derive(Clone)]
struct Net {
    dims: [usize; 3],
    data: Vec<Vector>,
}
impl Net {
    fn index(&self, p: [usize; 3]) -> usize {
        (p[0] * self.dims[1] + p[1]) * self.dims[2] + p[2]
    }
    fn point(&self, p: [usize; 3]) -> Vector {
        self.data[self.index(p)]
    }
    fn coordinates(&self) -> impl Iterator<Item = [usize; 3]> + '_ {
        (0..self.dims[0]).flat_map(move |i| {
            (0..self.dims[1]).flat_map(move |j| (0..self.dims[2]).map(move |k| [i, j, k]))
        })
    }
    fn center(&self) -> Result<Vector> {
        let mut current = self.clone();
        for axis in 0..3 {
            let mut dims = current.dims;
            dims[axis] = 1;
            let mut next = Net {
                dims,
                data: vec![[I::point(0.); 3]; dims.iter().product()],
            };
            for base in next.coordinates().collect::<Vec<_>>() {
                let mut q: Vec<_> = (0..current.dims[axis])
                    .map(|i| {
                        let mut p = base;
                        p[axis] = i;
                        current.point(p)
                    })
                    .collect();
                for r in 1..q.len() {
                    for i in 0..q.len() - r {
                        for k in 0..3 {
                            q[i][k] = q[i][k].add(q[i + 1][k])?.mul(I::point(0.5))?;
                        }
                    }
                }
                let at = next.index(base);
                next.data[at] = q[0];
            }
            current = next;
        }
        Ok(current.data[0])
    }
}
fn hull(values: impl Iterator<Item = I>) -> I {
    values.fold(
        I {
            lo: f64::INFINITY,
            hi: f64::NEG_INFINITY,
        },
        |a, b| I {
            lo: a.lo.min(b.lo),
            hi: a.hi.max(b.hi),
        },
    )
}
fn determinant(a: [[I; 3]; 3]) -> Result<I> {
    let mut sum = I::point(0.);
    for i in 0..3 {
        sum = sum.add(
            a[0][i].mul(
                a[1][(i + 1) % 3]
                    .mul(a[2][(i + 2) % 3])?
                    .sub(a[1][(i + 2) % 3].mul(a[2][(i + 1) % 3])?)?,
            )?,
        )?;
    }
    Ok(sum)
}
enum Verdict {
    Excluded,
    Unique([I; 3], i8),
    Unresolved,
}
fn krawczyk(net: &Net) -> Result<Verdict> {
    if (0..3).any(|k| {
        let h = hull(net.data.iter().map(|p| p[k]));
        h.lo > 0. || h.hi < 0.
    }) {
        return Ok(Verdict::Excluded);
    }
    let mut jac = [[I::point(0.); 3]; 3];
    for axis in 0..3 {
        let degree = net.dims[axis] - 1;
        if degree == 0 {
            return Ok(Verdict::Unresolved);
        }
        for k in 0..3 {
            let values = net
                .coordinates()
                .filter(|p| p[axis] < degree)
                .map(|p| {
                    let mut q = p;
                    q[axis] += 1;
                    net.point(q)[k]
                        .sub(net.point(p)[k])?
                        .mul(I::point(degree as f64))
                })
                .collect::<Result<Vec<_>>>()?;
            jac[k][axis] = hull(values.into_iter());
        }
    }
    let a = jac.map(|r| r.map(|x| x.lo * 0.5 + x.hi * 0.5));
    let det = (0..3)
        .map(|i| {
            a[0][i]
                * (a[1][(i + 1) % 3] * a[2][(i + 2) % 3] - a[1][(i + 2) % 3] * a[2][(i + 1) % 3])
        })
        .sum::<f64>();
    if !det.is_finite() || det == 0. {
        return Ok(Verdict::Unresolved);
    }
    let mut inverse = [[0.; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            inverse[i][j] = (a[(j + 1) % 3][(i + 1) % 3] * a[(j + 2) % 3][(i + 2) % 3]
                - a[(j + 1) % 3][(i + 2) % 3] * a[(j + 2) % 3][(i + 1) % 3])
                / det;
        }
    }
    if !inverse.iter().flatten().all(|x| x.is_finite()) {
        return Ok(Verdict::Unresolved);
    }
    let d = determinant(inverse.map(|r| r.map(I::point)))?;
    if d.lo <= 0. && d.hi >= 0. {
        return Ok(Verdict::Unresolved);
    }
    let center = net.center()?;
    let mut image = [I::point(0.5); 3];
    let mut contraction = 0_f64;
    for i in 0..3 {
        for j in 0..3 {
            image[i] = image[i].sub(I::point(inverse[i][j]).mul(center[j])?)?;
        }
        let mut norm = I::point(0.);
        for k in 0..3 {
            let mut m = I::point(if i == k { 1. } else { 0. });
            for j in 0..3 {
                m = m.sub(I::point(inverse[i][j]).mul(jac[j][k])?)?;
            }
            norm = norm.add(I::point(m.lo.abs().max(m.hi.abs())))?;
            image[i] = image[i].add(m.mul(I::new(-0.5, 0.5)?)?)?;
        }
        contraction = contraction.max(norm.hi);
    }
    if image.iter().any(|x| x.hi < 0. || x.lo > 1.) {
        return Ok(Verdict::Excluded);
    }
    if contraction < 0.5 && image.iter().all(|x| x.lo > 0. && x.hi < 1.) {
        // ||I-RJ|| < 1 connects RJ to I through invertible matrices,
        // so det(RJ)>0. With the proved sign of det(R), this proves
        // the root orientation without sampling a normal or derivative.
        Ok(Verdict::Unique(image, if d.lo > 0. { 1 } else { -1 }))
    } else {
        Ok(Verdict::Unresolved)
    }
}
fn field_net(c: &Curve, s: &Surface, spans: [usize; 3], domain: Box3) -> Result<Net> {
    let ch = crate::curve_distance::restricted_controls(
        c,
        spans[0],
        I::new(domain[0][0], domain[0][1])?,
    )?;
    let sh = crate::curve_surface_composition::surface_net_on(
        s,
        [spans[1], spans[2]],
        [domain[1], domain[2]],
    )?;
    let origin = &c.control_points[spans[0] - c.degree];
    let dims = [ch.len(), sh.len(), sh[0].len()];
    let mut data = Vec::with_capacity(dims.iter().product());
    for cp in &ch {
        for row in &sh {
            for sp in row {
                let mut f = [I::point(0.); 3];
                for k in 0..3 {
                    // Curve extraction is translated by its original first span pole.
                    let hc = cp[k].add(I::point(origin[k]).mul(cp[3])?)?;
                    f[k] = hc.mul(sp[3])?.sub(sp[k].mul(cp[3])?)?;
                }
                data.push(f);
            }
        }
    }
    Ok(Net { dims, data })
}
fn map_box(span: Box3, cell: Box3, local: [I; 3]) -> Result<Box3> {
    let mut out = [[0.; 2]; 3];
    for k in 0..3 {
        let q = I::point(cell[k][0]).add(
            I::point(cell[k][1])
                .sub(I::point(cell[k][0]))?
                .mul(local[k])?,
        )?;
        let x = I::point(span[k][0])
            .add(I::point(span[k][1]).sub(I::point(span[k][0]))?.mul(q)?)?
            .intersect(span[k][0], span[k][1])?;
        out[k] = [x.lo, x.hi];
    }
    Ok(out)
}
// Avoid an ill-conditioned reparameterization when one coordinate is already
// at its binary64 enclosure floor while another still needs refinement. This
// enlarges a *search box*, never geometry or the root enclosure returned below.
// Every new inclusion must pass the unchanged strict Krawczyk conditions.
fn refinement_box(domain: Box3, original: Box3) -> Box3 {
    std::array::from_fn(|k| {
        let magnitude = original[k][0]
            .abs()
            .max(original[k][1].abs())
            .max(original[k][1] - original[k][0]);
        let floor = (magnitude * (256. * f64::EPSILON)).max(f64::from_bits(256));
        if domain[k][1] - domain[k][0] >= floor {
            return domain[k];
        }
        let middle = domain[k][0] * 0.5 + domain[k][1] * 0.5;
        [
            (middle - floor * 0.5)
                .next_down()
                .min(domain[k][0])
                .max(original[k][0]),
            (middle + floor * 0.5)
                .next_up()
                .max(domain[k][1])
                .min(original[k][1]),
        ]
    })
}

/// Complete parameter-product search on nonperiodic positive-weight NURBS.
/// Generic roots own strict Krawczyk proofs on explicit natural existence
/// domains within original knot products. The monotone affine-plane family has
/// an independent original-source proof. Coefficients enclose homogeneous data;
/// approximate extracted patches and sampled residuals never authorize roots.
/// Root boxes may be broad: callers must independently establish positional
/// accuracy before authoring a vertex. Distinct parameter roots stay distinct
/// even at the same world point. No trim, global self-intersection, topology
/// mutation or solid admission follows from this root report.
pub fn intersections(c: &Curve, s: &Surface, max_cells: usize) -> Result<Report> {
    c.validate()?;
    s.validate()?;
    check(
        c.control_points[0].len() == 3
            && s.control_points[0][0].len() == 3
            && !c.periodic
            && !s.periodic_u
            && !s.periodic_v,
        "Certified curve/surface roots need nonperiodic 3D sources",
    )?;
    check(
        (1..=100000).contains(&max_cells),
        "Certified root work must be in 1..100000 cells",
    )?;
    if !((c.degree + 1)
        .checked_mul(s.degree_u + 1)
        .and_then(|n| n.checked_mul(s.degree_v + 1))
        .is_some_and(|n| n <= 4096))
    {
        return Err(resource(
            "Certified curve/surface degree product exceeds 4096",
        ));
    }
    let cs: Vec<_> = (c.degree..c.control_points.len())
        .filter(|&i| c.knots[i] < c.knots[i + 1])
        .collect();
    let us: Vec<_> = (s.degree_u..s.control_points.len())
        .filter(|&i| s.knots_u[i] < s.knots_u[i + 1])
        .collect();
    let vs: Vec<_> = (s.degree_v..s.control_points[0].len())
        .filter(|&i| s.knots_v[i] < s.knots_v[i + 1])
        .collect();
    if !(cs
        .len()
        .checked_mul(us.len())
        .and_then(|n| n.checked_mul(vs.len()))
        .is_some_and(|n| n <= 1024))
    {
        return Err(resource(
            "Certified curve/surface span product exceeds 1024",
        ));
    }
    let alternate = affine_plane::attempt(c, s, max_cells)?;
    if let Some(report) = alternate.report {
        return Ok(report);
    }
    let full = [
        c.domain(),
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    if alternate.cells == max_cells {
        let mut unresolved = Vec::new();
        for &ct in &cs {
            for &su in &us {
                for &sv in &vs {
                    unresolved.push(Unresolved {
                        spans: [ct, su, sv],
                        normalized_cell: [[0., 1.]; 3],
                        parameter_box: [
                            [c.knots[ct], c.knots[ct + 1]],
                            [s.knots_u[su], s.knots_u[su + 1]],
                            [s.knots_v[sv], s.knots_v[sv + 1]],
                        ],
                    });
                }
            }
        }
        return Ok(Report {
            roots: vec![],
            unresolved,
            cells: alternate.cells,
            excluded: 0,
            complete: false,
        });
    }
    let result = window::all_in_window(c, s, full, max_cells - alternate.cells)?;
    let normalized = |ids: [usize; 3], domain: Box3| -> Result<Box3> {
        let knots = [
            c.knots.as_slice(),
            s.knots_u.as_slice(),
            s.knots_v.as_slice(),
        ];
        let mut out = [[0.; 2]; 3];
        for k in 0..3 {
            let a = I::point(knots[k][ids[k]]);
            let width = I::point(knots[k][ids[k] + 1]).sub(a)?;
            let x = I::new(domain[k][0], domain[k][1])?
                .sub(a)?
                .div(width)?
                .intersect(0., 1.)?;
            out[k] = [x.lo, x.hi];
        }
        Ok(out)
    };
    let mut roots = Vec::new();
    for r in result.roots {
        roots.push(Root {
            proof: RootProof::InteriorKrawczyk,
            spans: r.spans,
            normalized_cell: normalized(r.spans, r.existence_domain)?,
            existence_domain: Some(r.existence_domain),
            parameter_box: r.parameter_box,
            position_bounds: r.position_bounds,
            crossing_sign: r.crossing_sign,
        });
    }
    let mut unresolved = Vec::new();
    let knots = [
        c.knots.as_slice(),
        s.knots_u.as_slice(),
        s.knots_v.as_slice(),
    ];
    let degrees = [c.degree, s.degree_u, s.degree_v];
    let counts = [
        c.control_points.len(),
        s.control_points.len(),
        s.control_points[0].len(),
    ];
    for domain in result.unresolved {
        let mut ids = [0; 3];
        for k in 0..3 {
            ids[k] = (degrees[k]..counts[k])
                .find(|&i| {
                    knots[k][i] < knots[k][i + 1]
                        && knots[k][i] <= domain[k][0]
                        && domain[k][1] <= knots[k][i + 1]
                })
                .ok_or_else(|| crate::input("Root coverage cell lost its original knot owner"))?;
        }
        unresolved.push(Unresolved {
            spans: ids,
            normalized_cell: normalized(ids, domain)?,
            parameter_box: domain,
        });
    }
    roots.sort_by(|a, b| {
        a.parameter_box[0][0]
            .total_cmp(&b.parameter_box[0][0])
            .then(a.parameter_box[1][0].total_cmp(&b.parameter_box[1][0]))
            .then(a.parameter_box[2][0].total_cmp(&b.parameter_box[2][0]))
    });
    Ok(Report {
        roots,
        unresolved,
        cells: alternate.cells + result.cells,
        excluded: result.excluded,
        complete: result.complete,
    })
}

mod affine_plane;

mod window;
pub use window::{all_in_window, first_in_window, WindowReport, WindowRoot};

mod window_source;
pub use window_source::{
    all_iso_in_window, all_surface_iso_in_window, first_iso_in_window, first_surface_iso_in_window,
};

mod plane_contacts;
pub use plane_contacts::{affine_plane_contacts, AffinePlaneContact, AffinePlaneContacts};

mod original_patch_contacts;
pub use original_patch_contacts::{
    original_affine_patch_contacts, OriginalPatchContact, OriginalPatchContacts,
    OriginalPatchUvProof,
};

mod patch_uv;

mod patch_uv_injectivity;
pub use patch_uv_injectivity::BilinearUvInjectivity;

mod patch_exact_membership;

mod patch_coordinate_bounds;
mod quadric_carrier;
pub use quadric_carrier::{
    original_quadric_carrier_roots, OriginalQuadricCarrierRoot, OriginalQuadricCarrierRoots,
};
mod quadric_graph;
pub use quadric_graph::{
    original_quadric_graph_contacts, GraphBoundaryProof, OriginalQuadricGraphContact,
    OriginalQuadricGraphContacts, OriginalQuadricGraphParameterization,
    OriginalQuadricGraphUvProof,
};
