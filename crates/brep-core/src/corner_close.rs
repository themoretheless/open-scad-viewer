//! Close an owned four-edge opening with a continuously qualified G2 patch.
//! Affine isoparametric rim uses; this is not global embedding/volume proof.
use crate::{Body, Coedge, Face, FaceUse, Loop, Model, TopologyLineageRecord};
use nurbs_core::{Error, Result, corner_blend, surface_blend::Boundary};
#[derive(Clone, Copy)]
pub struct BoundaryUse {
    pub face: usize,
    pub edge: usize,
}
pub struct Closed {
    pub model: Model,
    pub corner: corner_blend::Report,
    pub closure: crate::watertight::QualifiedReport,
    pub face: usize,
    pub body: usize,
    pub cells: usize,
    pub exact_work: u64,
}
fn refused(s: &str) -> Error {
    Error::new("BREP_CORNER_CLOSE_UNPROVEN", s)
}
/// Source endpoints and edges survive unchanged. The four boundary uses must
/// lie in one open shell and form its sole remaining opening. A newly authored
/// face uses those actual owned 3D curves. Full saved-use verification and
/// shell closure are mandatory after construction; source is never mutated.
pub fn close(
    source: &Model,
    shell: usize,
    uses: [BoundaryUse; 4],
    options: corner_blend::Options,
    tolerance_uv: f64,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
) -> Result<Closed> {
    source.validate()?;
    if !source.persistent_naming_complete()
        || shell >= source.shells.len()
        || source.shells[shell].closed
        || !(1..=100000).contains(&max_cells)
        || max_exact > 1000000
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || tolerances[0] > source.tolerance_mm
        || source
            .bodies
            .iter()
            .any(|b| b.outer_shell == shell || b.inner_shells.contains(&shell))
    {
        return Err(refused(
            "Need a bounded owned open shell and unchanged position tolerance",
        ));
    }
    let Prepared {
        corner,
        rim,
        reversed,
        mut cells,
        mut exact_work,
    } = prepare(
        source, shell, uses, options, tolerances, max_cells, max_exact, 1,
    )?;
    let mut model = source.clone();
    let wire = model.loops.len();
    let face = model.faces.len();
    let body = model.bodies.len();
    model.loops.push(Loop { coedges: rim });
    model.faces.push(Face {
        surface: corner.surface.clone(),
        outer: wire,
        holes: vec![],
    });
    model.shells[shell].faces.push(FaceUse { face, reversed });
    model.shells[shell].closed = true;
    model.bodies.push(Body {
        outer_shell: shell,
        inner_shells: vec![],
    });
    model.rebuild_topology_ids();
    // Explicitly retain identities for every unchanged source entity.
    model.1.vertices[..source.vertices.len()].clone_from_slice(&source.1.vertices);
    model.1.edges[..source.edges.len()].clone_from_slice(&source.1.edges);
    model.1.loops[..source.loops.len()].clone_from_slice(&source.1.loops);
    model.1.faces[..source.faces.len()].clone_from_slice(&source.1.faces);
    for i in 0..source.shells.len() {
        if i != shell {
            model.1.shells[i] = source.1.shells[i];
        }
    }
    model.1.bodies[..source.bodies.len()].clone_from_slice(&source.1.bodies);
    model.1.lineage = source.1.lineage.clone();
    if source.1.shells[shell] != model.1.shells[shell] {
        model.1.lineage.push(TopologyLineageRecord {
            operation: "modify".into(),
            entity_kind: "shell".into(),
            parents: vec![source.1.shells[shell]],
            children: vec![model.1.shells[shell]],
        });
    }
    model.refresh_change_set(&[source]);
    model.validate()?;
    if max_cells - cells < 2 {
        return Err(refused("Corner trim audit work exhausted"));
    }
    let stage = (max_cells - cells) / 2;
    let trim = crate::face_domain::audit_trim_regions(&model, tolerance_uv, 10000, stage, stage)?;
    cells += trim.cells + trim.domain_cells;
    if !trim.all_valid {
        return Err(refused("Corner result trim regions are unproven"));
    }
    let closure =
        crate::watertight::inspect_with_exact(&model, max_cells - cells, max_exact - exact_work)?;
    cells += closure.closure.agreement.cells;
    exact_work += closure.exact.work;
    if closure.closure.status != crate::watertight::Closure::WithinTolerance {
        return Err(refused(
            "Corner insertion did not close all shell boundaries",
        ));
    }
    Ok(Closed {
        model,
        corner,
        closure,
        face,
        body,
        cells,
        exact_work,
    })
}

