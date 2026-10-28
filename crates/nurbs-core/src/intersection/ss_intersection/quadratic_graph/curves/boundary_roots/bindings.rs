//! Source chart inversion and exact shared-vertex incidence, not nearest edges.
use super::*;
mod vertices;
#[derive(Clone, Debug, PartialEq)]
pub struct GraphRootEdgeClock {
    pub original_edge: usize,
    pub original_branch: usize,
    pub clock: [f64; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub enum GraphRootLocation {
    Interior(GraphRootEdgeClock),
    Vertex {
        original_vertex: usize,
        /// Every incident edge port is retained, including a closed-loop seam.
        incidences: Vec<GraphRootEdgeClock>,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct GraphRootBinding {
    pub original_root: usize,
    pub location: GraphRootLocation,
}
#[derive(Clone, Debug)]
pub struct QualifiedGraphBoundaryBindings {
    census: QualifiedGraphBoundaryRoots,
    max_operations: usize,
    bindings: Vec<GraphRootBinding>,
    operations: usize,
}
impl QualifiedGraphBoundaryBindings {
    pub fn census(&self) -> &QualifiedGraphBoundaryRoots {
        &self.census
    }
    pub fn bindings(&self) -> &[GraphRootBinding] {
        &self.bindings
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = bind_graph_boundary_roots(&self.census, self.max_operations)?;
        check(
            fresh.bindings == self.bindings && fresh.operations == self.operations,
            "Original boundary root branch bindings changed",
        )
    }
}
fn signed(a: I, c: I, b: &mut Arithmetic) -> Result<I> {
    b.tick(1)?;
    a.div_signed(c)
}
fn edge_branch(original: &QualifiedQuadraticGraphSS, edge: usize) -> Result<usize> {
    let owners: Vec<_> = original
        .branches()
        .iter()
        .enumerate()
        .filter(|(_, v)| v.edges.contains(&edge))
        .map(|(i, _)| i)
        .collect();
    check(owners.len() == 1, "Original SS edge needs one branch owner")?;
    Ok(owners[0])
}
fn diagonal(
    original: &QualifiedQuadraticGraphSS,
    uv: [I; 2],
    b: &mut Arithmetic,
) -> Result<[I; 2]> {
    let a = original.data.basis;
    let d0 = b.sub(uv[0], I::point(original.data.center[0]))?;
    let d1 = b.sub(uv[1], I::point(original.data.center[1]))?;
    let left = b.mul(I::point(a[0][0]), I::point(a[1][1]))?;
    let right = b.mul(I::point(a[0][1]), I::point(a[1][0]))?;
    let det = b.sub(left, right)?;
    let left = b.mul(d0, I::point(a[1][1]))?;
    let right = b.mul(d1, I::point(a[0][1]))?;
    let numerator = b.sub(left, right)?;
    let x = signed(numerator, det, b)?;
    let left = b.mul(d1, I::point(a[0][0]))?;
    let right = b.mul(d0, I::point(a[1][0]))?;
    let numerator = b.sub(left, right)?;
    let y = signed(numerator, det, b)?;
    Ok([x, y])
}
fn ellipse(
    original: &QualifiedQuadraticGraphSS,
    root: &GraphBoundaryRoot,
    b: &mut Arithmetic,
) -> Result<GraphRootLocation> {
    let uv = [
        I::new(root.normalized_uv[0][0], root.normalized_uv[0][1])?,
        I::new(root.normalized_uv[1][0], root.normalized_uv[1][1])?,
    ];
    let mut unit = diagonal(original, uv, b)?;
    for a in 0..2 {
        let r2 = b.div(
            I::point(-original.data.diagonal[2]),
            I::point(original.data.diagonal[a]),
        )?;
        let radius = b.sqrt(r2)?;
        unit[a] = b.div(unit[a], radius)?;
    }
    let neg = |i: I| I {
        lo: -i.hi,
        hi: -i.lo,
    };
    let (quarter, c, s) = if unit[0].lo > 0. && unit[1].lo > 0. {
        (0, unit[0], unit[1])
    } else if unit[0].hi < 0. && unit[1].lo > 0. {
        (1, unit[1], neg(unit[0]))
    } else if unit[0].hi < 0. && unit[1].hi < 0. {
        (2, neg(unit[0]), neg(unit[1]))
    } else if unit[0].lo > 0. && unit[1].hi < 0. {
        (3, neg(unit[1]), unit[0])
    } else {
        return Err(crate::numeric_err(
            "Original root quadrant or exact shared vertex remains unproven",
        ));
    };
    // In each exact original quarter, t = sin/(1+cos). The complete original
    // conic identity and strict quadrant signs prove unique chart membership.
    let den = b.add(I::point(1.), c)?;
    let t = b.div(s, den)?.intersect(0., 1.)?;
    check(
        t.lo > 0. && t.hi < 1.,
        "Original root interior edge clock remains unresolved",
    )?;
    let edges: Vec<_> = original
        .edges()
        .iter()
        .enumerate()
        .filter(|(_, e)| e.geometry == OriginalGraphSSEdgeGeometry::EllipseQuarter(quarter))
        .map(|(i, _)| i)
        .collect();
    check(
        edges.len() == 1,
        "Original conic quarter edge is not uniquely owned",
    )?;
    let edge = edges[0];
    Ok(GraphRootLocation::Interior(GraphRootEdgeClock {
        original_edge: edge,
        original_branch: edge_branch(original, edge)?,
        clock: [t.lo, t.hi],
    }))
}
fn segments(
    original: &QualifiedQuadraticGraphSS,
    root: &GraphBoundaryRoot,
    b: &mut Arithmetic,
) -> Result<GraphRootLocation> {
    let uv = [
        I::new(root.normalized_uv[0][0], root.normalized_uv[0][1])?,
        I::new(root.normalized_uv[1][0], root.normalized_uv[1][1])?,
    ];
    let mut candidates = Vec::new();
    for (edge, e) in original.edges().iter().enumerate() {
        b.tick(1)?;
        let OriginalGraphSSEdgeGeometry::Segment(p) = e.geometry else {
            return Err(crate::input("Expected complete original segment graph"));
        };
        let d = [
            b.sub(I::point(p[1][0]), I::point(p[0][0]))?,
            b.sub(I::point(p[1][1]), I::point(p[0][1]))?,
        ];
        let q = [
            b.sub(uv[0], I::point(p[0][0]))?,
            b.sub(uv[1], I::point(p[0][1]))?,
        ];
        let left = b.mul(q[0], d[1])?;
        let right = b.mul(q[1], d[0])?;
        let residual = b.sub(left, right)?;
        if residual.lo > 0. || residual.hi < 0. {
            continue;
        }
        let axis = if d[0].lo > 0. || d[0].hi < 0. { 0 } else { 1 };
        let t = signed(q[axis], d[axis], b)?;
        if t.hi < 0. || t.lo > 1. {
            continue;
        }
        candidates.push((edge, t));
    }
    // Every original root belongs to the complete owned factor graph. Excluding
    // all other exact segment carriers therefore identifies this one, without
    // claiming that an overlapping UV box by itself proves line membership.
    check(
        candidates.len() == 1,
        "Original factor edge membership is ambiguous",
    )?;
    let (edge, t) = candidates[0];
    check(
        t.lo > 0. && t.hi < 1.,
        "Original factor endpoint incidence remains unproven",
    )?;
    Ok(GraphRootLocation::Interior(GraphRootEdgeClock {
        original_edge: edge,
        original_branch: edge_branch(original, edge)?,
        clock: [t.lo, t.hi],
    }))
}
/// Bind every original query root to an exact source vertex or a unique original
/// edge clock. No root is merged, even if different boundary events map to one
/// source vertex. Binding does not order overlapping cuts or authorize topology.
pub fn bind_graph_boundary_roots(
    census: &QualifiedGraphBoundaryRoots,
    max_operations: usize,
) -> Result<QualifiedGraphBoundaryBindings> {
    check(
        (1..=1000000).contains(&max_operations),
        "Invalid original root binding work allowance",
    )?;
    census.reverify()?;
    let mut b = Arithmetic {
        used: 0,
        max: max_operations,
    };
    b.tick(1)?;
    let original = census.original();
    let mut bindings = Vec::new();
    for (i, root) in census.roots().iter().enumerate() {
        b.tick(1)?;
        let location = if let Some(v) = vertices::bind(census, i, &mut b)? {
            v
        } else if original.kind() == QuadraticGraphSSKind::ClosedEllipse {
            ellipse(original, root, &mut b)?
        } else {
            segments(original, root, &mut b)?
        };
        bindings.push(GraphRootBinding {
            original_root: i,
            location,
        });
    }
    Ok(QualifiedGraphBoundaryBindings {
        census: census.clone(),
        max_operations,
        bindings,
        operations: b.used,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changed_edge_clocks_normalized_root_reports_and_pruned_vertex_ports_cannot_replay() {
        let a = crate::polynomial::graph(
            [0., 1., 0., 1.],
            &[vec![0.4375, -1., 1.], vec![-1., 0., 0.], vec![1., 0., 0.]],
        )
        .unwrap();
        let z = crate::polynomial::graph([0., 1., 0., 1.], &[vec![0., 0.], vec![0., 0.]]).unwrap();
        let sources = FiberSources::new(&a, &z).unwrap();
        let original = qualify_quadratic_graph_ss(&sources, Default::default()).unwrap();
        let query = |y| Curve::from_polyline(vec![vec![0., y], vec![1., y]]).unwrap();
        let roots =
            qualify_graph_boundary_roots(&original, &query(0.5625), 0, Default::default()).unwrap();
        let mut owner = bind_graph_boundary_roots(&roots, 1000000).unwrap();
        let GraphRootLocation::Interior(ref mut e) = owner.bindings[0].location else {
            panic!()
        };
        e.clock = [0., 1.];
        assert!(owner.reverify().is_err());
        let mut forged = roots;
        forged.roots[0].normalized_uv = [[0., 1.]; 2];
        assert!(bind_graph_boundary_roots(&forged, 1000000).is_err());
        let roots =
            qualify_graph_boundary_roots(&original, &query(0.5), 0, Default::default()).unwrap();
        let mut owner = bind_graph_boundary_roots(&roots, 1000000).unwrap();
        let GraphRootLocation::Vertex {
            ref mut incidences, ..
        } = owner.bindings[0].location
        else {
            panic!()
        };
        incidences.pop();
        assert!(owner.reverify().is_err());
    }
}
