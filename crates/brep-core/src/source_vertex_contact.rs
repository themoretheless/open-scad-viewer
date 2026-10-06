//! Exact supporting-plane contact confined to an owned original chart vertex.
use crate::{
    source_boundary_fragment::{Endpoint, Fragment},
    source_contour_proposal::SourceRegion,
    source_shell_incidence::{Address, Shell},
};
use cad_predicates::Sign;
use nurbs_core::{Error, Result};
use std::collections::BTreeMap;
pub struct Certificate {
    faces: [usize; 2],
    regions: [SourceRegion; 2],
    vertex: usize,
    point: [f64; 3],
    plane: [[f64; 3]; 3],
    uses: [(Address, usize); 2],
    material_hulls: [Option<crate::source_planar_material_hull::Certificate>;2],
    corner_image: Option<crate::source_corner_plane_image::Certificate>,
}
impl Certificate {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn regions(&self) -> &[SourceRegion; 2] {
        &self.regions
    }
    pub fn vertex(&self) -> usize {
        self.vertex
    }
    pub fn point(&self) -> [f64; 3] {
        self.point
    }
    pub fn plane(&self) -> [[f64; 3]; 3] {
        self.plane
    }
    pub fn uses(&self) -> &[(Address, usize); 2] {
        &self.uses
    }
    pub fn corner_image(&self)->Option<&crate::source_corner_plane_image::Certificate> {self.corner_image.as_ref()}
    pub fn material_hulls(&self)->&[Option<crate::source_planar_material_hull::Certificate>;2] {&self.material_hulls}
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub reason: &'static str,
}
pub(crate) fn corner(source: &Fragment, end: usize) -> Option<[f64; 3]> {
    let Endpoint::Parameter(t) = source.endpoints()[end] else {
        return None;
    };
    let curve = source.curve();
    let domain = curve.domain();
    let side = domain.iter().position(|v| *v == t)?;
    let clamped = |knots: &[f64], degree: usize, n: usize| {
        knots[..=degree].iter().all(|v| *v == knots[degree])
            && knots[n..].iter().all(|v| *v == knots[n])
    };
    if !clamped(&curve.knots, curve.degree, curve.control_points.len()) {
        return None;
    }
    let uv = &curve.control_points[if side == 0 {
        0
    } else {
        curve.control_points.len() - 1
    }];
    let s = source.surface();
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let mut indexes = [0; 2];
    for (axis, (knots, degree)) in [(&s.knots_u, s.degree_u), (&s.knots_v, s.degree_v)]
        .into_iter()
        .enumerate()
    {
        if !clamped(knots, degree, counts[axis]) {
            return None;
        }
        indexes[axis] = if uv[axis] == knots[degree] {
            0
        } else if uv[axis] == knots[counts[axis]] {
            counts[axis] - 1
        } else {
            return None;
        };
    }
    Some(
        s.control_points[indexes[0]][indexes[1]]
            .as_slice()
            .try_into()
            .unwrap(),
    )
}
/// Exact clamped canonical carrier endpoint, including a UV point inside its
/// surface chart. The private SharedEdge proves the full original composition.
/// Mapped ranges and root-valued endpoints require their own point proof.
pub(crate) fn carrier_corner(shell: &Shell, address: Address, end: usize) -> Option<[f64;3]> {
    if end>1 {return None;}
    let (index,slot)=shell.uses().iter().enumerate().find_map(|(i,uses)| {
        uses.iter().position(|a|*a==address).map(|slot|(i,slot))
    })?;
    let shared=&shell.edges()[index];
    if shared.ranges().is_some() {return None;}
    let source=&shared.uses()[slot];
    let Endpoint::Parameter(t)=source.endpoints()[end] else {return None;};
    let domain=source.curve().domain();
    let side=domain.iter().position(|p|*p==t)?;
    let world=shared.world();
    let n=world.control_points.len();
    let d=world.domain();
    if world.periodic || !world.knots[..=world.degree].iter().all(|t|*t==d[0])
        || !world.knots[n..].iter().all(|t|*t==d[1]) {return None;}
    let reversed=shared.reversed()[slot]^source.reversed();
    let canonical=side^usize::from(reversed);
    let i=if canonical==0 {0}else{n-1};
    if world.weights[i]<=0. {return None;}
    world.control_points[i].as_slice().try_into().ok()
}
pub fn certify(shell: &Shell, faces: [usize; 2], max_work: u64) -> Result<Report> {
    let regions = shell
        .regions()
        .ok_or_else(|| Error::new("BREP_SOURCE_CONTACT", "Qualified regions required"))?;
    if faces[0] == faces[1]
        || faces.iter().any(|&f| f >= regions.len())
        || !(1..=100_000_000).contains(&max_work)
    {
        return Err(Error::new(
            "BREP_SOURCE_CONTACT",
            "Choose distinct source faces and bounded vertex work",
        ));
    }
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        reason: "source-vertex-corner-ownership-unproven",
    };
    let collect = |face: usize| {
        let mut vertices = BTreeMap::new();
        for (wire, fragments) in regions[face].loops().iter().enumerate() {
            for (edge, source) in fragments.iter().enumerate() {
                for end in 0..2 {
                    let address=Address {face,wire,edge};
                    if let Some(point)=corner(source,end).or_else(||carrier_corner(shell,address,end)) {
                        vertices.entry(shell.vertices()[face][wire][edge][end])
                            .or_insert((point,(address,end)));
                    }
                }
            }
        }
        vertices
    };
    let vertices = faces.map(collect);
    if !vertices[0].keys().any(|v| vertices[1].contains_key(v)) {
        return Ok(out);
    }

    let mut material_hulls=[None,None];
    for slot in 0..2 {
        if out.exact_work==max_work {return Ok(out);}
        let r=crate::source_planar_material_hull::certify(shell,faces[slot],max_work-out.exact_work)?;
        out.exact_work+=r.exact_work;material_hulls[slot]=r.certificate;
    }
    let points=std::array::from_fn::<_,2,_>(|slot| {
        material_hulls[slot].as_ref().map(|h|h.points().to_vec()).unwrap_or_else(|| {
            regions[faces[slot]].loops()[0][0].surface().control_points.iter().flatten()
                .map(|p|[p[0],p[1],p[2]]).collect::<Vec<_>>()
        })
    });
    let centers=points.each_ref().map(|points| {
        let count=points.len() as f64;
        std::array::from_fn::<_,3,_>(|k|points.iter().map(|p|p[k]/count).sum::<f64>())
    });
    for (vertex, (point, first)) in &vertices[0] {
        if out.exact_work == max_work {
            out.reason = "source-vertex-contact-work-limit";
            return Ok(out);
        }
        let Some((other, second)) = vertices[1].get(vertex) else {
            continue;
        };
        if point != other {
            continue;
        }
        let mut normal = std::array::from_fn::<_, 3, _>(|k| centers[0][k] - centers[1][k]);
        let scale = normal.iter().map(|v| v.abs()).fold(0_f64, f64::max);
        if scale == 0. || !scale.is_finite() {
            continue;
        }
        for v in &mut normal {
            *v /= scale;
        }
        let axis = (0..3)
            .min_by(|&a, &b| normal[a].abs().total_cmp(&normal[b].abs()))
            .unwrap();
        let mut basis = [0.; 3];
        basis[axis] = 1.;
        let cross = |a: [f64; 3], b: [f64; 3]| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let x = cross(normal, basis);
        let y = cross(normal, x);
        let plane = [
            *point,
            std::array::from_fn(|k| point[k] + x[k]),
            std::array::from_fn(|k| point[k] + y[k]),
        ];
        if plane.iter().flatten().any(|v| !v.is_finite()) {
            continue;
        }
        out.reason = "source-vertex-support-unproven";
        if !crate::source_allowed_contact::independent(
            plane[0],
            plane[1],
            plane[2],
            &mut out.exact_work,
            max_work,
        )? {
            continue;
        }
        let mut sides = [None; 2];
        let mut supported = true;
        for slot in 0..2 {
            for &world in &points[slot] {
                let Some(sign) = crate::source_allowed_contact::orient(
                    &[plane[0], plane[1], plane[2], world],
                    None,
                    &mut out.exact_work,
                    max_work,
                )?
                else {
                    out.reason = "source-vertex-contact-work-limit";
                    return Ok(out);
                };
                if sign == Sign::Zero {
                    supported &= world == *point;
                } else if sides[slot].is_some_and(|s| s != sign) {
                    supported = false;
                } else {
                    sides[slot] = Some(sign);
                }
            }
        }
        if supported && sides.iter().all(Option::is_some) && sides[0] != sides[1] {
            out.certificate = Some(Certificate {
                faces,
                regions: [regions[faces[0]].clone(), regions[faces[1]].clone()],
                vertex: *vertex,
                point: *point,
                plane,
                uses: [*first, *second],
                material_hulls,
                corner_image: None,
            });
            out.reason = "source-vertex-contact-qualified";
            return Ok(out);
        }
    }
    // A coplanar face need not have a strict side. The other original chart
    // can independently prove its entire plane image is the owned point.
    for (vertex,(point,first)) in &vertices[0] {
        let Some((other,second))=vertices[1].get(vertex) else {continue;};
        if point!=other {continue;}
        for planar in 0..2 {
            if out.exact_work==max_work {return Ok(out);}
            let Some(plane)=crate::source_allowed_contact::plane(
                regions[faces[planar]].loops()[0][0].surface(),&mut out.exact_work,max_work,
            )? else {continue;};
            if out.exact_work==max_work {return Ok(out);}
            let r=crate::source_corner_plane_image::certify(
                regions[faces[1-planar]].loops()[0][0].surface(),plane,*point,max_work-out.exact_work,
            )?;
            out.exact_work+=r.exact_work;
            if let Some(corner_image)=r.certificate {
                out.certificate=Some(Certificate {
                    faces,regions:[regions[faces[0]].clone(),regions[faces[1]].clone()],
                    vertex:*vertex,point:*point,plane,uses:[*first,*second],material_hulls,
                    corner_image:Some(corner_image),
                });
                out.reason="source-vertex-plane-image-qualified";
                return Ok(out);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_interior_parameter_does_not_claim_a_natural_world_corner() {
        let spans =
            crate::linear_canal::construct([[0.; 3], [0., 0., 8.]], [0.5, 1.], [1., 0., 0.], 0.7)
                .unwrap();
        let curve =
            nurbs_core::curve::Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let source = Fragment::new(
            spans[0].surface(),
            &curve,
            Endpoint::Parameter(0.25),
            Endpoint::Parameter(1.),
        )
        .unwrap();
        assert!(corner(&source, 0).is_none());
        assert_eq!(
            corner(&source, 1),
            Some(
                spans[0].surface().control_points[1][2]
                    .as_slice()
                    .try_into()
                    .unwrap()
            )
        );
    }
    #[test]
    fn original_vertex_contacts_qualify_without_an_edge_and_refuse_exhausted_work() {
        for radii in [[0.5, 1.25], [1.25, 0.5], [0., 1.], [1., 0.]] {
            for sweep in [std::f64::consts::TAU, -std::f64::consts::TAU] {
                let spans = crate::linear_canal::construct(
                    [[10., -7., 5.], [13., -3., 17.]],
                    radii,
                    [1., 0., 0.],
                    sweep,
                )
                .unwrap();
                let shell = crate::linear_canal::to_capped_source_shell(
                    &spans,
                    1e-7,
                    1e-8,
                    crate::trimmed_face_recipe::Limits {
                        pairs: 10000,
                        region_cells: 10000,
                        domain_cells: 10000,
                        agreement_cells: 10000,
                    },
                    100_000_000,
                )
                .unwrap()
                .shell
                .unwrap();
                let mut qualified = 0;
                for b in 1..shell.faces().len() {
                    if shell
                        .uses()
                        .iter()
                        .any(|u| u.iter().any(|a| a.face == 0) && u.iter().any(|a| a.face == b))
                    {
                        continue;
                    }
                    let r = certify(&shell, [0, b], 1000000).unwrap();
                    if let Some(c) = r.certificate {
                        qualified += 1;
                        assert_eq!(c.faces(), [0, b]);
                        for (a, end) in c.uses() {
                            assert_eq!(shell.vertices()[a.face][a.wire][a.edge][*end], c.vertex());
                        }
                        let back = certify(&shell, [b, 0], 1000000)
                            .unwrap()
                            .certificate
                            .unwrap();
                        assert_eq!(c.point(), back.point());
                        assert_eq!(c.vertex(), back.vertex());
                        assert!(certify(&shell, [0, b], 1).unwrap().certificate.is_none());
                    }
                }
                assert!(qualified > 0, "radii={radii:?} sweep={sweep}");
                assert!(certify(&shell, [0, 0], 1000000).is_err());
            }
        }
    }
}
