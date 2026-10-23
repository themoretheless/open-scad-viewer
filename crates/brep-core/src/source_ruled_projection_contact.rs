//! Original Jordan projection confines ruled-wall contacts to owned boundaries.
use crate::{
    source_contour_proposal::SourceRegion,
    source_fiber_boundary::{self, Locus},
    source_shared_edge::SharedEdge,
    source_shell_incidence::{Address, Shell},
};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface, surface_projected_jordan};
pub struct Certificate {
    faces: [usize; 2],
    regions: [SourceRegion; 2],
    projection: surface_projected_jordan::Certificate,
    edges: Vec<SharedEdge>,
}
impl Certificate {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn regions(&self) -> &[SourceRegion; 2] {
        &self.regions
    }
    pub fn projection(&self) -> &surface_projected_jordan::Certificate {
        &self.projection
    }
    pub fn edges(&self) -> &[SharedEdge] {
        &self.edges
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub driver_cells: usize,
    pub reason: &'static str,
}
fn clamped(s: &Surface) -> bool {
    let test = |d: usize, k: &[f64], n: usize| {
        (1..=8).contains(&d)
            && n == d + 1
            && k.len() == 2 * (d + 1)
            && k[0] < k[d + 1]
            && k[..=d].iter().all(|t| *t == k[0])
            && k[d + 1..].iter().all(|t| *t == k[d + 1])
    };
    s.control_points.iter().flatten().all(|p| p.len() == 3)
        && !s.periodic_u
        && !s.periodic_v
        && test(s.degree_u, &s.knots_u, s.control_points.len())
        && test(s.degree_v, &s.knots_v, s.control_points[0].len())
        && s.weights.iter().flatten().all(|w| *w > 0.)
}
fn natural(s: &Surface, index: usize) -> Vec<(usize, usize)> {
    let (p, q) = (s.degree_u, s.degree_v);
    match index {
        0 => (0..=p).map(|i| (i, 0)).collect(),
        1 => (0..=q).map(|j| (p, j)).collect(),
        2 => (0..=p).rev().map(|i| (i, q)).collect(),
        _ => (0..=q).rev().map(|j| (0, j)).collect(),
    }
}
fn boundary(index: usize) -> (usize, bool) {
    [(1, false), (0, true), (1, true), (0, false)][index]
}
fn trace(s: &Surface, axes: [usize; 2], linear: usize) -> Option<Curve> {
    if [s.degree_u, s.degree_v][linear] != 1 {
        return None;
    }
    let count = if linear == 1 {
        s.control_points.len()
    } else {
        s.control_points[0].len()
    };
    let indices = (0..count)
        .map(|i| {
            if linear == 1 {
                (i, 0, i, 1)
            } else {
                (0, i, 1, i)
            }
        })
        .collect::<Vec<_>>();
    if indices.iter().any(|&(i, j, k, l)| {
        s.weights[i][j] != s.weights[k][l]
            || axes
                .iter()
                .any(|&a| s.control_points[i][j][a] != s.control_points[k][l][a])
    }) {
        return None;
    }
    let degree = count - 1;
    Some(Curve {
        degree,
        knots: std::iter::repeat_n(0., count)
            .chain(std::iter::repeat_n(1., count))
            .collect(),
        control_points: indices
            .iter()
            .map(|&(i, j, _, _)| axes.map(|a| s.control_points[i][j][a]).to_vec())
            .collect(),
        weights: indices
            .iter()
            .map(|&(i, j, _, _)| s.weights[i][j])
            .collect(),
        periodic: false,
    })
}
pub fn certify(
    shell: &Shell,
    faces: [usize; 2],
    tolerance: f64,
    max_work: u64,
    max_driver: usize,
) -> Result<Report> {
    let regions = shell
        .regions()
        .ok_or_else(|| Error::new("BREP_SOURCE_RULED", "Original regions required"))?;
    if faces[0] == faces[1]
        || faces.iter().any(|f| *f >= regions.len())
        || !tolerance.is_finite()
        || tolerance <= 0.
        || !(1..=100_000_000).contains(&max_work)
        || !(1..=100000).contains(&max_driver)
    {
        return Err(Error::new(
            "BREP_SOURCE_RULED",
            "Choose distinct faces and bounded work",
        ));
    }
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        driver_cells: 0,
        reason: "source-ruled-projection-contact-unproven",
    };
    let shared = shell
        .uses()
        .iter()
        .enumerate()
        .filter(|(_, uses)| uses.iter().all(|a| faces.contains(&a.face)))
        .collect::<Vec<_>>();
    if shared.is_empty() {
        return Ok(out);
    }
    for slot in 0..2 {
        let curved = faces[slot];
        let wall = faces[1 - slot];
        // Natural chart ownership is required here. Arbitrary trimmed charts
        // require an additional material/fiber restriction proof.
        if !regions[curved].whole_chart_material() {
            continue;
        }
        let s = regions[curved].loops()[0][0].surface();
        let w = regions[wall].loops()[0][0].surface();
        if !clamped(s) || !clamped(w) {
            continue;
        }
        let addresses = shared
            .iter()
            .map(|(_, uses)| *uses.iter().find(|a| a.face == curved).unwrap())
            .collect::<Vec<_>>();
        for axes in [[0, 1], [0, 2], [1, 2]] {
            for linear in 0..2 {
                let Some(rail) = trace(w, axes, linear) else {
                    continue;
                };
                for index in 0..4 {
                    let indices = natural(s, index);
                    let points = indices
                        .iter()
                        .map(|&(i, j)| axes.map(|a| s.control_points[i][j][a]).to_vec())
                        .collect::<Vec<_>>();
                    let weights = indices
                        .iter()
                        .map(|&(i, j)| s.weights[i][j])
                        .collect::<Vec<_>>();
                    let matches = (0..2).any(|reverse| {
                        points.len() == rail.control_points.len()
                            && (0..points.len()).all(|i| {
                                let j = if reverse == 0 {
                                    i
                                } else {
                                    points.len() - 1 - i
                                };
                                points[j] == rail.control_points[i] && weights[j] == rail.weights[i]
                            })
                    });
                    if !matches || points.iter().all(|p| p == &points[0]) {
                        continue;
                    }
                    if out.exact_work == max_work || out.driver_cells == max_driver {
                        return Ok(out);
                    }
                    let proof = surface_projected_jordan::certify(
                        s,
                        axes,
                        tolerance,
                        max_work - out.exact_work,
                        max_driver - out.driver_cells,
                    )?;
                    out.exact_work += proof.exact_work;
                    out.driver_cells += proof.boundary_cells;
                    let Some(projection) = proof.certificate else {
                        continue;
                    };
                    let mut owned = true;
                    for &collapsed in projection.collapsed_boundaries() {
                        let entries = natural(s, collapsed);
                        let (i, j) = entries[0];
                        let point = &s.control_points[i][j];
                        if entries
                            .iter()
                            .any(|&(i, j)| &s.control_points[i][j] != point)
                        {
                            owned = false;
                            break;
                        }
                        let point = [point[0], point[1], point[2]];
                        let mut pole_owned = false;
                        for (address, pole) in shell
                            .poles()
                            .iter()
                            .filter(|(a, p)| a.face == curved && p.point() == point)
                        {
                            if out.driver_cells == max_driver {
                                return Ok(out);
                            }
                            let p = source_fiber_boundary::inspect_natural(
                                pole.source(),
                                s,
                                boundary(collapsed),
                                max_driver - out.driver_cells,
                            )?;
                            out.driver_cells += p.driver_cells;
                            let ids = shell.vertices()[curved][address.wire][address.edge];
                            pole_owned |= p.locus == Locus::EntireFragment
                                && ids[0] == ids[1]
                                && addresses.iter().any(|a| {
                                    (0..2).any(|end| {
                                        shell.vertices()[curved][a.wire][a.edge][end] == ids[0]
                                            && crate::source_vertex_contact::corner(
                                                &regions[curved].loops()[a.wire][a.edge],
                                                end,
                                            ) == Some(point)
                                    })
                                });
                        }
                        owned &= pole_owned;
                    }
                    let mut contact_use = false;
                    for (wire, fragments) in regions[curved].loops().iter().enumerate() {
                        for (edge, fragment) in fragments.iter().enumerate() {
                            if out.driver_cells == max_driver {
                                return Ok(out);
                            }
                            let p = source_fiber_boundary::inspect_natural(
                                fragment,
                                s,
                                boundary(index),
                                max_driver - out.driver_cells,
                            )?;
                            out.driver_cells += p.driver_cells;
                            let address = Address {
                                face: curved,
                                wire,
                                edge,
                            };
                            let vertex_owned = |end| {
                                addresses.iter().any(|a| {
                                    shell.vertices()[curved][a.wire][a.edge]
                                        .contains(&shell.vertices()[curved][wire][edge][end])
                                })
                            };
                            owned &= match p.locus {
                                Locus::EntireFragment => {
                                    contact_use |= addresses.contains(&address);
                                    addresses.contains(&address)
                                }
                                Locus::Endpoints(ends) => {
                                    (0..2).all(|i| !ends[i] || vertex_owned(i))
                                }
                                Locus::Away => true,
                                _ => false,
                            };
                        }
                    }
                    if owned && contact_use {
                        out.certificate = Some(Certificate {
                            faces,
                            regions: [regions[faces[0]].clone(), regions[faces[1]].clone()],
                            projection,
                            edges: shared
                                .iter()
                                .map(|(i, _)| shell.edges()[*i].clone())
                                .collect(),
                        });
                        out.reason = "source-ruled-projection-contact-qualified";
                        return Ok(out);
                    }
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ruled_projection_requires_original_coordinate_and_weight_equality() {
        let span = crate::circular_blend::plane_cylinder_transition(
            20.,
            6.,
            0.,
            1.25,
            0.,
            std::f64::consts::FRAC_PI_6,
        )
        .unwrap();
        let sheet = span.trimmed_cylinder_sheet(1e-7).unwrap();
        let s = &sheet.faces[0].surface;
        assert!(clamped(s));
        assert!(trace(s, [0, 1], 1).is_some());
        let mut changed = s.clone();
        changed.control_points[2][1][0] =
            f64::from_bits(changed.control_points[2][1][0].to_bits() + 1);
        assert!(trace(&changed, [0, 1], 1).is_none());
        changed = s.clone();
        changed.weights[2][1] = f64::from_bits(changed.weights[2][1].to_bits() + 1);
        assert!(trace(&changed, [0, 1], 1).is_none());
        changed = s.clone();
        changed.control_points[2][1][2] += 0.25;
        // The original projected trace remains the same; 3D contact ownership
        // still requires the original shell's independently proven carriers.
        assert!(trace(&changed, [0, 1], 1).is_some());
    }
}
