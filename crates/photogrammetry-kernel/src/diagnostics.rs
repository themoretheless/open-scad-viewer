//! Data-only diagnostics; neither the numerical core nor this report depends on a UI or clock.
#[derive(Clone, Debug, Default)]
pub struct ImageReport {
    pub image: usize,
    pub features: usize,
    pub candidate_correspondences: usize,
    pub accepted_observations: usize,
    pub conflicting_matches: usize,
    pub pose_attempts: usize,
    pub registered: bool,
    pub reason: &'static str,
}
#[derive(Clone, Debug, Default)]
pub struct ReconstructionReport {
    pub images: Vec<ImageReport>,
    pub initial_pair: Option<[usize; 2]>,
    pub seed_pairs_tested: usize,
    pub seed_trials: Vec<SeedTrialReport>,
    pub matching_requests: usize,
    pub computed_pairs: usize,
    pub bundle_runs: Vec<crate::bundle::BundleReport>,
    pub warnings: Vec<String>,
}

/// Each bounded initialization attempt, including failures; not a ground-truth quality score.
#[derive(Clone, Debug)]
pub struct SeedTrialReport {
    pub pair: [usize; 2],
    pub registered_images: usize,
    pub points: usize,
    pub reprojection_rmse: Option<f64>,
    pub error: Option<String>,
}
