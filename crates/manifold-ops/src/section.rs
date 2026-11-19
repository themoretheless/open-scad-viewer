//! Plane section, capped plane cut, projection and coplanar simplification.
//!
//! Reuse, not duplication:
//! - [`plane_section`] maps the mesh into a [`PlaneFrame`] and calls the
//!   z-section of `mesh_section::MeshSectionIndex` (exact vertex/edge
//!   endpoint identities, typed `SECTION_*` errors);
//! - [`cut_by_plane`] intersects / subtracts an oriented half-space box via
//!   [`crate::boolean_manifold`], so both halves are capped and strictly manifold;
//! - [`project_to_plane`] reuses `mesh_section::project` (planar union);
//! - [`simplify_coplanar`] reuses `polygon_core::solid::primitives::simplify`
//!   and re-checks manifoldness and volume.

use crate::geom::{PlaneFrame, point};
use crate::{BooleanError, BooleanOp, boolean_manifold};
use mesh_topology::MeshView;

#[derive(Debug)]
pub enum SectionError {
    /// Bad plane, buffers or non-finite data.
    InvalidInput(String),
    /// Error from `mesh-section` (code and message preserved).
    Section { code: &'static str, message: String },
    /// Capping boolean failed.
    Boolean(BooleanError),
    /// The plane misses the mesh, so one half would be empty.
    EmptyHalf,
    /// Simplification changed manifoldness or volume.
    NotPreserved(String),
}

impl std::fmt::Display for SectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SectionError {}

fn from_core(e: math_core::Error) -> SectionError {
    SectionError::Section {
        code: e.code,
        message: e.message,
    }
}

/// Closed section contours of a plane.
#[derive(Debug, Clone)]
pub struct PlaneSection {
    pub frame: PlaneFrame,
    /// Closed polylines in world coordinates (last point connects to first).
    pub polylines: Vec<Vec<[f64; 3]>>,
    /// Same contours in frame coordinates `(u, v)`.
    pub contours: Vec<Vec<[f64; 2]>>,
}

impl PlaneSection {
    /// Signed-area sum of the contours (outer positive, holes negative for a
    /// consistently oriented closed mesh) and total perimeter.
    pub fn area_and_perimeter(&self) -> (f64, f64) {
        mesh_section::section_area_and_perimeter(&self.polylines)
    }
}

fn local_positions(positions: &[f64], frame: &PlaneFrame) -> Vec<f64> {
    (0..positions.len() / 3)
        .flat_map(|i| frame.to_local(point(positions, i)))
        .collect()
}

fn validate(
    positions: &[f64],
    indices: &[usize],
    origin: [f64; 3],
    normal: [f64; 3],
) -> Result<PlaneFrame, SectionError> {
    if !positions.len().is_multiple_of(3)
        || !indices.len().is_multiple_of(3)
        || positions.iter().any(|v| !v.is_finite())
    {
        return Err(SectionError::InvalidInput(
            "malformed or non-finite mesh buffers".into(),
        ));
    }
    PlaneFrame::new(origin, normal).ok_or_else(|| {
        SectionError::InvalidInput("plane needs a finite origin and nonzero normal".into())
    })
}

/// Section of a closed mesh by the plane through `origin` with `normal`.
pub fn plane_section(
    positions: &[f64],
    indices: &[usize],
    origin: [f64; 3],
    normal: [f64; 3],
) -> Result<PlaneSection, SectionError> {
    let frame = validate(positions, indices, origin, normal)?;
    let local = local_positions(positions, &frame);
    let index =
        mesh_section::MeshSectionIndex::new(&MeshView::new(&local, indices)).map_err(from_core)?;
    let section = index.section(0.0).map_err(from_core)?;
    let contours: Vec<Vec<[f64; 2]>> = section.contours.into_iter().map(|c| c.points).collect();
    let polylines = contours
        .iter()
        .map(|c| {
            c.iter()
                .map(|q| frame.to_world([q[0], q[1], 0.0]))
                .collect()
        })
        .collect();
    Ok(PlaneSection {
        frame,
        polylines,
        contours,
    })
}

/// Two capped, strictly manifold halves of a plane cut.
#[derive(Debug, Clone)]
pub struct PlaneCut {
    /// Part on the side the normal points to.
    pub positive: (Vec<f64>, Vec<usize>),
    pub negative: (Vec<f64>, Vec<usize>),
    pub section: PlaneSection,
}

/// Oriented box covering the half-space `height >= 0` within `extent` of the origin.
fn half_space_box(frame: &PlaneFrame, extent: f64) -> (Vec<f64>, Vec<usize>) {
    let l = extent;
    let corners = [
        [-l, -l, 0.0],
        [l, -l, 0.0],
        [l, l, 0.0],
        [-l, l, 0.0],
        [-l, -l, l],
        [l, -l, l],
        [l, l, l],
        [-l, l, l],
    ];
    let positions = corners.iter().flat_map(|c| frame.to_world(*c)).collect();
    let indices = vec![
        0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 2, 3, 7, 2, 7, 6, 1, 2, 6, 1, 6, 5,
        3, 0, 4, 3, 4, 7,
    ];
    (positions, indices)
}

/// Cut a closed manifold mesh by a plane and cap both halves.
pub fn cut_by_plane(
    positions: &[f64],
    indices: &[usize],
    origin: [f64; 3],
    normal: [f64; 3],
) -> Result<PlaneCut, SectionError> {
    let frame = validate(positions, indices, origin, normal)?;
    let section = plane_section(positions, indices, origin, normal)?;
    if section.contours.is_empty() {
        return Err(SectionError::EmptyHalf);
    }
    let local = local_positions(positions, &frame);
    let extent = local.iter().fold(1.0_f64, |m, v| m.max(v.abs())) * 4.0;
    let (bp, bi) = half_space_box(&frame, extent);
    let pos = boolean_manifold(positions, indices, &bp, &bi, BooleanOp::Intersection)
        .map_err(SectionError::Boolean)?;
    let neg = boolean_manifold(positions, indices, &bp, &bi, BooleanOp::Difference)
        .map_err(SectionError::Boolean)?;
    if pos.indices.is_empty() || neg.indices.is_empty() {
        return Err(SectionError::EmptyHalf);
    }
    Ok(PlaneCut {
        positive: (pos.positions, pos.indices),
        negative: (neg.positions, neg.indices),
        section,
    })
}

/// Silhouette of the mesh projected onto the plane, as frame-coordinate rings (union of triangles).
pub fn project_to_plane(
    positions: &[f64],
    indices: &[usize],
    origin: [f64; 3],
    normal: [f64; 3],
) -> Result<Vec<Vec<[f64; 2]>>, SectionError> {
    let frame = validate(positions, indices, origin, normal)?;
    let local = local_positions(positions, &frame);
    mesh_section::project(&MeshView::new(&local, indices)).map_err(from_core)
}

/// Merge coplanar triangles and retriangulate without changing the shape;
/// refuses results that lose manifoldness or change the volume.
pub fn simplify_coplanar(
    positions: &[f64],
    indices: &[usize],
) -> Result<(Vec<f64>, Vec<usize>), SectionError> {
    if !manifold_core::check(positions, indices).is_manifold() {
        return Err(SectionError::InvalidInput(
            "input must be a strict 2-manifold".into(),
        ));
    }
    let mesh = polygon_core::Mesh {
        positions: positions.to_vec(),
        indices: indices.to_vec(),
        uv: None,
    };
    let out = polygon_core::solid::primitives::simplify(&mesh).map_err(from_core)?;
    let welded = manifold_core::repair(&out.positions, &out.indices, 0.0);
    let (p, i) = (welded.positions, welded.indices);
    if !manifold_core::check(&p, &i).is_manifold() {
        return Err(SectionError::NotPreserved(
            "simplified mesh is not manifold".into(),
        ));
    }
    let v0 = mesh.volume();
    let v1 = polygon_core::Mesh {
        positions: p.clone(),
        indices: i.clone(),
        uv: None,
    }
    .volume();
    if (v1 - v0).abs() > 1e-9 * v0.abs().max(1.0) {
        return Err(SectionError::NotPreserved(format!(
            "volume changed {v0} -> {v1}"
        )));
    }
    Ok((p, i))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::fixtures::{cuboid, volume};

    fn torus() -> (Vec<f64>, Vec<usize>) {
        let m = polygon_core::solid::torus(3.0, 1.0, 48, 24);
        (m.positions, m.indices)
    }

    #[test]
    fn cube_tilted_section_is_one_closed_loop() {
        let (p, t) = cuboid([0.0; 3], [2.0; 3]);
        let s = plane_section(&p, &t, [1.0, 1.0, 1.0], [0.0, 0.0, 1.0]).unwrap();
        assert_eq!(s.polylines.len(), 1);
        let (area, perim) = s.area_and_perimeter();
        assert!(
            (area.abs() - 4.0).abs() < 1e-9 && (perim - 8.0).abs() < 1e-9,
            "{area} {perim}"
        );
        // Plane through the centre perpendicular to a body diagonal: regular hexagon of side
        // sqrt(2); face diagonals add collinear points on its sides.
        let s = plane_section(&p, &t, [1.0, 1.0, 1.0], [1.0, 1.0, 1.0]).unwrap();
        assert_eq!(s.polylines.len(), 1);
        assert!(s.polylines[0].len() >= 6);
        let (area, _) = s.area_and_perimeter();
        assert!((area.abs() - 3.0 * 3f64.sqrt()).abs() < 1e-9, "{area}");
        for q in &s.polylines[0] {
            assert!((q[0] + q[1] + q[2] - 3.0).abs() < 1e-12);
        }
    }

    #[test]
    fn torus_sections_have_expected_loop_counts() {
        let (p, t) = torus();
        // Equatorial plane: outer and inner circle.
        assert_eq!(
            plane_section(&p, &t, [0.0, 0.0, 0.1], [0.0, 0.0, 1.0])
                .unwrap()
                .polylines
                .len(),
            2
        );
        // Plane parallel to the axis, slightly off it: two disks.
        assert_eq!(
            plane_section(&p, &t, [0.05, 0.0, 0.0], [1.0, 0.0, 0.0])
                .unwrap()
                .polylines
                .len(),
            2
        );
        // Exactly through vertices that sit at |x| ~ 1e-16: mesh-section refuses with a typed error
        // instead of returning a collapsed contour.
        if let Err(e) = plane_section(&p, &t, [0.0; 3], [1.0, 0.0, 0.0]) {
            assert!(matches!(e, SectionError::Section { .. }));
        }
        // Plane missing the torus.
        assert!(
            plane_section(&p, &t, [0.0, 0.0, 5.0], [0.0, 0.0, 1.0])
                .unwrap()
                .polylines
                .is_empty()
        );
    }

    #[test]
    fn cut_cube_into_two_capped_halves() {
        let (p, t) = cuboid([0.0; 3], [2.0; 3]);
        let cut = cut_by_plane(&p, &t, [0.0, 0.0, 0.5], [0.0, 0.0, 1.0]).unwrap();
        for (hp, ht) in [&cut.positive, &cut.negative] {
            assert!(manifold_core::check(hp, ht).is_manifold());
        }
        assert!((volume(&cut.positive.0, &cut.positive.1) - 6.0).abs() < 1e-9);
        assert!((volume(&cut.negative.0, &cut.negative.1) - 2.0).abs() < 1e-9);
        assert!(matches!(
            cut_by_plane(&p, &t, [0.0, 0.0, 5.0], [0.0, 0.0, 1.0]),
            Err(SectionError::EmptyHalf)
        ));
    }

    #[test]
    fn cut_torus_tilted_conserves_volume() {
        let (p, t) = torus();
        let total = volume(&p, &t);
        let cut = cut_by_plane(&p, &t, [0.3, 0.0, 0.0], [1.0, 0.2, 0.1]).unwrap();
        for (hp, ht) in [&cut.positive, &cut.negative] {
            assert!(manifold_core::check(hp, ht).is_manifold());
        }
        let sum =
            volume(&cut.positive.0, &cut.positive.1) + volume(&cut.negative.0, &cut.negative.1);
        assert!((sum - total).abs() < 1e-6 * total, "{sum} vs {total}");
        assert_eq!(cut.section.polylines.len(), 2);
    }

    #[test]
    fn projection_and_coplanar_simplification() {
        let (p, t) = cuboid([0.0; 3], [2.0, 3.0, 4.0]);
        let rings = project_to_plane(&p, &t, [0.0; 3], [0.0, 0.0, 1.0]).unwrap();
        assert_eq!(rings.len(), 1);
        let refined = crate::transform::refine_midpoint(&p, &t, 1).unwrap();
        let (sp, si) = simplify_coplanar(&refined.0, &refined.1).unwrap();
        assert!(si.len() < refined.1.len());
        assert!((volume(&sp, &si) - 24.0).abs() < 1e-9);
        assert!(plane_section(&p, &t, [0.0; 3], [0.0; 3]).is_err());
    }
}
