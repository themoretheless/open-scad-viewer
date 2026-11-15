//! Time and peak heap of manifold operations on 1M+ triangle meshes.
//!
//! ```sh
//! cargo run --release -p manifold-ops --example profile_large -- [subdivisions] [--json FILE] [--only OP_PREFIX]
//! ```
//!
//! The default fixture is a level-8 icosphere (1,310,720 triangles). Peak heap
//! is the high-water mark of live heap bytes above the level at operation
//! start (inputs excluded, outputs included), counted by a global allocator.
//! Booleans run on two overlapping level-`n-6` spheres (320 triangles each
//! for the default) at the default work budget because the BSP kernel is
//! superlinear.
#[path = "support/large_mesh.rs"]
mod large_mesh;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use large_mesh::{icosphere, place, unwelded};
use manifold_core::{
    RepairMode, SelfIntersectionOptions, check, mass_properties, metrics, repair, repair_with_mode,
    self_intersections,
};
use manifold_ops::decimate::{DecimateOptions, decimate};
use manifold_ops::hull::{HullOptions, hull_mesh};
use manifold_ops::{BooleanOp, BooleanOptions, boolean_manifold_with_options};

struct Counting;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            let live = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, layout, new_size) };
        if !q.is_null() {
            if new_size >= layout.size() {
                let live = LIVE.fetch_add(new_size - layout.size(), Ordering::Relaxed)
                    + (new_size - layout.size());
                PEAK.fetch_max(live, Ordering::Relaxed);
            } else {
                LIVE.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }
        q
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

struct Row {
    op: &'static str,
    triangles: usize,
    ms: f64,
    peak_mib: f64,
    note: String,
}

fn measure<T>(
    only: &Option<String>,
    rows: &mut Vec<Row>,
    op: &'static str,
    triangles: usize,
    f: impl FnOnce() -> T,
    note: impl FnOnce(&T) -> String,
) {
    if only.as_deref().is_some_and(|o| !op.starts_with(o)) {
        return;
    }
    let base = LIVE.load(Ordering::Relaxed);
    PEAK.store(base, Ordering::Relaxed);
    let start = Instant::now();
    let out = f();
    let ms = start.elapsed().as_secs_f64() * 1e3;
    let peak_mib = (PEAK.load(Ordering::Relaxed) - base) as f64 / (1024.0 * 1024.0);
    let note = note(&out);
    drop(out);
    eprintln!("{op:<28} {triangles:>9} tris {ms:>10.1} ms {peak_mib:>9.1} MiB  {note}");
    rows.push(Row {
        op,
        triangles,
        ms,
        peak_mib,
        note,
    });
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut level = 8usize;
    let mut json = None;
    let mut only = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--json" {
            json = it.next().cloned();
        } else if a == "--only" {
            only = it.next().cloned();
        } else {
            level = a.parse().expect("subdivisions must be an integer");
        }
    }
    let (p, i) = icosphere(level);
    let n = i.len() / 3;
    let (soup_p, soup_i) = unwelded(&p, &i);
    let mut rows = Vec::new();

    measure(
        &only,
        &mut rows,
        "check",
        n,
        || check(&p, &i),
        |r| format!("manifold={}", r.is_manifold()),
    );
    measure(
        &only,
        &mut rows,
        "metrics",
        n,
        || metrics(&p, &i),
        |m| format!("genus={:?}", m.components[0].genus),
    );
    measure(
        &only,
        &mut rows,
        "repair-conservative/soup",
        n,
        || repair(&soup_p, &soup_i, 0.0),
        |o| format!("vertices={}", o.positions.len() / 3),
    );
    measure(
        &only,
        &mut rows,
        "repair-full/soup",
        n,
        || repair_with_mode(&soup_p, &soup_i, 0.0, RepairMode::Full),
        |o| format!("triangles={}", o.indices.len() / 3),
    );
    measure(
        &only,
        &mut rows,
        "mass_properties",
        n,
        || mass_properties(&p, &i, 1.0).unwrap(),
        |m| format!("volume={:.6}", m.volume),
    );
    measure(
        &only,
        &mut rows,
        "self_intersections",
        n,
        || {
            self_intersections(
                &p,
                &i,
                SelfIntersectionOptions {
                    max_candidate_pairs: usize::MAX,
                    ..Default::default()
                },
            )
        },
        |r| match r {
            Ok(r) => format!("pairs={}", r.pairs.len()),
            Err(e) => format!("error={e:?}"),
        },
    );
    measure(
        &only,
        &mut rows,
        "hull_mesh",
        n,
        || {
            hull_mesh(
                &p,
                &i,
                HullOptions {
                    max_points: usize::MAX,
                    ..Default::default()
                },
            )
        },
        |r| match r {
            Ok(h) => format!("hull_triangles={}", h.indices.len() / 3),
            Err(e) => format!("error={e:?}"),
        },
    );
    measure(
        &only,
        &mut rows,
        "decimate/10%",
        n,
        || {
            decimate(
                &p,
                &i,
                DecimateOptions {
                    target_triangles: n / 10,
                    measure_deviation: false,
                    ..Default::default()
                },
            )
        },
        |r| match r {
            Ok(d) => format!("output_triangles={}", d.report.output_triangles),
            Err(e) => format!("error={e:?}"),
        },
    );
    let small = level.saturating_sub(6);
    let (ap, ai) = icosphere(small);
    let (mut bp, bi) = icosphere(small);
    place(&mut bp, 1.0, [0.5, 0.3, 0.2]);
    measure(
        &only,
        &mut rows,
        "boolean/union",
        ai.len() / 3 + bi.len() / 3,
        || {
            boolean_manifold_with_options(
                &ap,
                &ai,
                &bp,
                &bi,
                BooleanOp::Union,
                BooleanOptions::default(),
            )
        },
        |r| match r {
            Ok(o) => format!("triangles={}", o.indices.len() / 3),
            Err(e) => format!("error={e:?}"),
        },
    );

    if let Some(file) = json {
        let body: Vec<String> = rows
            .iter()
            .map(|r| {
                format!(
                    "    {{\"op\": \"{}\", \"triangles\": {}, \"ms\": {:.1}, \"peak_mib\": {:.1}, \"note\": \"{}\"}}",
                    r.op, r.triangles, r.ms, r.peak_mib, r.note.replace('"', "'")
                )
            })
            .collect();
        std::fs::write(
            &file,
            format!(
                "{{\n  \"subdivisions\": {level},\n  \"rows\": [\n{}\n  ]\n}}\n",
                body.join(",\n")
            ),
        )
        .expect("write json");
    }
}
