//! Editing operations at the JSON boundary. Algorithms live in the geometry crates.
use super::codec::{
    decode_boxes, decode_cut_hit, decode_path, decode_paths, decode_snap_geometry, encode_cut_hit,
    encode_path_pieces, encode_paths,
};
use crate::Result;
use crate::encode;
use crate::field;
use crate::input;
use planar_geometry::edit;
use planar_geometry::path::BezierPath;
use planar_geometry::scissors;
use value_codec::Value;
use value_codec::json;

pub(super) fn dispatch(action: &str, v: &Value) -> Result<Option<Value>> {
    let output = match action {
        "snap_geometry" => {
            let shapes = v["geometry"]
                .as_array()
                .ok_or_else(|| input("geometry must be an array"))?
                .iter()
                .map(decode_snap_geometry)
                .collect::<Result<Vec<_>>>()?;
            match edit::find_geometry_snap(
                field(v, "cursor")?,
                field(v, "threshold")?,
                v["grid"].as_f64(),
                &shapes,
            )? {
                Some(h) => {
                    json!({"point":h.point,"kind":format!("{:?}",h.kind),"distance":h.distance})
                }
                None => Value::Null,
            }
        }
        "drag_snap" => {
            let moving = edit::BBox {
                min: field(&v["moving"], "min")?,
                max: field(&v["moving"], "max")?,
            };
            let result = edit::find_drag_snap(
                moving,
                &decode_boxes(&v["targets"])?,
                field(v, "threshold")?,
            )?;
            let guides: Vec<_> = result
                .guides
                .iter()
                .map(|g| {
                    json!({
                        "vertical":g.vertical,"position":g.position,
                        "moving":{"min":g.moving.min,"max":g.moving.max},
                        "target":{"min":g.target.min,"max":g.target.max},
                    })
                })
                .collect();
            json!({"delta":result.delta,"guides":guides})
        }
        "distance_marks" => {
            let moving = edit::BBox {
                min: field(&v["moving"], "min")?,
                max: field(&v["moving"], "max")?,
            };
            Value::Array(
                edit::compute_distance_marks(moving, &decode_boxes(&v["targets"])?)
                    .iter()
                    .map(|m| json!({"from":m.from,"to":m.to,"distance":m.distance}))
                    .collect(),
            )
        }
        "intersecting_paths" => Value::Array(
            edit::intersecting_paths(&decode_paths(&v["paths"])?, field(v, "a")?, field(v, "b")?)?
                .into_iter()
                .map(|(angle, point)| json!({"angle":angle,"point":point}))
                .collect(),
        ),
        "intersecting_directions" => encode(edit::intersecting_path_directions(
            &decode_paths(&v["paths"])?,
            field(v, "a")?,
            field(v, "b")?,
        )?)?,
        "snap" => {
            let paths_v = v["paths"]
                .as_array()
                .ok_or_else(|| input("paths must be an array"))?;
            let paths: Vec<BezierPath> = paths_v.iter().map(decode_path).collect::<Result<_>>()?;
            match edit::find_snap(
                field(v, "cursor")?,
                field(v, "threshold")?,
                v["grid"].as_f64(),
                &paths,
            )? {
                Some(h) => {
                    json!({"point":h.point,"kind":format!("{:?}",h.kind),"distance":h.distance})
                }
                None => Value::Null,
            }
        }
        "align" => {
            let boxes = decode_boxes(&v["boxes"])?;
            let reference = edit::BBox {
                min: field(&v["reference"], "min")?,
                max: field(&v["reference"], "max")?,
            };
            let h = match v["h"].as_str() {
                Some("left") => Some(edit::HAlign::Left),
                Some("center") => Some(edit::HAlign::Center),
                Some("right") => Some(edit::HAlign::Right),
                _ => None,
            };
            let vert = match v["v"].as_str() {
                Some("top") => Some(edit::VAlign::Top),
                Some("center") => Some(edit::VAlign::Center),
                Some("bottom") => Some(edit::VAlign::Bottom),
                _ => None,
            };
            encode(edit::align_boxes(&boxes, reference, h, vert))?
        }
        "distribute" => encode(edit::distribute_centers(
            &decode_boxes(&v["boxes"])?,
            v["horizontal"].as_bool().unwrap_or(true),
        )?)?,
        "path_hit_test" => match scissors::hit_test(
            &decode_path(&v["path"])?,
            field(v, "click")?,
            field(v, "maxDist")?,
        )? {
            Some(hit) => encode_cut_hit(hit),
            None => Value::Null,
        },
        "path_cut_at" => encode_paths(&scissors::cut_at(
            &decode_path(&v["path"])?,
            decode_cut_hit(&v["hit"])?,
        )?),
        "path_cut_many" => {
            let hits = v["hits"]
                .as_array()
                .ok_or_else(|| input("hits must be an array"))?
                .iter()
                .map(decode_cut_hit)
                .collect::<Result<Vec<_>>>()?;
            encode_paths(&scissors::cut_at_many(&decode_path(&v["path"])?, &hits)?)
        }
        "compound_hit_test" => match scissors::hit_test_compound(
            &decode_paths(&v["paths"])?,
            field(v, "click")?,
            field(v, "maxDist")?,
        )? {
            Some(cut) => {
                let mut hit = encode_cut_hit(cut.hit);
                hit["ring"] = json!(cut.ring);
                hit
            }
            None => Value::Null,
        },
        "compound_knife_hits" => {
            let hits = scissors::knife_hits_compound(
                &decode_paths(&v["paths"])?,
                field(v, "a")?,
                field(v, "b")?,
            )?;
            Value::Array(
                hits.into_iter()
                    .map(|cut| {
                        let mut hit = encode_cut_hit(cut.hit);
                        hit["ring"] = json!(cut.ring);
                        hit
                    })
                    .collect(),
            )
        }
        "compound_cut_at" => encode_path_pieces(&scissors::cut_compound_at(
            &decode_paths(&v["paths"])?,
            scissors::CompoundCutHit {
                ring: field(&v["hit"], "ring")?,
                hit: decode_cut_hit(&v["hit"])?,
            },
        )?),
        "compound_cut_many" => {
            let hits = v["hits"]
                .as_array()
                .ok_or_else(|| input("hits must be an array"))?
                .iter()
                .map(|hit| {
                    Ok(scissors::CompoundCutHit {
                        ring: field(hit, "ring")?,
                        hit: decode_cut_hit(hit)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            encode_path_pieces(&scissors::cut_compound_at_many(
                &decode_paths(&v["paths"])?,
                &hits,
            )?)
        }
        "compound_scissors" => encode_path_pieces(&scissors::scissors_cut_compound(
            &decode_paths(&v["paths"])?,
            field(v, "click")?,
            field(v, "maxDist")?,
        )?),
        "compound_knife" => encode_path_pieces(&scissors::knife_cut_compound(
            &decode_paths(&v["paths"])?,
            field(v, "a")?,
            field(v, "b")?,
        )?),
        "compound_knife_split" => encode_path_pieces(&scissors::knife_split_compound(
            &decode_paths(&v["paths"])?,
            field(v, "a")?,
            field(v, "b")?,
        )?),
        "scissors" => encode_paths(&edit::scissors_cut(
            &decode_path(&v["path"])?,
            field(v, "click")?,
            field(v, "maxDist")?,
        )?),
        "knife" => encode_paths(&edit::knife_cut(
            &decode_path(&v["path"])?,
            field(v, "a")?,
            field(v, "b")?,
        )?),
        "knife_split" => encode_paths(&edit::knife_split(
            &decode_path(&v["path"])?,
            field(v, "a")?,
            field(v, "b")?,
        )?),
        "measure" => {
            let m = edit::measure(field(v, "a")?, field(v, "b")?)?;
            json!({"a":m.a,"b":m.b,"distance":m.distance,"angleDeg":m.angle_deg,"delta":m.delta})
        }
        "snap_rays" => match edit::snap_to_rays(
            field(v, "start")?,
            field(v, "cursor")?,
            v["base"].as_f64().unwrap_or(0.),
            v["count"].as_u64().unwrap_or(8) as usize,
            v["toleranceDegrees"].as_f64().unwrap_or(3.),
        )? {
            Some((point, angle)) => json!({"point":point,"angle":angle}),
            None => Value::Null,
        },
        "snap_angle" => encode(edit::snap_to_angle(
            field(v, "start")?,
            field(v, "end")?,
            v["stepDeg"].as_f64().unwrap_or(45.0),
        )?)?,
        "drag_guides" => {
            let moving = edit::BBox {
                min: field(&v["moving"], "min")?,
                max: field(&v["moving"], "max")?,
            };
            let targets = decode_boxes(&v["targets"])?;
            let g = edit::drag_align_guides(moving, &targets, field(v, "threshold")?)?;
            json!({
                "delta": g.delta,
                "guides": g.guides.iter().map(|l| json!({"vertical":l.vertical,"position":l.position})).collect::<Vec<_>>()
            })
        }
        "align_relative" => {
            let boxes = decode_boxes(&v["boxes"])?;
            let relative = match v["relative"].as_str().unwrap_or("selection") {
                "key" | "keyObject" => edit::RelativeTo::KeyObject,
                "page" => edit::RelativeTo::Page,
                "last" | "lastSelected" => edit::RelativeTo::LastSelected,
                _ => edit::RelativeTo::Selection,
            };
            let page = if v["page"].is_object() {
                Some(edit::BBox {
                    min: field(&v["page"], "min")?,
                    max: field(&v["page"], "max")?,
                })
            } else {
                None
            };
            let h = match v["h"].as_str() {
                Some("left") => Some(edit::HAlign::Left),
                Some("center") => Some(edit::HAlign::Center),
                Some("right") => Some(edit::HAlign::Right),
                _ => None,
            };
            let vert = match v["v"].as_str() {
                Some("top") => Some(edit::VAlign::Top),
                Some("center") => Some(edit::VAlign::Center),
                Some("bottom") => Some(edit::VAlign::Bottom),
                _ => None,
            };
            encode(edit::align_boxes_relative(
                &boxes,
                relative,
                v["key"].as_u64().map(|n| n as usize),
                page,
                v["last"].as_u64().map(|n| n as usize),
                h,
                vert,
            )?)?
        }
        "distribute_objects" => {
            let anchor = match v["anchor"].as_str().unwrap_or("center") {
                "start" | "leftOrTop" => edit::DistributeAnchor::LeftOrTop,
                "end" | "rightOrBottom" => edit::DistributeAnchor::RightOrBottom,
                _ => edit::DistributeAnchor::Center,
            };
            encode(edit::distribute_objects(
                &decode_boxes(&v["boxes"])?,
                v["horizontal"].as_bool().unwrap_or(true),
                anchor,
            )?)?
        }
        "distribute_spacing" => encode(edit::distribute_spacing(
            &decode_boxes(&v["boxes"])?,
            v["horizontal"].as_bool().unwrap_or(true),
            field(v, "gap")?,
        )?)?,
        _ => return Ok(None),
    };
    Ok(Some(output))
}
