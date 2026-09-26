//! Browser WebGPU SDF sweep: prepare/finish split over the value dispatch.
//! The kernel flattens the field and ships the shared shader text; the host
//! samples the grid on the GPU; extraction finishes here on the CPU reference
//! path. Ineligible fields return null and the caller uses `sdf_tessellate`.
//! `prepare_batch` registers many grids at once so the host samples them all
//! in one WebGPU session (one pipeline, one submit) with the shader shipped a
//! single time.
use crate::{encode, field, input};
use sdf_core::{Field, Grid, SDF_WGSL};
use std::cell::RefCell;
use std::collections::BTreeMap;
use value_codec::{Value, json};

/// Job count and total sample ceilings for one batch sweep. The values bound
/// the browser-side upload plus the f32 output readback (16 Mi samples is a
/// 64 MiB Float32Array).
const MAX_BATCH_JOBS: usize = 64;
const MAX_BATCH_SAMPLES: u64 = 16 * 1024 * 1024;

thread_local! {
    #[allow(clippy::type_complexity)]
    static PENDING: RefCell<(u64, BTreeMap<u64, (Field, Grid)>)> =
        const { RefCell::new((0, BTreeMap::new())) };
}

fn register(sdf: Field, grid: Grid) -> u64 {
    PENDING.with(|pending| {
        let mut pending = pending.borrow_mut();
        pending.0 += 1;
        let id = pending.0;
        pending.1.insert(id, (sdf, grid));
        id
    })
}

fn job_payload(sdf: &Field, grid: &Grid) -> crate::Result<Option<value_codec::Value>> {
    sdf_core::check_grid_budget(sdf, grid)?;
    let Some(flat) = sdf.to_flat() else {
        return Ok(None);
    };
    Ok(Some(json!({
        "kinds": flat.kinds,
        "params": flat.params,
        "aux": flat.aux,
        "triangles": flat.triangles,
        "min": grid.min,
        "max": grid.max,
        "cells": grid.cells,
    })))
}

pub fn prepare(v: &Value) -> crate::Result<Value> {
    let sdf: Field = field(v, "field")?;
    let grid: Grid = field(v, "grid")?;
    let Some(mut payload) = job_payload(&sdf, &grid)? else {
        return encode(Value::Null);
    };
    let id = register(sdf, grid);
    payload["id"] = json!(id);
    payload["wgsl"] = json!(SDF_WGSL);
    encode(payload)
}

/// Registers a batch of `{field, grid}` jobs in one call. Slots come back in
/// input order; ineligible or over-budget jobs are `null` and the host leaves
/// them to the CPU reference path. The shared shader text ships once.
pub fn prepare_batch(v: &Value) -> crate::Result<Value> {
    let jobs = v["jobs"]
        .as_array()
        .cloned()
        .ok_or_else(|| input("sdf_prepare_batch expects a jobs array"))?;
    if jobs.len() > MAX_BATCH_JOBS {
        return Err(input(format!(
            "sdf_prepare_batch accepts at most {MAX_BATCH_JOBS} jobs, got {}",
            jobs.len()
        )));
    }
    let mut total_samples = 0u64;
    let mut slots = Vec::with_capacity(jobs.len());
    for job in &jobs {
        let (sdf, grid) = match (
            value_codec::from_value::<Field>(job["field"].clone()),
            value_codec::from_value::<Grid>(job["grid"].clone()),
        ) {
            (Ok(sdf), Ok(grid)) => (sdf, grid),
            // A malformed slot is a caller bug, unlike an ineligible field.
            _ => return Err(input("sdf_prepare_batch jobs need field and grid")),
        };
        let payload = match job_payload(&sdf, &grid) {
            Ok(Some(payload)) => payload,
            // Ineligible or over-budget: the CPU path re-checks and reports.
            _ => {
                slots.push(Value::Null);
                continue;
            }
        };
        let samples = (grid.cells[0] as u64 + 1)
            * (grid.cells[1] as u64 + 1)
            * (grid.cells[2] as u64 + 1);
        if total_samples + samples > MAX_BATCH_SAMPLES {
            return Err(input(format!(
                "sdf_prepare_batch samples exceed the {MAX_BATCH_SAMPLES} ceiling"
            )));
        }
        total_samples += samples;
        let id = register(sdf, grid);
        let mut payload = payload;
        payload["id"] = json!(id);
        slots.push(payload);
    }
    encode(json!({ "jobs": slots, "wgsl": SDF_WGSL }))
}

pub fn finish(v: &Value) -> crate::Result<Value> {
    let id = field::<u64>(v, "id")?;
    let values64 = field::<Vec<f64>>(v, "values")?;
    let values: Vec<f32> = values64.iter().map(|&x| x as f32).collect();
    let (field, grid) = PENDING
        .with(|pending| pending.borrow_mut().1.remove(&id))
        .ok_or_else(|| input("Unknown SDF sweep handle"))?;
    let mesh =
        crate::mesh_from_triangles(sdf_core::polygonize_with_values(&field, &grid, &values)?);
    let report = mesh.inspect()?;
    encode(polygon_core::BuiltMesh { mesh, report })
}

#[cfg(test)]
mod tests {
    use super::*;
    use value_codec::json;

