//! Browser host-GPU descriptor matching (MAT1): the kernel packs one payload
//! with the shared descriptor table and the seed pair list, the browser runs
//! every pair through [`MATCH_WGSL`] in a single WebGPU session, and the
//! response bytes come back here for the shared acceptance filter.

use crate::features::{self, Feature, FeatureOptions, Match};
use crate::{Image, Result, error, seeding};

/// Qualified WGSL for the descriptor matcher — the same template the native
/// `gpu` feature compiles (`match_rows` / `match_cols`), plus the combined
/// `match_pair` entry point the browser uses: one dispatch per pair covers
/// `rows + cols` workgroups, each workgroup reducing exactly one row or one
/// column, so per-row/per-column semantics are identical to the native path.
/// `Params` carries descriptor-table offsets (in 128-float records) so both
/// directions bind one shared table instead of per-pair copies; the native
/// path passes zero offsets with per-pair buffers. `WG` stays at the
/// compute-core anchor (256); the browser dispatches with the same size.
pub const MATCH_WGSL: &str = r#"
struct Params {
    rows: u32,
    cols: u32,
    offset_a: u32,
    offset_b: u32,
}
struct RowBest {
    j: u32,
    d1: f32,
    d2: f32,
}
struct ColBest {
    i: u32,
    d1: f32,
}

@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> da: array<f32>;
@group(0) @binding(2) var<storage, read> db: array<f32>;
@group(0) @binding(3) var<storage, read_write> rows_out: array<RowBest>;
@group(0) @binding(4) var<storage, read_write> cols_out: array<ColBest>;

const NONE: u32 = 0xFFFFFFFFu;
const INF: f32 = 3.402823466e+38;
const WG: u32 = 256;

var<workgroup> sh_d: array<f32, WG>;
var<workgroup> sh_i: array<u32, WG>;
var<workgroup> sh_s: array<f32, WG>;

// True when (a_d, a_i) outranks (b_d, b_i): smaller distance, then smaller index.
fn better(a_d: f32, a_i: u32, b_d: f32, b_i: u32) -> bool {
    return a_d < b_d || (a_d == b_d && a_i < b_i);
}

fn dist(row: u32, col: u32) -> f32 {
    var d = 0.0;
    let ra = (params.offset_a + row) * 128u;
    let rb = (params.offset_b + col) * 128u;
    for (var k = 0u; k < 128u; k++) {
        let v = da[ra + k] - db[rb + k];
        d += v * v;
    }
    return d;
}

fn reduce_rows(row: u32, lid: u32) {
    var best_d = INF;
    var best_j = NONE;
    var second_d = INF;
    var col = lid;
    while (col < params.cols) {
        let d = dist(row, col);
        if (d < best_d) {
            second_d = best_d;
            best_d = d;
            best_j = col;
        } else if (d < second_d) {
            second_d = d;
        }
        col += WG;
    }
    sh_d[lid] = best_d;
    sh_i[lid] = best_j;
    sh_s[lid] = second_d;
    workgroupBarrier();
    var stride = WG / 2u;
    while (stride > 0u) {
        if (lid < stride) {
            let other = lid + stride;
            if (better(sh_d[other], sh_i[other], sh_d[lid], sh_i[lid])) {
                // other becomes best; old best competes for second
                sh_s[lid] = min(min(sh_s[lid], sh_s[other]), sh_d[lid]);
                sh_d[lid] = sh_d[other];
                sh_i[lid] = sh_i[other];
            } else {
                sh_s[lid] = min(min(sh_s[lid], sh_s[other]), sh_d[other]);
            }
        }
        workgroupBarrier();
        stride = stride / 2u;
    }
    if (lid == 0u) {
        rows_out[row] = RowBest(sh_i[0], sh_d[0], sh_s[0]);
    }
}

fn reduce_cols(col: u32, lid: u32) {
    var best_d = INF;
    var best_i = NONE;
    var row = lid;
    while (row < params.rows) {
        let d = dist(row, col);
        if (d < best_d) {
            best_d = d;
            best_i = row;
        }
        row += WG;
    }
    sh_d[lid] = best_d;
    sh_i[lid] = best_i;
    workgroupBarrier();
    var stride = WG / 2u;
    while (stride > 0u) {
        if (lid < stride) {
            let other = lid + stride;
            if (better(sh_d[other], sh_i[other], sh_d[lid], sh_i[lid])) {
                sh_d[lid] = sh_d[other];
                sh_i[lid] = sh_i[other];
            }
        }
        workgroupBarrier();
        stride = stride / 2u;
    }
    if (lid == 0u) {
        cols_out[col] = ColBest(sh_i[0], sh_d[0]);
    }
}