pub(crate) struct Prepared {
    pub corner: corner_blend::Report,
    pub rim: Vec<Coedge>,
    pub reversed: bool,
    pub cells: usize,
    pub exact_work: u64,
}
/// Fresh original-support construction shared by open-rim closure and closed
/// region replacement. Both callers must independently admit final topology.
pub(crate) fn prepare(
    source: &Model,
    shell: usize,
    uses: [BoundaryUse; 4],
    options: corner_blend::Options,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
    expected_uses: usize,
) -> Result<Prepared> {
    prepare_sides(
        source,
        shell,
        uses.map(|u| vec![u]),
        options,
        tolerances,
        max_cells,
        max_exact,
        expected_uses,
    )
}
pub(crate) fn prepare_sides(
    source: &Model,
    shell: usize,
    sides: [Vec<BoundaryUse>; 4],
    options: corner_blend::Options,
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
    expected_uses: usize,
) -> Result<Prepared> {
    source.validate()?;
    if !source.persistent_naming_complete()
        || shell >= source.shells.len()
        || !(1..=100000).contains(&max_cells)
        || max_exact > 1000000
        || !(1..=2).contains(&expected_uses)
        || tolerances[0] > source.tolerance_mm
        || sides.iter().any(|s| !(1..=64).contains(&s.len()))
        || sides.iter().map(Vec::len).sum::<usize>() > 256
    {
        return Err(refused(
            "Need four bounded owned rim-side chains and shared corner work",
        ));
    }
    let mut selected = std::collections::BTreeSet::new();
    let mut saved: Vec<(usize, BoundaryUse, Coedge)> = Vec::new();
    let mut traces = Vec::new();
    let mut orientation = None;
    for side in 0..4 {
        let face_id = sides[side][0].face;
        if face_id >= source.faces.len() || sides[side].iter().any(|u| u.face != face_id) {
            return Err(refused(
                "Each rim-side chain must belong to one original support chart",
            ));
        }
        let face = &source.faces[face_id];
        let owners = source.shells[shell]
            .faces
            .iter()
            .filter(|u| u.face == face_id)
            .collect::<Vec<_>>();
        if owners.len() != 1 {
            return Err(refused("Corner support needs unique shell ownership"));
        }
        let owner = owners[0];
        let surface = &face.surface;
        let domain = [
            [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
            ],
            [
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ],
        ];
        let fixed = usize::from(matches!(
            options.boundaries[side],
            Boundary::VStart | Boundary::VEnd
        ));
        let along = 1 - fixed;
        let mut at = None;
        let mut segments = Vec::new();
        for &use_ in &sides[side] {
            if use_.edge >= source.edges.len() || !selected.insert(use_.edge) {
                return Err(refused("Corner rim must select distinct original edges"));
            }
            let incident = source
                .faces
                .iter()
                .flat_map(|f| std::iter::once(f.outer).chain(f.holes.iter().copied()))
                .flat_map(|w| &source.loops[w].coedges)
                .filter(|c| c.edge == use_.edge)
                .count();
            if incident != expected_uses {
                return Err(refused("Corner rim edge has unexpected face incidence"));
            }
            let hits = source.loops[face.outer]
                .coedges
                .iter()
                .filter(|c| c.edge == use_.edge)
                .collect::<Vec<_>>();
            if hits.len() != 1 {
                return Err(refused(
                    "Rim edge needs one original outer-boundary occurrence",
                ));
            }
            let c = hits[0];
            let pc = &c.pcurve;
            if pc.degree != 1
                || pc.control_points.len() != 2
                || pc.weights[0] != pc.weights[1]
                || pc.periodic
            {
                return Err(refused(
                    "Owned corner G2 requires affine isoparametric rim segments",
                ));
            }
            let coordinate = pc.control_points[0][fixed];
            if at.is_some_and(|a| a != coordinate)
                || pc.control_points.iter().any(|p| {
                    p[fixed] != coordinate
                        || (0..2).any(|k| p[k] < domain[k][0] || p[k] > domain[k][1])
                })
            {
                return Err(refused(
                    "Rim-side segments must lie on one original isoparametric trace",
                ));
            }
            at = Some(coordinate);
            let d = pc.domain();
            if pc.knots[..=pc.degree].iter().any(|k| *k != d[0])
                || pc.knots[pc.control_points.len()..]
                    .iter()
                    .any(|k| *k != d[1])
            {
                return Err(refused("Corner rim needs clamped endpoint authority"));
            }
            let x = pc.control_points[0][along];
            let y = pc.control_points[1][along];
            if x == y {
                return Err(refused(
                    "Rim segment must have distinct original chart endpoints",
                ));
            }
            let vertices = source.edges[use_.edge].vertices;
            let start = vertices[usize::from(c.reversed)];
            let end = vertices[usize::from(!c.reversed)];
            let (lo, hi, lv, hv) = if x < y {
                (x, y, start, end)
            } else {
                (y, x, end, start)
            };
            segments.push((lo, hi, lv, hv, use_, c));
        }
        segments.sort_by(|a, b| a.0.total_cmp(&b.0));
        for pair in segments.windows(2) {
            if pair[0].1 != pair[1].0 || pair[0].3 != pair[1].2 {
                return Err(refused(
                    "Rim-side chain has a gap, overlap or mismatched owned vertex",
                ));
            }
        }
        let range = [segments[0].0, segments.last().unwrap().1];
        traces.push(corner_blend::Trace {
            at: at.unwrap(),
            along: range,
        });
        for (_, _, _, _, use_, c) in segments {
            let mut pc = c.pcurve.clone();
            for p in &mut pc.control_points {
                let mut t = if p[along] == range[0] {
                    0.
                } else if p[along] == range[1] {
                    1.
                } else {
                    (p[along] - range[0]) / (range[1] - range[0])
                };
                if options.reverse_along[side] {
                    t = 1. - t;
                }
                *p = if side < 2 {
                    vec![side as f64, t]
                } else {
                    vec![t, (side - 2) as f64]
                };
            }
            let axis = usize::from(side < 2);
            let first = pc.control_points[0][axis];
            let last = pc.control_points[1][axis];
            if !first.is_finite()
                || !last.is_finite()
                || first == last
                || first < 0.
                || first > 1.
                || last < 0.
                || last > 1.
            {
                return Err(refused(
                    "Rim segment parameter correspondence is unresolved",
                ));
            }
            let forward = side == 1 || side == 2;
            let reverse = (first < last) != forward;
            if reverse {
                pc = pc.reverse()?;
            }
            let reversed = c.reversed ^ reverse;
            let face_reversed = !(reversed ^ c.reversed ^ owner.reversed);
            if orientation.is_some_and(|r| r != face_reversed) {
                return Err(refused("Corner rim orientation is inconsistent"));
            }
            orientation = Some(face_reversed);
            saved.push((
                side,
                use_,
                Coedge::new(
                    use_.edge,
                    reversed,
                    pc,
                ),
            ));
        }
    }
    let supports = std::array::from_fn(|i| &source.faces[sides[i][0].face].surface);
    let corner = corner_blend::g2_traces(
        supports,
        traces.try_into().ok().unwrap(),
        options,
        tolerances,
        max_cells,
        max_exact,
    )?;
    if !corner.complete || !corner.within_tolerance || !corner.interior.certified {
        return Err(refused(
            "Original-support G2 seams or whole corner chart remain unproven",
        ));
    }
    let mut cells = corner.cells;
    let exact_work = corner.exact_work;
    if exact_work > max_exact {
        return Err(refused("Corner knot proof exceeds shared exact work"));
    }
    for (_, use_, c) in &saved {
        let original = &source.faces[use_.face];
        let old = source.loops[original.outer]
            .coedges
            .iter()
            .find(|c| c.edge == use_.edge)
            .unwrap();
        for (pc, surface, reversed) in [
            (&old.pcurve, &original.surface, old.reversed),
            (&c.pcurve, &corner.surface, c.reversed),
        ] {
            if cells == max_cells {
                return Err(refused("Owned rim agreement work exhausted"));
            }
            let r = nurbs_core::curve_surface_agreement::verify(
                &source.edges[c.edge].curve,
                pc,
                surface,
                reversed,
                source.tolerance_mm,
                max_cells - cells,
            )?;
            cells += r.cells;
            if r.status != nurbs_core::curve_surface_agreement::Status::WithinTolerance {
                return Err(refused(
                    "Actual saved rim does not qualify on both original support and new chart",
                ));
            }
        }
    }
    let mut rim = Vec::new();
    for side in [2, 1, 3, 0] {
        let axis = usize::from(side < 2);
        let forward = side == 1 || side == 2;
        let mut chain = saved
            .iter()
            .filter(|(s, _, _)| *s == side)
            .map(|(_, _, c)| c.clone())
            .collect::<Vec<_>>();
        chain.sort_by(|a, b| {
            let order =
                a.pcurve.control_points[0][axis].total_cmp(&b.pcurve.control_points[0][axis]);
            if forward { order } else { order.reverse() }
        });
        rim.extend(chain);
    }
    Ok(Prepared {
        corner,
        rim,
        reversed: orientation.unwrap(),
        cells,
        exact_work,
    })
}
