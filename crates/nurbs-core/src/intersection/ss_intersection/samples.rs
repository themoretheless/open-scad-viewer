//! Native predictor/corrector continuation samples.
pub struct ContinuationSample {
    pub point: [f64; 3],
    pub uv_first: [f64; 2],
    pub uv_second: [f64; 2],
    pub parameter: f64,
    pub seam_wrap: Option<[[i32; 2]; 2]>,
}