@compute @workgroup_size(WG)
fn match_rows(@builtin(workgroup_id) wg: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>) {
    reduce_rows(wg.x, lid.x);
}

@compute @workgroup_size(WG)
fn match_cols(@builtin(workgroup_id) wg: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>) {
    reduce_cols(wg.x, lid.x);
}

@compute @workgroup_size(WG)
fn match_pair(@builtin(workgroup_id) wg: vec3<u32>, @builtin(local_invocation_id) lid: vec3<u32>) {
    if (wg.x < params.rows) {
        reduce_rows(wg.x, lid.x);
    } else {
        reduce_cols(wg.x - params.rows, lid.x);
    }
}
"#;

/// MAT1 payload magic ('MAT1').
pub const MATCH_PAYLOAD_MAGIC: u32 = 0x314D_4154;
/// Descriptor stride in f32 records; fixed by the feature extractor.
pub const MATCH_DESCRIPTOR_DIM: usize = 128;

/// Everything the browser needs for one host-GPU sparse matching round trip.
pub struct HostMatchingPlan {
    /// MAT1 blob: magic, image/pair counts, descriptor dim, per-image feature
    /// counts, one concatenated little-endian f32 descriptor block, then the
    /// pair list. Handed to the browser as an opaque buffer.
    pub payload: Vec<u8>,
    /// Seed-candidate pairs in payload order; the response arrives in the same
    /// order.
    pub pairs: Vec<(usize, usize)>,
    /// Features extracted with the reconstruction options; shared with the
    /// finish stage so extraction never runs twice.
    pub features: Vec<Vec<Feature>>,
}

/// Eligibility for the browser host-GPU matching path. Every seed-candidate
/// pair is matched there only when the candidate list covers the full pair
/// grid (up to 24 photos), so seeding and incremental registration both hit
/// the precomputed cache; larger sets keep the lazy host-side matcher. The
/// `second_chance` pass needs exact reverse seconds the GPU pair pass does
/// not produce, so it declines as well.
pub fn prepare_host_matching(
    images: &[Image],
    feature_limit: usize,
    options: &FeatureOptions,
) -> Result<Option<HostMatchingPlan>> {
    if images.len() < 2 || images.len() > 24 || options.second_chance {
        return Ok(None);
    }
    let mut features = Vec::with_capacity(images.len());
    for image in images {
        features.push(features::extract_with_options(image, feature_limit, options)?);
    }
    let pairs = seeding::pair_candidates(images.len());
    Ok(Some(HostMatchingPlan {
        payload: pack_payload(&features, &pairs),
        pairs,
        features,
    }))
}

