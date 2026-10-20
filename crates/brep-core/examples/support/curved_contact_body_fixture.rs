//! Curved-support binding/end fixture; this does not authorize a solid graft.
use brep_core::{Model, contact_bind, contact_end, fillet_contact};
use nurbs_core::{
    curve::Curve,
    offset::fillet_surfaces::{FilletOptions, RadiusLaw},
};
pub struct Fixture {
    pub original: Model,
    pub source: Model,
    pub patch: Model,
    pub mapping: [usize; 4],
    pub keeps: [usize; 4],
    pub law_domain: [f64; 2],
    pub source_pcurves: Vec<Option<Curve>>,
    pub sides: [f64; 2],
}
pub fn build() -> Result<Fixture, Box<dyn std::error::Error>> {
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let line = |a: [f64; 2], b: [f64; 2]| Curve::from_polyline(vec![a.to_vec(), b.to_vec()]);
    let arc = |p: [[f64; 2]; 3]| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: p.map(|x| x.to_vec()).to_vec(),
        weights: vec![1., w, 1.],
        periodic: false,
    };
    let profile = vec![
        line([0., 0.], [10., 0.])?,
        line([10., 0.], [10., 10.])?,
        line([10., 10.], [0., 10.])?,
        line([0., 10.], [0., 6.])?,
        arc([[0., 6.], [3., 6.], [3., 3.]]),
        arc([[3., 3.], [3., 0.], [0., 0.]]),
    ];
    let base = brep_core::prism::extrude(&[profile], -7., -3.)?;
    let mut source = brep_core::transform::affine(
        &base,
        [
            [1., 0., 0., 0.],
            [0., 0., -1., 0.],
            [0., 1., 0., 0.],
            [0., 0., 0., 1.],
        ],
    )?;
    let floor = source
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 0.)
        })
        .unwrap();
    let cylinder = source
        .faces
        .iter()
        .position(|f| {
            f.surface.degree_u == 2
                && f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[2] <= 3.)
        })
        .unwrap();
    // Fixture charts extend the original extrusion support, while their actual
    // trim loops and unchanged 3D boundaries retain y=3..7 material membership.
    for id in [floor, cylinder] {
        for row in &mut source.faces[id].surface.control_points {
            row[0][1] = 10.;
            row[1][1] = 0.;
        }
        let outer = source.faces[id].outer;
        for c in &mut source.loops[outer].coedges {
            for p in &mut c.pcurve.control_points {
                p[1] = 0.3 + 0.4 * p[1];
            }
        }
    }
    source.validate()?;
    let a = source.faces[floor].surface.clone();
    let b = source.faces[cylinder].surface.clone();
    let na = a.evaluate(0.4, 0.4)?.unit_normal().unwrap();
    let e = b.evaluate(0.4, 0.4)?;
    let nb = e.unit_normal().unwrap();
    let signs = [
        na[2].signum(),
        (nb[0] * e.point[0] + nb[2] * (e.point[2] - 3.)).signum(),
    ];
    let built = fillet_contact::build_variable_g2(
        [&a, &b],
        RadiusLaw::Linear {
            start: 1.,
            end: 1.5,
        },
        &FilletOptions {
            side_a: signs[0],
            side_b: signs[1],
            max_sections: 5,
            ..Default::default()
        },
        1e-6,
        100000,
        1000000,
    )?;
    let contacts = built
        .contacts
        .as_ref()
        .ok_or("Contact construction unqualified")?;
    let skin = built.fillet.surface.as_ref().unwrap();
    let d = contacts[0].curve.domain();
    let increasing =
        contacts[0].curve.evaluate(d[1])?.point[1] > contacts[0].curve.evaluate(d[0])?.point[1];
    let ys = if increasing { [3., 7.] } else { [7., 3.] };
    let end_face = |m: &Model, y: f64| {
        m.faces
            .iter()
            .position(|f| f.surface.control_points.iter().flatten().all(|p| p[1] == y))
            .unwrap()
    };
    let cutter = |y: f64| -> Result<(Model, usize), Box<dyn std::error::Error>> {
        let m = brep_core::cuboid([-5., y, -5.], [15., y + 0.5, 15.])?;
        let f = end_face(&m, y);
        Ok((m, f))
    };
    let (first_cut, f) = cutter(ys[0])?;
    let (last_cut, l) = cutter(ys[1])?;
    let source_original = source.clone();
    let first = contact_bind::automatically_between_faces(
        &source,
        &contacts[0],
        [&a, skin],
        [(&first_cut, f, 0), (&last_cut, l, 0)],
        1e-8,
        100000,
        1000000,
    )?;

    let last = contact_bind::automatically_between_faces(
        &first.source,
        &contacts[1],
        [&b, skin],
        [(&first_cut, f, 0), (&last_cut, l, 0)],
        1e-8,
        100000,
        1000000,
    )?;

    let patch = contact_end::patch(
        [&first.restriction, &last.restriction],
        skin,
        [(&first_cut, f), (&last_cut, l)],
        last.source.tolerance_mm,
        1e-8,
        100000,
        1000000,
    )?;

    let mapping = [
        first.vertices[0],
        last.vertices[0],
        last.vertices[1],
        first.vertices[1],
    ];
    let source = &last.source;
    let vertex = |point: [f64; 3]| {
        source
            .vertices
            .iter()
            .position(|v| v.point == point)
            .unwrap()
    };
    let keeps = [
        vertex([10., ys[0], 10.]),
        vertex([3., ys[0], 3.]),
        vertex([10., ys[1], 10.]),
        vertex([10., ys[0], 0.]),
    ];
    Ok(Fixture {
        original: source_original,
        source: source.clone(),
        patch: patch.model,
        mapping,
        keeps,
        law_domain: d,
        source_pcurves: vec![
            None,
            Some(last.restriction.contact.support_pcurve.clone()),
            None,
            Some(first.restriction.contact.support_pcurve.clone()),
        ],
        sides: signs,
    })
}

