use super::*;
use crate::ss_intersection::{
    FiberSources, OriginalGraphSSStation, QualifiedSurfaceIntersection as Q,
    construct_qualified_ss_branches,
};

fn station(value: OriginalGraphSSStation) -> Value {
    json!({
        "firstUv":value.first_uv,
        "secondUv":value.second_uv,
        "worldMm":value.world,
    })
}

pub(super) fn inspect(v: &Value) -> Result<Value> {
    let first = field(v, "surface")?;
    let second = field(v, "target")?;
    let source = FiberSources::new(&first, &second)?;
    let qualified = construct_qualified_ss_branches(&source, Default::default())?;
    let mut output = match &qualified {
        Q::ClosedOrDegenerateGraph(graph) => {
            let vertices = graph.vertices().iter().enumerate().map(|(index, vertex)| {
                let mut value = station(graph.vertex_station(index, 4096)?);
                value["singular"] = json!(vertex.singular);
                Ok(value)
            }).collect::<Result<Vec<_>>>()?;
            let edges = graph.edges().iter().enumerate().map(|(index, edge)| {
                Ok(json!({"vertices":edge.vertices,"multiplicity":edge.multiplicity,
                    "midpoint":station(graph.edge_station(index, 0.5, 4096)?)}))
            }).collect::<Result<Vec<_>>>()?;
            let branches: Vec<_> = graph.branches().iter().map(|branch| {
                json!({"edges":branch.edges,"closed":branch.closed})
            }).collect();
            let trims: Vec<_> = graph.trims().iter().map(|trim| {
                json!({"edge":trim.edge,"surface":trim.surface})
            }).collect();
            json!({"kind":format!("{:?}",graph.kind()),"vertices":vertices,
                "edges":edges,"branches":branches,"trims":trims})
        }
        Q::PeriodicPlaneSection(section) => {
            let poles: Vec<_> = section.level_control_points().iter()
                .map(|point| point.clone().map(|q| q.to_string())).collect();
            let weights: Vec<_> = section.level_weights().iter().map(ToString::to_string).collect();
            json!({"kind":format!("{:?}",section.kind()),"periodicSurface":section.periodic_surface(),
                "periodicAxis":section.periodic_axis(),"levelParameter":section.transverse_parameter().map(ToString::to_string),
                "levelControlPoints":poles,"levelWeights":weights,
                "collapsedPoint":section.pole_point().map(|p|p.clone().map(|q|q.to_string()))})
        }
        Q::RegularOpenArcs(owner) => {
            let branches: Vec<_> = owner.branches().iter().map(|branch| {
                json!({"component":branch.component,"charts":branch.charts,"closed":false,
                    "domain":branch.domain,"freeAxis":format!("{:?}",branch.free_axis),"vertices":branch.vertices})
            }).collect();
            let trims: Vec<_> = owner.trims().iter().map(|trim| {
                json!({"branch":trim.branch,"surface":trim.surface})
            }).collect();
            json!({"kind":"RegularOpenArcs","branches":branches,"trims":trims})
        }
        Q::RegularClosedLoops(owner) => {
            let branches: Vec<_> = owner.branches().iter().map(|branch| {
                json!({"component":branch.component,"charts":branch.charts,"closed":true})
            }).collect();
            let trims: Vec<_> = owner.trims().iter().map(|trim| {
                json!({"branch":trim.branch,"surface":trim.surface})
            }).collect();
            json!({"kind":"RegularClosedLoops","branches":branches,"trims":trims})
        }
        Q::IsolatedGraphContact(contact) => json!({
            "kind":"IsolatedGraphContact","parameters":contact.native_parameters().map(|q|q.to_string()),
        }),
        Q::ConvexGraphContact(contact) => json!({
            "kind":"ConvexGraphContact","parameters":contact.native_parameters().map(|q|q.to_string()),
        }),
    };
    output["coversEntireSurfaces"] = json!(qualified.covers_entire_surfaces());
    Ok(output)
}