    #[test]
    fn prepare_finish_roundtrip_matches_reference_extraction() {
        // Sphere radius 10 at the origin, small grid.
        let field = json!({"kind": "sphere", "center": [0., 0., 0.], "radius": 10.});
        let grid =
            json!({"min": [-12., -12., -12.], "max": [12., 12., 12.], "cells": [16, 16, 16]});
        let prepared = prepare(&json!({"field": field, "grid": grid})).unwrap();
        let id = prepared["id"].as_f64().unwrap() as u64;
        assert!(prepared["wgsl"].as_str().unwrap().contains("@compute"));
        // Host substitute: sample the field on the CPU, rounding through f32
        // exactly like the shader output.
        let reference = sdf_core::polygonize(
            &value_codec::from_value::<Field>(field).unwrap(),
            &value_codec::from_value::<Grid>(grid).unwrap(),
        )
        .unwrap();
        let mut values = Vec::new();
        let [nx, ny, nz] = [16usize, 16, 16];
        for z in 0..=nz {
            for y in 0..=ny {
                for x in 0..=nx {
                    let p: [f64; 3] =
                        std::array::from_fn(|i| -12. + 24. * [x, y, z][i] as f64 / 16.);
                    let dist = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt() - 10.;
                    values.push(dist as f32 as f64);
                }
            }
        }
        let mesh_value = finish(&json!({"id": id, "values": values})).unwrap();
        let mesh: polygon_core::BuiltMesh = value_codec::from_value(mesh_value).unwrap();
        assert_eq!(mesh.mesh.indices, reference.indices);
        // Second use of the same handle must fail.
        assert!(finish(&json!({"id": id, "values": []})).is_err());
    }

    fn sphere_values(radius: f64, cells: usize) -> Vec<f64> {
        let mut values = Vec::new();
        for z in 0..=cells {
            for y in 0..=cells {
                for x in 0..=cells {
                    let p: [f64; 3] =
                        std::array::from_fn(|i| -12. + 24. * [x, y, z][i] as f64 / cells as f64);
                    values.push((p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt() - radius);
                }
            }
        }
        values.iter().map(|&v| v as f32 as f64).collect()
    }

    #[test]
    fn batch_prepare_registers_eligible_jobs_and_keeps_input_order() {
        let sphere = |radius| {
            json!({"field": {"kind": "sphere", "center": [0., 0., 0.], "radius": radius},
                   "grid": {"min": [-12., -12., -12.], "max": [12., 12., 12.], "cells": [8, 8, 8]}})
        };
        // Deform is not flattenable: it must come back null, not abort the batch.
        let deform = json!({
            "field": {"kind": "deform", "input": {"kind": "sphere", "center": [0., 0., 0.], "radius": 5.},
                      "deformation": {"kind": "twist", "origin": [0., 0., 0.], "radians_per_unit": 0.1}},
            "grid": {"min": [-12., -12., -12.], "max": [12., 12., 12.], "cells": [8, 8, 8]}
        });
        let prepared = prepare_batch(&json!({
            "jobs": [sphere(10.), deform, sphere(6.)]
        }))
        .unwrap();
        assert!(prepared["wgsl"].as_str().unwrap().contains("@compute"));
        let slots = prepared["jobs"].as_array().unwrap();
        assert_eq!(slots.len(), 3);
        assert!(slots[0].is_object() && slots[0]["id"].is_number());
        assert!(slots[1].is_null());
        assert!(slots[2].is_object() && slots[2]["id"].is_number());
        assert_ne!(slots[0]["id"], slots[2]["id"]);
        assert_eq!(slots[0]["cells"], json!([8, 8, 8]));
        // Every eligible slot finishes exactly like the single-job path.
        for (slot, radius) in [&slots[0], &slots[2]].into_iter().zip([10., 6.]) {
            let id = slot["id"].as_f64().unwrap() as u64;
            let mesh_value =
                finish(&json!({"id": id, "values": sphere_values(radius, 8)})).unwrap();
            let mesh: polygon_core::BuiltMesh = value_codec::from_value(mesh_value).unwrap();
            assert!(!mesh.mesh.indices.is_empty());
            assert!(finish(&json!({"id": id, "values": []})).is_err());
        }
    }

    #[test]
    fn batch_prepare_rejects_malformed_slots() {
        assert!(prepare_batch(&json!({"jobs": [{"field": 1, "grid": 2}]})).is_err());
    }

    #[test]
    fn batch_prepare_enforces_the_total_sample_ceiling() {
        // Per-grid budget allows at most 64 cells per axis; 62 full grids
        // (65^3 samples each) exceed the 16 Mi-sample batch ceiling, 61 fit.
        let job = json!({"field": {"kind": "sphere", "center": [0., 0., 0.], "radius": 10.},
                        "grid": {"min": [-12., -12., -12.], "max": [12., 12., 12.], "cells": [64, 64, 64]}});
        let fits: Vec<Value> = (0..61).map(|_| job.clone()).collect();
        assert!(prepare_batch(&json!({ "jobs": fits })).is_ok());
        let over: Vec<Value> = (0..62).map(|_| job.clone()).collect();
        assert!(prepare_batch(&json!({ "jobs": over })).is_err());
    }

    #[test]
    fn batch_prepare_caps_job_count() {
        let job = json!({"field": {"kind": "sphere", "center": [0., 0., 0.], "radius": 10.},
                        "grid": {"min": [-12., -12., -12.], "max": [12., 12., 12.], "cells": [4, 4, 4]}});
        let too_many: Vec<Value> = (0..=MAX_BATCH_JOBS).map(|_| job.clone()).collect();
        assert!(prepare_batch(&json!({ "jobs": too_many })).is_err());
        let exactly: Vec<Value> = (0..MAX_BATCH_JOBS).map(|_| job.clone()).collect();
        assert!(prepare_batch(&json!({ "jobs": exactly })).is_ok());
    }
}