pub struct SavedChecks {
    pub seams: Vec<nurbs_core::surface_g2_pcurves::Report>,
    pub radius: brep_core::fillet_radius::Report,
    pub cells: usize,
    pub exact_work: u64,
}
pub fn inspect_saved(
    f: &Fixture,
    out: &brep_core::contact_graft::Graft,
    mut cells: usize,
    mut exact: u64,
) -> Result<SavedChecks, Box<dyn std::error::Error>> {
    if cells > 100000 || exact > 1000000 {
        return Err("Initial admission work exceeds fixed limits".into());
    }
    let mut seams = Vec::new();
    for index in [3, 1] {
        let edge = out.patch_edges[index];
        let skin = &out.model.faces[out.patch_face];
        let skin_use = out.model.loops[skin.outer]
            .coedges
            .iter()
            .find(|c| c.edge == edge)
            .ok_or("Missing skin edge use")?;
        let (support, support_use) = out
            .model
            .faces
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != out.patch_face)
            .find_map(|(_, face)| {
                std::iter::once(face.outer)
                    .chain(face.holes.iter().copied())
                    .flat_map(|w| out.model.loops[w].coedges.iter())
                    .find(|c| c.edge == edge)
                    .map(|c| (face, c))
            })
            .ok_or("Missing original support edge use")?;
        let proof = nurbs_core::surface_g2_pcurves::inspect(
            &out.model.edges[edge].curve,
            [&support_use.pcurve, &skin_use.pcurve],
            [&support.surface, &skin.surface],
            [support_use.reversed, skin_use.reversed],
            1e-7,
            1e-5,
            1e-4,
            100000 - cells,
            1000000 - exact,
        )?;
        cells += proof.geometric.cells;
        exact += proof.positions.exact_work;
        seams.push(proof);
    }
    let radius = brep_core::fillet_radius::inspect_model(
        &out.model,
        out.patch_face,
        [out.patch_edges[3], out.patch_edges[1]],
        &RadiusLaw::Linear {
            start: 1.,
            end: 1.5,
        },
        f.law_domain,
        f.sides,
        1e-6,
        100000 - cells,
        1000000 - exact,
    )?;
    cells += radius.cells;
    exact += radius.exact_work;
    Ok(SavedChecks {
        seams,
        radius,
        cells,
        exact_work: exact,
    })
}
