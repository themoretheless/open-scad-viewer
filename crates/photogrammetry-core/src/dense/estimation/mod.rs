//! Photometric hypotheses only. Geometry checks and fusion do not mutate these maps.
use super::{DenseDiagnostics, DenseEstimator, DenseOptions, DepthMap, cancelled};
use crate::{Image, Reconstruction, Result, camera::Camera, math::*};

/// The NCC sweep compute shader (WGSL), shared by the native `gpu` feature and
/// the browser WebGPU host path; both must execute the identical text.
pub const SWEEP_WGSL: &str = r#"
struct Params {
    width: u32,
    height: u32,
    n_hyp: u32,
    n_src: u32,
    plen: u32,
    radius: u32,
    needed: u32,
    ref_w: u32,
    ref_h: u32,
    ref_goff: u32,
    min_corr: f32,
    margin: f32,
    step: f32,
    ref_f: f32,
    ref_cx: f32,
    ref_cy: f32,
}
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> hyps: array<f32>;
@group(0) @binding(2) var<storage, read> srcf: array<f32>;
@group(0) @binding(3) var<storage, read> srcm: array<u32>;
@group(0) @binding(4) var<storage, read> grays: array<f32>;
@group(0) @binding(5) var<storage, read_write> scores: array<f32>;
// sweep_select only: per-pixel hypothesis bin range (lo, hi) and the packed
// selection result (valid, best bin, parabolic offset, score).
@group(0) @binding(6) var<storage, read> bins: array<u32>;
@group(0) @binding(7) var<storage, read_write> results: array<vec4<f32>>;

// Gray levels live in [0, 1]; -1 marks an invalid (out-of-frame) sample,
// matching the CPU's Option-returning bilinear sampler.
fn sample_gray(offset: u32, w: u32, h: u32, x: f32, y: f32) -> f32 {
    if (!(x >= 0.0) || !(y >= 0.0) || x >= f32(w) - 1.0 || y >= f32(h) - 1.0) {
        return -1.0;
    }
    let ix = u32(x);
    let iy = u32(y);
    let a = x - f32(ix);
    let b = y - f32(iy);
    let i = offset + iy * w + ix;
    let p00 = grays[i];
    let p10 = grays[i + 1u];
    let p01 = grays[i + w];
    let p11 = grays[i + w + 1u];
    return (1.0 - b) * ((1.0 - a) * p00 + a * p10) + b * ((1.0 - a) * p01 + a * p11);
}

// Centered reference patch and its energy; returns false for flat patches.
fn reference_patch(px: f32, py: f32, centered: ptr<function, array<f32, 25>>, energy: ptr<function, f32>) -> bool {
    let ps = params.radius * 2u + 1u;
    var refv: array<f32, 25>;
    for (var i = 0u; i < params.plen; i++) {
        let ox = f32(i % ps) - f32(params.radius);
        let oy = f32(i / ps) - f32(params.radius);
        // Interior pixels keep the reference patch fully in frame (CPU invariant).
        refv[i] = sample_gray(params.ref_goff, params.ref_w, params.ref_h, px + ox * params.step, py + oy * params.step);
    }
    var mean = 0.0;
    for (var i = 0u; i < params.plen; i++) {
        mean += refv[i];
    }
    mean = mean / f32(params.plen);
    var e = 0.0;
    for (var i = 0u; i < params.plen; i++) {
        (*centered)[i] = refv[i] - mean;
        e += (*centered)[i] * (*centered)[i];
    }
    *energy = e;
    return e >= 0.0001;
}

// Mean NCC over the sources supporting depth z at reference pixel (px, py);
// -1 when fewer than params.needed sources correlate above 0.4.
fn score_hypothesis(px: f32, py: f32, z: f32, centered: ptr<function, array<f32, 25>>, energy: f32) -> f32 {
    let ps = params.radius * 2u + 1u;
    var sum = 0.0;
    var count = 0u;
    for (var s = 0u; s < params.n_src; s++) {
        let fb = s * 16u;
        let mb = s * 4u;
        let goff = srcm[mb];
        let gw = srcm[mb + 1u];
        let gh = srcm[mb + 2u];
        let focal = srcf[fb + 12u];
        let cx = srcf[fb + 13u];
        let cy = srcf[fb + 14u];
        var ssum = 0.0;
        var sq = 0.0;
        var cov = 0.0;
        var ok = true;
        for (var i = 0u; i < params.plen; i++) {
            let ox = (f32(i % ps) - f32(params.radius)) * params.step;
            let oy = (f32(i / ps) - f32(params.radius)) * params.step;
            let rx = (px - params.ref_cx + ox) / params.ref_f;
            let ry = (py - params.ref_cy + oy) / params.ref_f;
            let vx = srcf[fb] * rx + srcf[fb + 1u] * ry + srcf[fb + 2u];
            let vy = srcf[fb + 3u] * rx + srcf[fb + 4u] * ry + srcf[fb + 5u];
            let vz = srcf[fb + 6u] * rx + srcf[fb + 7u] * ry + srcf[fb + 8u];
            let wx = vx * z + srcf[fb + 9u];
            let wy = vy * z + srcf[fb + 10u];
            let wz = vz * z + srcf[fb + 11u];
            if (wz <= 0.0) {
                ok = false;
                break;
            }
            let value = sample_gray(goff, gw, gh, focal * wx / wz + cx, focal * wy / wz + cy);
            if (value < 0.0) {
                ok = false;
                break;
            }
            ssum += value;
            sq += value * value;
            cov += (*centered)[i] * value;
        }
        if (!ok) {
            continue;
        }
        let energy_s = sq - ssum * ssum / f32(params.plen);
        if (energy_s < 0.0001) {
            continue;
        }
        let ncc = clamp(cov / sqrt(energy * energy_s), -1.0, 1.0);
        if (ncc > 0.4) {
            sum += ncc;
            count++;
        }
    }
    return select(-1.0, sum / f32(count), count >= params.needed);
}

