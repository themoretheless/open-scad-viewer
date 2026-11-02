use crate::{
    Entity, EntityId, Error, Geometry, Result,
    document::{State, next_key},
};
use modeling_clean::{
    boolean::{self, Boolean},
    curve::Curve,
    math::{Affine, Bounds, Point},
    mesh::Mesh,
    surface::Surface,
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug)]
pub enum Operation {
    AddCurve(Curve),
    AddSurface(Surface),
    AddMesh(Mesh),
    Box {
        bounds: Bounds,
    },
    Cylinder {
        radius: f64,
        height: f64,
        segments: usize,
    },
    Sphere {
        radius: f64,
        latitude: usize,
        longitude: usize,
    },
    Torus {
        major: f64,
        minor: f64,
        segments: [usize; 2],
    },
    ExtrudePolygon {
        profile: Vec<[f64; 2]>,
        height: f64,
    },
    ExtrudeSurface {
        profile: EntityId,
        vector: Point,
    },
    RevolveSurface {
        profile: EntityId,
        origin: Point,
        axis: Point,
        angle: f64,
    },
    LoftSurface {
        profiles: Vec<EntityId>,
    },
    SweepSurface {
        profile: EntityId,
        path: Vec<Point>,
        up: Point,
        twist: f64,
    },
    BoxBoolean {
        left: EntityId,
        right: EntityId,
        kind: Boolean,
    },
    ShellBox {
        bounds: Bounds,
        thickness: f64,
        opening: Option<(usize, bool)>,
    },
    Transform {
        target: EntityId,
        transform: Affine,
    },
    InsertKnot {
        target: EntityId,
        parameter: f64,
    },
    TrimCurve {
        target: EntityId,
        interval: [f64; 2],
    },
    SplitCurve {
        target: EntityId,
        parameter: f64,
    },
    ImportObj(String),
    Delete(EntityId),
}
fn get(entities: &BTreeMap<EntityId, Entity>, id: EntityId) -> Result<&Geometry> {
    Ok(entities
        .get(&id)
        .ok_or(Error::MissingEntity(id))?
        .geometry())
}
fn curve(entities: &BTreeMap<EntityId, Entity>, id: EntityId) -> Result<&Curve> {
    match get(entities, id)? {
        Geometry::Curve(c) => Ok(c),
        _ => Err(Error::WrongGeometry {
            id,
            expected: "curve",
        }),
    }
}
fn boxes(entities: &BTreeMap<EntityId, Entity>, id: EntityId) -> Result<&[Bounds]> {
    match get(entities, id)? {
        Geometry::BoxCells(c) => Ok(c),
        _ => Err(Error::WrongGeometry {
            id,
            expected: "axis-aligned box cells",
        }),
    }
}

