//! Trial-basis quadratic shell on the UNCHANGED original reference metric.
//! Affine offsets retain original rest jets when the trial anchor is different.
use super::*;
pub(super) struct Assembly {
    pub h: Vec<Vec<f64>>,
    pub elastic: Vec<Vec<f64>>,
    pub force: Vec<f64>,
    pub rhs: Vec<f64>,
    pub rest: Vec<f64>,
    pub elastic_constant: f64,
    pub load_constant: f64,
    pub sites: usize,
    pub terms: u64,
}
/// Elastic quadratic `0.5 xᵀHx - restᵀx + constant` accumulated one strain row at a time.
struct ElasticForm {
    h: Vec<Vec<f64>>,
    rest: Vec<f64>,
    constant: f64,
}
impl ElasticForm {
    fn add(
        &mut self,
        row: &[f64],
        target: f64,
        weight: f64,
        work: &mut u64,
        max: u64,
    ) -> Result<()> {
        let Self { h, rest, constant } = self;
        let active = row
            .iter()
            .enumerate()
            .filter(|(_, x)| **x != 0.)
            .collect::<Vec<_>>();
        if !weight.is_finite()
            || weight < 0.
            || !target.is_finite()
            || active.iter().any(|(_, x)| !x.is_finite())
        {
            return Err(error("Shell affine strain assembly overflow"));
        }
        let terms = active.len() as u64 * active.len() as u64;
        if terms > max.saturating_sub(*work) {
            return Err(error("Shell assembly work allowance exhausted"));
        }
        *work += terms;
        *constant += 0.5 * weight * target * target;
        for &(i, &a) in &active {
            rest[i] += weight * a * target;
            for &(j, &b) in &active {
                h[i][j] += weight * a * b;
            }
        }
        Ok(())
    }
}
fn displacement(trial: &Surface, reference: &Surface, uv: [f64; 2]) -> Result<[f64; 3]> {
    let a = trial.evaluate(uv[0], uv[1])?.point;
    let b = reference.evaluate(uv[0], uv[1])?.point;
    Ok(std::array::from_fn(|k| a[k] - b[k]))
}
pub(super) fn assemble(
    seed: &Surface,
    reference: &Surface,
    m: &Model,
    o: Options,
) -> Result<Assembly> {
    let n = seed.control_points.len() * seed.control_points[0].len();
    let sites = quadrature::sites(seed, reference, o)?;
    let mut form = ElasticForm {
        h: vec![vec![0.; 3 * n]; 3 * n],
        rest: vec![0.; 3 * n],
        constant: 0.,
    };
    let mut force = vec![0.; 3 * n];
    let mut load_constant = 0.;
    let mut terms = 0;
    let nu = m.material.poisson;
    for &(uv, weight) in &sites {
        let frame = mechanics::Frame::at(reference, uv)?;
        let jets = super::super::jet::coefficient_jets(seed, uv, true)?;
        let offset = frame.offset(seed, reference, uv)?;
        let area = weight * frame.area;
        if !area.is_finite() || area <= 0. {
            return Err(error("Original reference area quadrature unresolved"));
        }
        let load: [f64; 3] = std::array::from_fn(|k| m.traction[k] + m.pressure * frame.normal[k]);
        load_constant += area * dot(&load, &displacement(seed, reference, uv)?);
        for k in 0..3 {
            for (i, r) in jets.iter().enumerate() {
                force[k * n + i] += area * r[0] * load[k];
            }
        }
        for ((rows, off), stiffness) in frame
            .strains(&jets)
            .iter()
            .zip(offset)
            .zip([m.material.membrane, m.material.bending])
        {
            if stiffness == 0. {
                continue;
            }
            let weight = area * stiffness / (1. - nu * nu);
            let sum = rows[0]
                .iter()
                .zip(&rows[1])
                .map(|(a, b)| a + b)
                .collect::<Vec<_>>();
            let diff = rows[0]
                .iter()
                .zip(&rows[1])
                .map(|(a, b)| a - b)
                .collect::<Vec<_>>();
            for (row, target, w) in [
                (&sum, -off[0] - off[1], weight * (1. + nu) * 0.5),
                (&diff, -off[0] + off[1], weight * (1. - nu) * 0.5),
                (&rows[2], -off[2], weight * 2. * (1. - nu)),
            ] {
                form.add(row, target, w, &mut terms, o.max_assembly_terms)?;
            }
        }
    }
    for p in &m.points {
        let row = super::super::rational_basis(seed, p.uv, None)?;
        load_constant += dot(&p.force, &displacement(seed, reference, p.uv)?);
        for k in 0..3 {
            for (i, b) in row.iter().enumerate() {
                force[k * n + i] += b * p.force[k];
            }
        }
    }
    let ElasticForm {
        h: elastic,
        rest,
        constant: elastic_constant,
    } = form;
    let mut h = elastic.clone();
    for (i, r) in h.iter_mut().enumerate() {
        r[i] += o.fitting.anchor_weight;
    }
    let rhs = force
        .iter()
        .zip(&rest)
        .map(|(f, r)| f + r)
        .collect::<Vec<_>>();
    if h.iter()
        .flatten()
        .chain(force.iter())
        .chain(&rest)
        .chain(&rhs)
        .any(|x| !x.is_finite())
        || !elastic_constant.is_finite()
        || !load_constant.is_finite()
    {
        return Err(error("Loaded shell affine matrix overflow"));
    }
    Ok(Assembly {
        h,
        elastic,
        force,
        rhs,
        rest,
        elastic_constant,
        load_constant,
        sites: sites.len(),
        terms,
    })
}
