use super::*;

mod assemblies;
mod control;
mod emitter;
mod mechanisms;
mod primitives;
mod sketches;
mod solids;
mod transforms;

/// Per-node traversal context shared by the `emit_*` handlers.
#[derive(Clone, Copy)]
struct Ctx<'a, 'c> {
    item: &'a Value,
    key: &'c str,
    op: &'c str,
    nodes: &'c Rc<Nodes<'a>>,
    scope: &'c Scope<'a>,
    current: &'c String,
    depth: usize,
    expected: GeometryType,
    parent: Option<&'c Parent>,
    allow_assembly: bool,
}