pub(crate) fn execute(
    state: &mut State,
    document: u64,
    operation: Operation,
) -> Result<Vec<EntityId>> {
    // Compute before publishing to the staged map. This also keeps direct job
    // execution atomic even if geometry evaluation returns an error.
    let mut replace = None;
    let geometry = match operation {
        Operation::AddCurve(c) => vec![Geometry::Curve(c)],
        Operation::AddSurface(s) => vec![Geometry::Surface(s)],
        Operation::AddMesh(m) => vec![Geometry::Mesh(m)],
        Operation::Box { bounds } => {
            Bounds::new(bounds.min, bounds.max)?;
            vec![Geometry::BoxCells(vec![bounds])]
        }
        Operation::Cylinder {
            radius,
            height,
            segments,
        } => vec![Geometry::Mesh(Mesh::cylinder(radius, height, segments)?)],
        Operation::Sphere {
            radius,
            latitude,
            longitude,
        } => vec![Geometry::Mesh(Mesh::sphere(radius, latitude, longitude)?)],
        Operation::Torus {
            major,
            minor,
            segments,
        } => vec![Geometry::Mesh(Mesh::torus(major, minor, segments)?)],
        Operation::ExtrudePolygon { profile, height } => {
            vec![Geometry::Mesh(Mesh::extrude(&profile, height)?)]
        }
        Operation::ExtrudeSurface { profile, vector } => vec![Geometry::Surface(Surface::extrude(
            curve(&state.entities, profile)?,
            vector,
        )?)],
        Operation::RevolveSurface {
            profile,
            origin,
            axis,
            angle,
        } => vec![Geometry::Surface(Surface::revolve(
            curve(&state.entities, profile)?,
            origin,
            axis,
            angle,
        )?)],
        Operation::LoftSurface { profiles } => {
            let c = profiles
                .iter()
                .map(|id| curve(&state.entities, *id).cloned())
                .collect::<Result<Vec<_>>>()?;
            vec![Geometry::Surface(Surface::loft(&c)?)]
        }
        Operation::SweepSurface {
            profile,
            path,
            up,
            twist,
        } => vec![Geometry::Surface(Surface::sweep(
            curve(&state.entities, profile)?,
            &path,
            up,
            twist,
        )?)],
        Operation::BoxBoolean { left, right, kind } => {
            let cells = boolean::box_cells(
                boxes(&state.entities, left)?,
                boxes(&state.entities, right)?,
                kind,
            )?;
            // Result must be displayable as a manifold before authoring it.
            boolean::mesh_cells(&cells)?;
            vec![Geometry::BoxCells(cells)]
        }
        Operation::ShellBox {
            bounds,
            thickness,
            opening,
        } => {
            let cells = boolean::shell_box(bounds, thickness, opening)?;
            boolean::mesh_cells(&cells)?;
            vec![Geometry::BoxCells(cells)]
        }
        Operation::Transform { target, transform } => {
            replace = Some(target);
            let shape = match get(&state.entities, target)? {
                Geometry::Curve(c) => Geometry::Curve(c.transformed(&transform)?),
                Geometry::Surface(s) => Geometry::Surface(s.transformed(&transform)?),
                Geometry::Mesh(m) => Geometry::Mesh(m.transformed(&transform)?),
                // Arbitrary affine placement intentionally relinquishes the
                // axis-aligned box CSG representation. No bounds substitution.
                Geometry::BoxCells(c) => {
                    Geometry::Mesh(boolean::mesh_cells(c)?.transformed(&transform)?)
                }
            };
            vec![shape]
        }
        Operation::InsertKnot { target, parameter } => {
            replace = Some(target);
            vec![Geometry::Curve(
                curve(&state.entities, target)?.insert_knot(parameter)?,
            )]
        }
        Operation::TrimCurve {
            target,
            interval: [a, b],
        } => {
            replace = Some(target);
            vec![Geometry::Curve(curve(&state.entities, target)?.trim(a, b)?)]
        }
        Operation::SplitCurve { target, parameter } => {
            let pair = curve(&state.entities, target)?.split(parameter)?;
            pair.into_iter().map(Geometry::Curve).collect()
        }
        Operation::ImportObj(text) => vec![Geometry::Mesh(Mesh::from_obj(&text)?)],
        Operation::Delete(id) => {
            if state.entities.remove(&id).is_none() {
                return Err(Error::MissingEntity(id));
            }
            return Ok(vec![]);
        }
    };
    if state.entities.len() + if replace.is_some() { 0 } else { geometry.len() } > 10000 {
        return Err(Error::Limit("document entity budget"));
    }
    let mut additions = Vec::new();
    for shape in geometry {
        let id = if let Some(id) = replace {
            id
        } else {
            EntityId {
                document,
                serial: next_key()?,
            }
        };
        additions.push((
            id,
            Entity {
                id,
                content: next_key()?,
                geometry: Arc::new(shape),
            },
        ));
    }
    let ids = additions.iter().map(|(id, _)| *id).collect();
    state.entities.extend(additions);
    Ok(ids)
}
