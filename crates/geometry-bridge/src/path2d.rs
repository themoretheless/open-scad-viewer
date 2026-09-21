//! Stable path2d transport facade. Domain handlers borrow the request; codecs
//! own wire defaults and validation, while the geometry crates own algorithms.
use crate::{Result, input};
use value_codec::Value;
mod appearance;
mod codec;
mod editing;
mod effects;
mod paths;

pub fn dispatch(v: Value) -> Result<Value> {
    let action = v["action"].as_str().unwrap_or("");
    for handler in [
        paths::dispatch,
        effects::dispatch,
        editing::dispatch,
        appearance::dispatch,
    ] {
        if let Some(output) = handler(action, &v)? {
            return Ok(output);
        }
    }
    Err(input(format!("Unknown path2d action '{action}'")))
}

#[cfg(test)]
use crate::field;
#[cfg(test)]
use codec::{decode_path, decode_paths, encode_path};
#[cfg(test)]
use planar_geometry::path::BezierPath;
#[cfg(test)]
use planar_geometry::path::PathSegment;
#[cfg(test)]
use value_codec::json;

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(min: [f64; 2], max: [f64; 2]) -> Value {
        encode_path(&BezierPath::from_rect(min, max).unwrap())
    }

    fn mesh_area(mesh: &Value) -> f64 {
        let points: Vec<[f64; 2]> = field(mesh, "positions").unwrap();
        let indices: Vec<usize> = field(mesh, "indices").unwrap();
        indices
            .as_chunks::<3>().0.iter()
            .map(|t| {
                let [a, b, c] = [points[t[0]], points[t[1]], points[t[2]]];
                ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() * 0.5
            })
            .sum()
    }

    #[test]
    fn dispatches_ridge_smoothing_and_keeps_legacy_repeat_defaults() {
        let line = encode_path(&BezierPath::from_polyline(&[[0., 0.], [8., 0.]], false).unwrap());
        let wave = decode_path(
            &dispatch(
                json!({"action":"zig_zag","path":line,"amplitude":1.,"ridges":3,"smooth":true}),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            wave.anchors(),
            vec![[0., 0.], [2., 1.], [4., -1.], [6., 1.], [8., 0.]]
        );
        assert!(
            wave.segments
                .iter()
                .all(|s| matches!(s, PathSegment::Cubic { .. }))
        );
        let path = rect([0., 0.], [2., 1.]);
        let legacy =
            dispatch(json!({"action":"step_and_repeat","path":path,"count":2,"delta":[10.,0.]}))
                .unwrap();
        let copies = dispatch(
            json!({"action":"step_and_repeat_paths","paths":[path],"count":2,"delta":[10.,0.]}),
        )
        .unwrap();
        assert_eq!(decode_paths(&legacy).unwrap()[0].start, [0., 0.]);
        assert_eq!(decode_paths(&copies).unwrap()[0].start, [10., 0.]);
    }

    #[test]
    fn dispatches_single_ring_stipple_jitter_and_seed_without_dropping_options() {
        let ring = json!([[0., 0.], [12., 0.], [12., 12.], [0., 12.]]);
        let single = dispatch(json!({"action":"stipple","ring":ring,"spacing":4.,"size":0.2,"jitter":0.8,"seed":42,"fillRule":"evenodd"})).unwrap();
        let multi = dispatch(json!({"action":"stipple","rings":[ring],"spacing":4.,"size":0.2,"jitter":0.8,"seed":42,"fillRule":"evenodd"})).unwrap();
        assert_eq!(single, multi);
        let other_seed = dispatch(json!({"action":"stipple","ring":ring,"spacing":4.,"size":0.2,"jitter":0.8,"seed":43,"fillRule":"evenodd"})).unwrap();
        assert_ne!(single, other_seed);
        assert!(!single.as_array().unwrap().is_empty());
    }

    #[test]
    fn dispatches_fill_rule_and_holes_for_mesh_offset_and_gradient() {
        let outer = rect([0., 0.], [10., 10.]);
        let hole = rect([4., 4.], [6., 6.]);
        let evenodd = dispatch(
            json!({"action":"tessellate","path":outer,"holes":[hole],"fillRule":"evenodd"}),
        )
        .unwrap();
        let nonzero = dispatch(
            json!({"action":"tessellate","path":outer,"holes":[hole],"fillRule":"nonzero"}),
        )
        .unwrap();
        assert!((mesh_area(&evenodd) - 96.).abs() < 1e-8);
        assert!((mesh_area(&nonzero) - 100.).abs() < 1e-8);
        let offset = dispatch(json!({"action":"offset","path":outer,"holes":[hole],"distance":0.5,"fillRule":"evenodd","miterLimit":2.,"tolerance":0.01})).unwrap();
        let offset_area: f64 = decode_paths(&offset)
            .unwrap()
            .iter()
            .map(|p| planar_geometry::rings::area(&p.to_ring(0.01).unwrap()))
            .sum();
        assert!((offset_area - 120.).abs() < 1e-8);
        let gradient = dispatch(json!({"action":"gradient_fill_mesh","path":outer,"holes":[hole],"fillRule":"evenodd","kind":"linear","p1":[0.,0.],"p2":[10.,0.],"stops":[{"offset":0.,"color":[255,0,0,255]},{"offset":1.,"color":[0,0,255,255]}]})).unwrap();
        assert!((mesh_area(&gradient) - 96.).abs() < 1e-8);
        assert_eq!(
            gradient["positions"].as_array().unwrap().len(),
            gradient["colors"].as_array().unwrap().len()
        );
    }

    #[test]
    fn dispatches_miter_limit_without_requiring_region_flag_and_preserves_concentric_nesting() {
        let triangle = encode_path(
            &BezierPath::from_polyline(&[[0., 0.], [10., 0.], [0.2, 0.2]], true).unwrap(),
        );
        let low =
            dispatch(json!({"action":"offset","path":triangle,"distance":1.,"miterLimit":1.}))
                .unwrap();
        let high =
            dispatch(json!({"action":"offset","path":triangle,"distance":1.,"miterLimit":100.}))
                .unwrap();
        assert_ne!(low, high);
        let result = dispatch(json!({"action":"concentric_offset","rings":[[[0.,0.],[20.,0.],[20.,20.],[0.,20.]],[[6.,6.],[14.,6.],[14.,14.],[6.,14.]]],"count":2,"step":1.,"fillRule":"evenodd"})).unwrap();
        let levels = result.as_array().unwrap();
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0].as_array().unwrap().len(), 2);
        assert_eq!(levels[1].as_array().unwrap().len(), 2);
    }

    #[test]
    fn node_editing_and_missing_mesh_tools_are_reachable() {
        let line = encode_path(&BezierPath::from_polyline(&[[0., 0.], [10., 0.]], false).unwrap());
        let subdivided = dispatch(json!({"action":"subdivide","path":line,"levels":2})).unwrap();
        assert_eq!(decode_path(&subdivided).unwrap().segments.len(), 4);
        let split =
            dispatch(json!({"action":"split_at_anchor","path":subdivided,"node":2})).unwrap();
        let paths = decode_paths(&split).unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].segments.last().unwrap().end(), paths[1].start);
        let dashes =
            dispatch(json!({"action":"dash_spans","path":line,"dash":[2.,2.],"tolerance":0.01}))
                .unwrap();
        assert_eq!(dashes.as_array().unwrap().len(), 3);
        let knife = dispatch(json!({"action":"knife_split","path":rect([0.,0.],[10.,10.]),"a":[5.,-1.],"b":[5.,11.]})).unwrap();
        assert_eq!(decode_paths(&knife).unwrap().len(), 2);
        let stroke = dispatch(json!({"action":"gradient_stroke_mesh","path":line,"width":1.,"cap":"square","kind":"linear","p1":[0.,0.],"p2":[10.,0.],"stops":[{"offset":0.,"color":[255,0,0,255]},{"offset":1.,"color":[0,0,255,255]}]})).unwrap();
        assert!((mesh_area(&stroke) - 11.).abs() < 1e-7);
    }

    #[test]
    fn rejects_invalid_holes_and_dash_values_instead_of_discarding_them() {
        let path = rect([0., 0.], [10., 10.]);
        assert!(dispatch(json!({"action":"hatch","path":path,"holes":42,"spacing":1.})).is_err());
        assert!(dispatch(json!({"action":"outline_stroke_styled","path":path,"width":1.,"dash":[2.,"bad",2.]})).is_err());
        assert!(dispatch(json!({"action":"tessellate","path":path,"fillRule":"bad"})).is_err());
    }

    #[test]
    fn semantic_editor_options_survive_transport() {
        let hit = dispatch(
            json!({"action":"snap_geometry","cursor":[4.8,0.],"threshold":0.3,"geometry":[
                {"type":"line","start":[0.,0.],"end":[10.,0.]},
                {"type":"line","start":[4.8,0.],"end":[4.8,10.]}
            ]}),
        )
        .unwrap();
        assert_eq!(hit["kind"].as_str(), Some("Midpoint"));
        assert_eq!(field::<[f64; 2]>(&hit, "point").unwrap(), [5., 0.]);
        let rounded = dispatch(json!({"action":"rounded_open_polyline","points":[[0.,0.],[10.,0.],[10.,10.]],"radii":[0.,-2.,0.]})).unwrap();
        assert_eq!(
            decode_path(&rounded).unwrap().flatten().unwrap(),
            vec![[0., 0.], [8., 0.], [10., 2.], [10., 10.]]
        );
        let ray =
            dispatch(json!({"action":"snap_rays","start":[0.,0.],"cursor":[10.,0.1]})).unwrap();
        assert_eq!(field::<[f64; 2]>(&ray, "point").unwrap(), [10., 0.]);
        let head = encode_path(&BezierPath::from_polyline(&[[0., 0.], [2., 0.]], false).unwrap());
        let tail = encode_path(&BezierPath::from_polyline(&[[4., 2.], [4., 4.]], false).unwrap());
        let joined = dispatch(json!({"action":"join_tangents","head":head,"tail":tail})).unwrap();
        assert_eq!(decode_path(&joined).unwrap().segments.len(), 4);
        let snap = dispatch(json!({"action":"drag_snap","moving":{"min":[0.,0.],"max":[10.,10.]},"targets":[{"min":[1.,20.],"max":[11.,30.]}],"threshold":1.1})).unwrap();
        assert_eq!(field::<[f64; 2]>(&snap, "delta").unwrap(), [1., 0.]);
        let marks = dispatch(json!({"action":"distance_marks","moving":{"min":[0.,0.],"max":[10.,10.]},"targets":[{"min":[-3.,-4.],"max":[15.,16.]}]})).unwrap();
        assert_eq!(marks.as_array().unwrap().len(), 4);
    }

    #[test]
    fn compound_knife_and_scissors_transport_preserves_ring_grouping() {
        let outer = rect([0., 0.], [20., 20.]);
        let hole = rect([2., 2.], [8., 8.]);
        let paths = json!([outer, hole]);
        let hit = dispatch(
            json!({"action":"compound_hit_test","paths":paths,"click":[2.,5.],"maxDist":0.1}),
        )
        .unwrap();
        assert_eq!(hit["ring"].as_u64(), Some(1));
        let pieces = dispatch(json!({"action":"compound_cut_at","paths":paths,"hit":hit})).unwrap();
        let pieces = pieces.as_array().unwrap();
        assert_eq!(pieces.len(), 2);
        assert_eq!(
            decode_paths(&pieces[0]).unwrap()[0],
            decode_path(&outer).unwrap()
        );
        assert!(!decode_paths(&pieces[1]).unwrap()[0].closed);
        let pieces = dispatch(
            json!({"action":"compound_knife_split","paths":paths,"a":[1.,5.],"b":[9.,5.]}),
        )
        .unwrap();
        let pieces = pieces.as_array().unwrap();
        assert_eq!(pieces.len(), 1);
        assert_eq!(decode_paths(&pieces[0]).unwrap().len(), 3);
    }
}
