use brep_core::{Coedge, Model, face_cut, region_replacement::RegionPatch};
use nurbs_core::{curve::Curve, curve_surface_agreement, surface::Surface};
#[path = "corner_close_fixture.rs"]
mod source_fixture;
pub struct Fixture {
    pub source: Model,
    pub chart: Surface,
    pub loops: [Vec<Vec<Coedge>>; 2],
    pub selected: [[usize; 1]; 2],
    pub reversed: [bool; 2],
    pub cut_edges: Vec<usize>,
}
impl Fixture {
    pub fn plans(&self) -> [RegionPatch<'_>; 2] {
        [0, 1].map(|i| RegionPatch {
            selected: &self.selected[i],
            surface: &self.chart,
            loops: &self.loops[i],
            reversed: self.reversed[i],
        })
    }
}
pub fn build() -> Fixture {
    let (open, supports, options) = source_fixture::fixture();
    let closed = brep_core::corner_close::close(
        &open,
        0,
        supports,
        options,
        1e-8,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    let chart = closed.model.faces[closed.face].surface.clone();
    let outer = &closed.model.loops[closed.model.faces[closed.face].outer].coedges;
    let ends =
        [0, 2].map(|i| closed.model.edges[outer[i].edge].vertices[usize::from(outer[i].reversed)]);
    let pc = Curve::from_polyline(
        [0, 2]
            .map(|i| outer[i].pcurve.control_points[0].clone())
            .to_vec(),
    )
    .unwrap();
    let owned = ends.map(|v| closed.model.vertices[v].point);
    let lifted = curve_surface_agreement::lift(&pc, &chart, Some(owned), 1e-7, 100000).unwrap();
    let divided = face_cut::partition(
        &closed.model,
        closed.face,
        ends,
        &[face_cut::Segment {
            curve: lifted.curve.unwrap(),
            pcurve: pc,
        }],
        1e-8,
        100000,
    )
    .unwrap();
    let source = divided.model;
    let loops = divided
        .faces
        .map(|f| vec![source.loops[source.faces[f].outer].coedges.clone()]);
    let reversed = divided.faces.map(|f| {
        source.shells[0]
            .faces
            .iter()
            .find(|u| u.face == f)
            .unwrap()
            .reversed
    });
    let selected = divided.faces.map(|f| [f]);
    Fixture {
        source,
        chart,
        loops,
        selected,
        reversed,
        cut_edges: divided.edges,
    }
}
