//! Saved ownership maps are expectations only. No proof flags are imported.
use super::*;
pub(super) fn value(step: &Step) -> Value {
    match &step.applied {
        Applied::Split(s) => wire::maps(s),
        Applied::Reclock(s) => value_codec::json!({
            "edge":s.edge(),"source":s.source().edges[s.edge()].curve.domain(),"output":s.domain(),
            "relations":s.relations().iter().map(|r|value_codec::json!({"face":r.face,"wire":r.wire,"coedge":r.coedge,"original":wire::trim(r.original),"transferred":wire::trim(r.transferred),"curve":r.curve_parameters.iter().map(ToString::to_string).collect::<Vec<_>>()})).collect::<Vec<_>>()
        }),
        Applied::Relocate(s) => value_codec::json!({
            "cycle":s.traversal().iter().map(|c|value_codec::json!({"edge":c.edge,"reversed":c.reversed})).collect::<Vec<_>>(),
            "vertex":s.seam_vertex(),"relations":wire::authored(s.relations())
        }),
        Applied::Compact(s) => value_codec::json!({
            "cycle":s.source_cycle().iter().map(|c|value_codec::json!({"edge":c.edge,"reversed":c.reversed})).collect::<Vec<_>>(),
            "edge":s.edge(),"vertex":s.seam_vertex(),
            "segments":s.segments().iter().map(|p|value_codec::json!({"source":p.source_parameters,"output":p.output_parameters})).collect::<Vec<_>>(),
            "relations":s.relations().iter().map(|r|value_codec::json!({
                "face":r.face,"wire":r.wire,"coedge":r.coedge,"sourceEdge":r.source_edge,
                "original":wire::trim(r.original),"transferred":wire::trim(r.transferred),
                "curve":r.curve_parameters.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "pcurve":r.pcurve_parameters.iter().map(ToString::to_string).collect::<Vec<_>>()
            })).collect::<Vec<_>>()
        }),
    }
}