// Scores for every hypothesis; the host (browser WebGPU path) selects on the CPU.
@compute @workgroup_size(16, 16)
fn sweep(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x;
    let y = id.y;
    if (x >= params.width || y >= params.height) {
        return;
    }
    let base = (y * params.width + x) * params.n_hyp;
    var centered: array<f32, 25>;
    var energy = 0.0;
    let px = f32(x) * params.step;
    let py = f32(y) * params.step;
    if (x < 3u || y < 3u || x + 3u >= params.width || y + 3u >= params.height
        || !reference_patch(px, py, &centered, &energy)) {
        for (var d = 0u; d < params.n_hyp; d++) {
            scores[base + d] = -1.0;
        }
        return;
    }
    for (var d = 0u; d < params.n_hyp; d++) {
        scores[base + d] = score_hypothesis(px, py, hyps[d], &centered, energy);
    }
}

// Last index wins ties, matching the CPU's Iterator::max_by on total order.
fn argmax(sc: ptr<function, array<f32, 128>>, lo: u32, hi: u32) -> u32 {
    var best = lo;
    for (var d = lo; d <= hi; d++) {
        if ((*sc)[d] >= (*sc)[best]) {
            best = d;
        }
    }
    return best;
}

// Native path: scores stay in registers and the CPU's pick_depth logic runs
// here per pixel (range clamp, uniqueness margin, parabolic sub-bin offset).
// results = (valid, best bin, offset, score); depth is rebuilt in f64 on the CPU.
@compute @workgroup_size(16, 16)
fn sweep_select(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x;
    let y = id.y;
    if (x >= params.width || y >= params.height) {
        return;
    }
    let pixel = y * params.width + x;
    results[pixel] = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    var centered: array<f32, 25>;
    var energy = 0.0;
    let px = f32(x) * params.step;
    let py = f32(y) * params.step;
    if (x < 3u || y < 3u || x + 3u >= params.width || y + 3u >= params.height
        || !reference_patch(px, py, &centered, &energy)) {
        return;
    }
    var sc: array<f32, 128>;
    for (var d = 0u; d < params.n_hyp; d++) {
        sc[d] = score_hypothesis(px, py, hyps[d], &centered, energy);
    }
    let last = params.n_hyp - 1u;
    let bin_lo = bins[pixel * 2u];
    let bin_hi = bins[pixel * 2u + 1u];
    let ranged = bin_lo > 0u || bin_hi < last;
    let best_ranged = argmax(&sc, bin_lo, bin_hi);
    var best = 0u;
    var alternative = -1.0;
    if (ranged && sc[best_ranged] >= params.min_corr) {
        best = best_ranged;
        for (var d = 0u; d < params.n_hyp; d++) {
            if (d + 3u < bin_lo || d > bin_hi + 3u) {
                alternative = max(alternative, sc[d]);
            }
        }
    } else {
        best = argmax(&sc, 0u, last);
        for (var d = 0u; d < params.n_hyp; d++) {
            if (max(d, best) - min(d, best) > 3u) {
                alternative = max(alternative, sc[d]);
            }
        }
    }
    if (sc[best] < params.min_corr || sc[best] - alternative < params.margin) {
        return;
    }
    var offset = 0.0;
    if (best > 0u && best < last) {
        let a = sc[best - 1u];
        let b = sc[best];
        let c = sc[best + 1u];
        let denom = a - 2.0 * b + c;
        if (a > 0.0 && c > 0.0 && abs(denom) > 1e-8) {
            offset = clamp(0.5 * (a - c) / denom, -0.5, 0.5);
        }
    }
    results[pixel] = vec4<f32>(1.0, f32(best), offset, sc[best]);
}
"#;

const SWEEP_ENTRY_PREFIX: &str = r#"@compute @workgroup_size(16, 16)
fn sweep(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = id.x;
    let y = id.y;"#;

const SWEEP_LINEAR_ENTRY_PREFIX: &str = r#"@compute @workgroup_size(256)
fn sweep(@builtin(global_invocation_index) index: u32) {
    let total = params.width * params.height;
    if (index >= total) {
        return;
    }
    let x = index % params.width;
    let y = index / params.width;"#;

pub const HOST_SWEEP_LINEAR_INDEXING_VARIANT_LABEL: &str = "photogrammetry-sweep-linear-indexing";

mod host_sweep_linear_indexing_wgsl;
mod host_sweep_view;
mod finish_host_views;
mod sweep_depth;
#[cfg(test)]
mod tests;
pub use host_sweep_linear_indexing_wgsl::*;
pub use host_sweep_view::*;
pub use finish_host_views::*;
use sweep_depth::*;
