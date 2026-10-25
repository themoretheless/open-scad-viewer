//! Closed quarter-sector control of an admitted unequal-radius G2 transition.
//! Only the two circular band/transition seams claim G2; end/radial faces are sharp.
use crate::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, Vertex};
use nurbs_core::{
    Error, Result,
    curve::Curve,
    radius_transition,
    surface::{Axis, Surface},
};
use std::collections::BTreeMap;
pub struct Report {
    pub model: Model,
    pub transition: radius_transition::Report,
    /// Actual saved circle edges at the two admitted G2 joins.
    pub smooth_edges: [usize; 2],
    pub trims: crate::face_domain::TrimAudit,
    pub closure: crate::watertight::QualifiedReport,
    pub cells: usize,
    pub exact_work: u64,
}
fn refused(s: &str) -> Error {
    Error::new("BREP_RADIUS_TRANSITION_UNPROVEN", s)
}
fn edge(
    model: &mut Model,
    edges: &mut BTreeMap<[usize; 2], usize>,
    vertices: [usize; 2],
    curve: Curve,
) -> usize {
    let mut key = vertices;
    key.sort();
    *edges.entry(key).or_insert_with(|| {
        let id = model.edges.len();
        model.edges.push(Edge {
            curve,
            vertices,
            degenerate: false,
        });
        id
    })
}
fn quad(
    model: &mut Model,
    edges: &mut BTreeMap<[usize; 2], usize>,
    surface: Surface,
    vertices: [usize; 4],
    reversed: bool,
) -> Result<()> {
    let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let curves = [
        surface.iso(Axis::V, 0.)?,
        surface.iso(Axis::U, 1.)?,
        surface.iso(Axis::V, 1.)?,
        surface.iso(Axis::U, 0.)?,
    ];
    let pairs = [
        [vertices[0], vertices[1]],
        [vertices[1], vertices[2]],
        [vertices[3], vertices[2]],
        [vertices[0], vertices[3]],
    ];
    let mut ring = Vec::new();
    for (i, curve) in curves.into_iter().enumerate() {
        let id = edge(model, edges, pairs[i], curve);
        let traversal = [vertices[i], vertices[(i + 1) % 4]];
        ring.push(Coedge::new(
            id,
            model.edges[id].vertices != traversal,
            Curve::from_polyline(vec![uv[i].to_vec(), uv[(i + 1) % 4].to_vec()])?,
        ));
    }
    let outer = model.loops.len();
    let face = model.faces.len();
    model.loops.push(Loop { coedges: ring });
    model.faces.push(Face {
        surface,
        outer,
        holes: vec![],
    });
    model.shells[0].faces.push(FaceUse { face, reversed });
    Ok(())
}
/// Build a closed native control with source bands and the ACTUAL qualified
/// bridge. Caps use injective plane charts with quarter-disk trims. No
/// degenerate polar cap, proximity welding or whole-body G2 flag is used.
pub fn build(o: radius_transition::Options) -> Result<Report> {
    let transition = radius_transition::build(o)?;
    if !transition.admitted {
        return Err(refused(
            "Original transition radius/G2/interior is unproven",
        ));
    }
    let skins = [
        &transition.original_supports[0],
        &transition.surface,
        &transition.original_supports[1],
    ];
    let origin = &transition.centerline.control_points[0];
    let last = &transition.centerline.control_points[1];
    let center = |phase: f64| {
        (0..3)
            .map(|k| origin[k] + phase * (last[k] - origin[k]))
            .collect::<Vec<_>>()
    };
    let mut model = Model::empty(o.tolerances[0])?;
    for station in 0..4 {
        let (skin, v) = match station {
            0 => (skins[0], 0),
            1 => (skins[0], 1),
            2 => (skins[2], 0),
            _ => (skins[2], 1),
        };
        for p in [
            center(station as f64 - 1.),
            skin.control_points[0][v].clone(),
            skin.control_points[2][v].clone(),
        ] {
            model.vertices.push(Vertex {
                point: p.try_into().unwrap(),
            });
        }
    }
    model.shells.push(Shell {
        faces: vec![],
        closed: true,
    });
    model.bodies.push(Body {
        outer_shell: 0,
        inner_shells: vec![],
    });
    let mut edges = BTreeMap::new();
    for (band, skin) in skins.into_iter().enumerate() {
        let low = 3 * band;
        let high = 3 * (band + 1);
        quad(
            &mut model,
            &mut edges,
            skin.clone(),
            [low + 1, low + 2, high + 2, high + 1],
            false,
        )?;
        for (row, corner, reversed) in [(0, 1, false), (skin.control_points.len() - 1, 2, true)] {
            if skin.weights[row].iter().any(|&w| w != 1.) {
                return Err(refused(
                    "Side rail requires the authored unit-weight section boundary",
                ));
            }
            let rail = (0..=skin.degree_v)
                .map(|j| center(band as f64 - 1. + j as f64 / skin.degree_v as f64))
                .collect::<Vec<_>>();
            let side = Surface {
                degree_u: 1,
                degree_v: skin.degree_v,
                knots_u: vec![0., 0., 1., 1.],
                knots_v: skin.knots_v.clone(),
                control_points: vec![rail, skin.control_points[row].clone()],
                weights: vec![vec![1.; skin.degree_v + 1]; 2],
                periodic_u: false,
                periodic_v: false,
            };
            quad(
                &mut model,
                &mut edges,
                side,
                [low, low + corner, high + corner, high],
                reversed,
            )?;
        }
    }
    for (station, skin, v, reversed) in [(0, skins[0], 0., true), (3, skins[2], 1., false)] {
        let base = 3 * station;
        let c = model.vertices[base].point;
        let x = model.vertices[base + 1].point;
        let y = model.vertices[base + 2].point;
        let xy = (0..3).map(|k| x[k] + y[k] - c[k]).collect::<Vec<_>>();
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![vec![c.to_vec(), y.to_vec()], vec![x.to_vec(), xy]],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let specs = [
            (
                [base, base + 1],
                Curve::from_polyline(vec![c.to_vec(), x.to_vec()])?,
                Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]])?,
            ),
            (
                [base + 1, base + 2],
                skin.iso(Axis::V, v)?,
                Curve {
                    degree: 2,
                    knots: vec![0., 0., 0., 1., 1., 1.],
                    control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
                    weights: vec![1., 0.5_f64.sqrt(), 1.],
                    periodic: false,
                },
            ),
            (
                [base + 2, base],
                Curve::from_polyline(vec![y.to_vec(), c.to_vec()])?,
                Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]])?,
            ),
        ];
        let mut ring = Vec::new();
        for (vertices, curve, pcurve) in specs {
            let id = edge(&mut model, &mut edges, vertices, curve);
            ring.push(Coedge::new(
                id,
                model.edges[id].vertices != vertices,
                pcurve,
            ));
        }
        let face = model.faces.len();
        let outer = model.loops.len();
        model.loops.push(Loop { coedges: ring });
        model.faces.push(Face {
            surface,
            outer,
            holes: vec![],
        });
        model.shells[0].faces.push(FaceUse { face, reversed });
    }
    let smooth_edges = [edges[&[4, 5]], edges[&[7, 8]]];
    model.rebuild_topology_ids();
    model.validate()?;
    let mut cells = transition.cells;
    if o.max_cells.saturating_sub(cells) < 3 {
        return Err(refused("No remaining body trim/closure work"));
    }
    let stage = (o.max_cells - cells) / 3;
    let trims =
        crate::face_domain::audit_trim_regions(&model, o.tolerances[0], 10000, stage, stage)?;
    cells += trims.cells + trims.domain_cells;
    if !trims.all_valid || cells >= o.max_cells {
        return Err(refused("Transition body trim regions are unproven"));
    }
    let closure = crate::watertight::inspect_with_exact(&model, o.max_cells - cells, 1000000)?;
    cells += closure.closure.agreement.cells;
    if closure.closure.status != crate::watertight::Closure::WithinTolerance {
        return Err(refused("Transition body closure is unproven"));
    }
    let exact_work = closure.exact.work;
    Ok(Report {
        model,
        transition,
        smooth_edges,
        trims,
        closure,
        cells,
        exact_work,
    })
}
