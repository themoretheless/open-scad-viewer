//! Generic building-block kernels, embedded at compile time.
//!
//! `WG` anchor convention: tunable sources declare
//! `const WG: u32 = 256;` and `@workgroup_size(WG)`; the runtime may
//! substitute a tuned power-of-two before compilation.

pub const SCALE_ADD_WGSL: &str = include_str!("../shaders/scale_add.wgsl");
pub const ZIP_MUL_WGSL: &str = include_str!("../shaders/zip_mul.wgsl");
pub const BLOCK_SUM_WGSL: &str = include_str!("../shaders/block_sum.wgsl");
/// vec4 variants: 16 bytes streamed per thread instead of 4.
pub const SCALE_ADD4_WGSL: &str = include_str!("../shaders/scale_add4.wgsl");
pub const ZIP_MUL4_WGSL: &str = include_str!("../shaders/zip_mul4.wgsl");

pub const UNARY_WGSL: &str = include_str!("../shaders/unary.wgsl");
pub const AFFINE_WGSL: &str = include_str!("../shaders/affine.wgsl");
pub const BINARY_WGSL: &str = include_str!("../shaders/binary.wgsl");
pub const COMPARE_WGSL: &str = include_str!("../shaders/compare.wgsl");

pub const SCAN_BLOCKS_WGSL: &str = include_str!("../shaders/scan_blocks.wgsl");
/// Four consecutive elements per lane; block capacity is 4 * workgroup size.
pub const SCAN_BLOCKS4_WGSL: &str = include_str!("../shaders/scan_blocks4.wgsl");
pub const SCAN_ADD_WGSL: &str = include_str!("../shaders/scan_add.wgsl");
pub const COMPACT_SCATTER_WGSL: &str = include_str!("../shaders/compact_scatter.wgsl");
pub const COMPACT_SCATTER_LOCAL_WGSL: &str = include_str!("../shaders/compact_scatter_local.wgsl");
pub const COMPACT_FINISH_WGSL: &str = include_str!("../shaders/compact_finish.wgsl");

/// Fixed 32x32 tile geometry; workgroup tuning does not apply.
pub const MATMUL_WGSL: &str = include_str!("../shaders/matmul.wgsl");

/// Requires all matrix dimensions to be positive multiples of 32.
pub const MATMUL_ALIGNED_WGSL: &str = include_str!("../shaders/matmul_aligned.wgsl");
pub const MATMUL_DIRECT_WGSL: &str = include_str!("../shaders/matmul_direct.wgsl");
pub const MATMUL_DOT_WGSL: &str = include_str!("../shaders/matmul_dot.wgsl");

/// Every shipped kernel (name, source), for validation tests.
pub const ALL: [(&str, &str); 19] = [
    ("scan_blocks", SCAN_BLOCKS_WGSL),
    ("scan_blocks4", SCAN_BLOCKS4_WGSL),
    ("scan_add", SCAN_ADD_WGSL),
    ("compact_scatter", COMPACT_SCATTER_WGSL),
    ("compact_scatter_local", COMPACT_SCATTER_LOCAL_WGSL),
    ("compact_finish", COMPACT_FINISH_WGSL),
    ("unary", UNARY_WGSL),
    ("affine", AFFINE_WGSL),
    ("binary", BINARY_WGSL),
    ("compare", COMPARE_WGSL),
    ("matmul", MATMUL_WGSL),
    ("matmul_aligned", MATMUL_ALIGNED_WGSL),
    ("matmul_direct", MATMUL_DIRECT_WGSL),
    ("matmul_dot", MATMUL_DOT_WGSL),
    ("scale_add", SCALE_ADD_WGSL),
    ("zip_mul", ZIP_MUL_WGSL),
    ("block_sum", BLOCK_SUM_WGSL),
    ("scale_add4", SCALE_ADD4_WGSL),
    ("zip_mul4", ZIP_MUL4_WGSL),
];