/// MAT1 layout: magic, image count, pair count, descriptor dim, per-image
/// feature counts, the concatenated descriptor block, then u32 (a, b) pairs.
/// All integers little-endian u32, descriptors little-endian f32.
pub fn pack_payload(features: &[Vec<Feature>], pairs: &[(usize, usize)]) -> Vec<u8> {
    let mut out = Vec::new();
    let u32s = |out: &mut Vec<u8>, v: u32| out.extend_from_slice(&v.to_le_bytes());
    u32s(&mut out, MATCH_PAYLOAD_MAGIC);
    u32s(&mut out, features.len() as u32);
    u32s(&mut out, pairs.len() as u32);
    u32s(&mut out, MATCH_DESCRIPTOR_DIM as u32);
    for list in features {
        u32s(&mut out, list.len() as u32);
    }
    for list in features {
        for feature in list {
            for v in feature.descriptor {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
    }
    for &(a, b) in pairs {
        u32s(&mut out, a as u32);
        u32s(&mut out, b as u32);
    }
    out
}

/// Parses the browser's match response: per plan pair, `rows * 12` bytes of
/// packed `RowBest` (j, d1, d2) followed by `cols * 8` bytes of packed
/// `ColBest` (i, d1), in payload pair order. Applies the shared acceptance
/// filter, so the result is interchangeable with `matches_with_options`.
pub fn matches_from_gpu_response(
    plan: &HostMatchingPlan,
    mut bytes: &[u8],
    options: &FeatureOptions,
) -> Result<Vec<((usize, usize), Vec<Match>)>> {
    let mut out = Vec::with_capacity(plan.pairs.len());
    for &(a, b) in &plan.pairs {
        let (rows, cols) = (plan.features[a].len(), plan.features[b].len());
        let (row_bytes, col_bytes) = (rows * 12, cols * 8);
        if bytes.len() < row_bytes + col_bytes {
            return Err(error("Host match response is truncated"));
        }
        let (rows_part, rest) = bytes.split_at(row_bytes);
        let (cols_part, rest) = rest.split_at(col_bytes);
        bytes = rest;
        let best_a: Vec<(usize, f32, f32)> = rows_part
            .chunks_exact(12)
            .map(|chunk| {
                (
                    match u32::from_le_bytes(chunk[0..4].try_into().unwrap()) {
                        u32::MAX => usize::MAX,
                        raw => raw as usize,
                    },
                    f32::from_le_bytes(chunk[4..8].try_into().unwrap()),
                    f32::from_le_bytes(chunk[8..12].try_into().unwrap()),
                )
            })
            .collect();
        let best_b: Vec<(usize, f32)> = cols_part
            .chunks_exact(8)
            .map(|chunk| {
                (
                    match u32::from_le_bytes(chunk[0..4].try_into().unwrap()) {
                        u32::MAX => usize::MAX,
                        raw => raw as usize,
                    },
                    f32::from_le_bytes(chunk[4..8].try_into().unwrap()),
                )
            })
            .collect();
        if best_a.len() != rows || best_b.len() != cols {
            return Err(error("Host match response has a mismatched pair layout"));
        }
        out.push((
            (a, b),
            features::select_matches(
                &plan.features[a],
                &plan.features[b],
                &best_a,
                &best_b,
                options,
            ),
        ));
    }
    if !bytes.is_empty() {
        return Err(error("Host match response has trailing bytes"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feature(seed: u32, n: usize) -> Vec<Feature> {
        (0..n)
            .map(|i| {
                let mut descriptor = [0f32; 128];
                for (k, value) in descriptor.iter_mut().enumerate() {
                    *value = (((seed * 131 + i as u32 * 17 + k as u32) % 1009) as f32) / 1009.;
                }
                Feature {
                    x: i as f64,
                    y: i as f64,
                    descriptor,
                }
            })
            .collect()
    }

    /// Plain full scan with the CPU path's ordering semantics (strict `<`, so
    /// equal distances keep the first index) — the reference the response
    /// parser must reproduce through `select_matches`.
    fn scan_bests(
        a: &[Feature],
        b: &[Feature],
    ) -> (Vec<(usize, f32, f32)>, Vec<(usize, f32)>) {
        let mut best_a = vec![(usize::MAX, f32::INFINITY, f32::INFINITY); a.len()];
        let mut best_b = vec![(usize::MAX, f32::INFINITY); b.len()];
        for (i, x) in a.iter().enumerate() {
            for (j, y) in b.iter().enumerate() {
                let mut d = 0f32;
                for k in 0..128 {
                    let v = x.descriptor[k] - y.descriptor[k];
                    d += v * v;
                }
                if d < best_a[i].1 {
                    best_a[i].2 = best_a[i].1;
                    best_a[i].1 = d;
                    best_a[i].0 = j;
                } else if d < best_a[i].2 {
                    best_a[i].2 = d;
                }
                if d < best_b[j].1 {
                    best_b[j] = (i, d);
                }
            }
        }
        (best_a, best_b)
    }

    fn pack_bests(best_a: &[(usize, f32, f32)], best_b: &[(usize, f32)]) -> Vec<u8> {
        let mut out = Vec::new();
        for &(j, d1, d2) in best_a {
            out.extend_from_slice(
                &(match j {
                    usize::MAX => u32::MAX,
                    raw => raw as u32,
                })
                .to_le_bytes(),
            );
            out.extend_from_slice(&d1.to_le_bytes());
            out.extend_from_slice(&d2.to_le_bytes());
        }
        for &(i, d1) in best_b {
            out.extend_from_slice(
                &(match i {
                    usize::MAX => u32::MAX,
                    raw => raw as u32,
                })
                .to_le_bytes(),
            );
            out.extend_from_slice(&d1.to_le_bytes());
        }
        out
    }

    #[test]
    fn pair_grid_covers_every_pair_up_to_24_then_windows() {
        let all = seeding::pair_candidates(24);
        assert_eq!(all.len(), 24 * 23 / 2);
        assert!(all.contains(&(0, 23)) && all.contains(&(22, 23)));
        assert!(all.iter().all(|&(a, b)| a < b));
        let window = seeding::pair_candidates(25);
        // a = 0..=16 contribute 8 pairs each, a = 17..=23 contribute 7..=1.
        assert_eq!(window.len(), 17 * 8 + 28);
        assert!(window.contains(&(0, 8)) && !window.contains(&(0, 9)));
        assert!(window.contains(&(16, 24)) && !window.contains(&(15, 24)));
        assert!(window.iter().all(|&(a, b)| a < b && b <= a + 9));
    }

    #[test]
    fn payload_roundtrips_through_the_response_parser() {
        let features = vec![feature(3, 5), feature(7, 4), feature(11, 6)];
        let pairs = vec![(0, 1), (0, 2), (1, 2)];
        let payload = pack_payload(&features, &pairs);
        assert_eq!(&payload[0..4], &MATCH_PAYLOAD_MAGIC.to_le_bytes(), "magic");
        assert_eq!(u32::from_le_bytes(payload[4..8].try_into().unwrap()), 3);
        assert_eq!(u32::from_le_bytes(payload[8..12].try_into().unwrap()), 3);
        let plan = HostMatchingPlan {
            payload,
            pairs: pairs.clone(),
            features: features.clone(),
        };
        let options = FeatureOptions::BASELINE;
        let response: Vec<u8> = pairs
            .iter()
            .flat_map(|&(a, b)| {
                let (best_a, best_b) = scan_bests(&features[a], &features[b]);
                pack_bests(&best_a, &best_b)
            })
            .collect();
        let parsed = matches_from_gpu_response(&plan, &response, &options).unwrap();
        assert_eq!(parsed.len(), pairs.len());
        for ((a, b), matched) in &parsed {
            let expected = features::matches_with_options(&features[*a], &features[*b], &options);
            assert_eq!(
                *matched, expected,
                "pair ({a}, {b}) must equal the CPU reference"
            );
        }
    }

    #[test]
    fn response_parser_rejects_truncated_and_trailing_bytes() {
        let features = vec![feature(1, 2), feature(2, 3)];
        let plan = HostMatchingPlan {
            payload: Vec::new(),
            pairs: vec![(0, 1)],
            features,
        };
        let full = 2 * 12 + 3 * 8;
        assert!(matches_from_gpu_response(&plan, &vec![0u8; full - 1], &FeatureOptions::BASELINE)
            .unwrap_err()
            .message
            .contains("truncated"));
        assert!(matches_from_gpu_response(&plan, &vec![0u8; full + 4], &FeatureOptions::BASELINE)
            .unwrap_err()
            .message
            .contains("trailing"));
    }

    #[test]
    fn prepare_declines_large_sets_and_second_chance() {
        let image = Image {
            width: 48,
            height: 48,
            focal: 50.,
            rgb: vec![128u8; 48 * 48 * 3],
        };
        let images: Vec<Image> = (0..25).map(|_| image.clone()).collect();
        assert!(
            prepare_host_matching(&images, 100, &FeatureOptions::BASELINE)
                .unwrap()
                .is_none()
        );
        let images: Vec<Image> = (0..2).map(|_| image.clone()).collect();
        assert!(
            prepare_host_matching(
                &images,
                100,
                &FeatureOptions {
                    second_chance: true,
                    ..FeatureOptions::ROOT
                },
            )
            .unwrap()
            .is_none()
        );
        let plan = prepare_host_matching(&images, 100, &FeatureOptions::BASELINE)
            .unwrap()
            .unwrap();
        assert_eq!(plan.pairs, vec![(0, 1)]);
    }
}
