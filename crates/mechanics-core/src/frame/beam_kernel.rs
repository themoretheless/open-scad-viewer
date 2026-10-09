use super::*;

/// Hermite cubics on [0,1]; rotation rows carry the member length factor.
pub(super) fn hermite(xi: f64, l: f64) -> [f64; 4] {
    let xi2 = xi * xi;
    let xi3 = xi2 * xi;
    [
        1. - 3. * xi2 + 2. * xi3,
        l * (xi - 2. * xi2 + xi3),
        3. * xi2 - 2. * xi3,
        l * (xi3 - xi2),
    ]
}

/// d/dξ of the Hermite cubics (rotation rows carry the length factor).
pub(super) fn hermite_dxi(xi: f64, l: f64) -> [f64; 4] {
    [
        -6. * xi + 6. * xi * xi,
        l * (1. - 4. * xi + 3. * xi * xi),
        6. * xi - 6. * xi * xi,
        l * (-2. * xi + 3. * xi * xi),
    ]
}

/// 4-point Gauss–Legendre nodes and weights on [0,1]; exact through degree 7.
const GAUSS_XI: [f64; 4] = [
    0.069_431_844_202_973_55,
    0.330_009_478_207_571_87,
    0.669_990_521_792_428_1,
    0.930_568_155_797_026_4,
];
const GAUSS_W: [f64; 4] = [
    0.173_927_422_568_726_92,
    0.326_072_577_431_273_05,
    0.326_072_577_431_273_05,
    0.173_927_422_568_726_92,
];

/// Local 12x12 Timoshenko stiffness, DOFs [u,v,w,rx,ry,rz] per node.
/// Standard 3D frame element (Logan): θy couples to w with flipped signs
/// because θy = −dw/dx while θz = +dv/dx.
#[allow(clippy::too_many_arguments)]
pub(super) fn beam_stiffness(
    e: f64,
    g: f64,
    a: f64,
    iyy: f64,
    izz: f64,
    j: f64,
    asy: Option<f64>,
    asz: Option<f64>,
    l: f64,
) -> DMatrix<f64> {
    let mut k = DMatrix::<f64>::zeros(12, 12);
    let axial = e * a / l;
    k[(0, 0)] = axial;
    k[(0, 6)] = -axial;
    k[(6, 0)] = -axial;
    k[(6, 6)] = axial;
    let torsion = g * j / l;
    k[(3, 3)] = torsion;
    k[(3, 9)] = -torsion;
    k[(9, 3)] = -torsion;
    k[(9, 9)] = torsion;
    // Bending about local z, DOFs [v1, rz1, v2, rz2].
    let phi_z = asy.map_or(0., |as_| 12. * e * izz / (g * as_ * l * l));
    let cz = e * izz / (l * l * l * (1. + phi_z));
    let bz = [
        [12., 6. * l, -12., 6. * l],
        [6. * l, (4. + phi_z) * l * l, -6. * l, (2. - phi_z) * l * l],
        [-12., -6. * l, 12., -6. * l],
        [6. * l, (2. - phi_z) * l * l, -6. * l, (4. + phi_z) * l * l],
    ];
    let dz = [1usize, 5, 7, 11];
    for i in 0..4 {
        for jj in 0..4 {
            k[(dz[i], dz[jj])] = cz * bz[i][jj];
        }
    }
    // Bending about local y, DOFs [w1, ry1, w2, ry2].
    let phi_y = asz.map_or(0., |as_| 12. * e * iyy / (g * as_ * l * l));
    let cy = e * iyy / (l * l * l * (1. + phi_y));
    let by = [
        [12., -6. * l, -12., -6. * l],
        [-6. * l, (4. + phi_y) * l * l, 6. * l, (2. - phi_y) * l * l],
        [-12., 6. * l, 12., 6. * l],
        [-6. * l, (2. - phi_y) * l * l, 6. * l, (4. + phi_y) * l * l],
    ];
    let dy = [2usize, 4, 8, 10];
    for i in 0..4 {
        for jj in 0..4 {
            k[(dy[i], dy[jj])] = cy * by[i][jj];
        }
    }
    k
}

/// Add the v/w bending rows of a transverse load contribution.
/// θy rows flip sign because θy = −dw/dx.
pub(super) fn add_transverse(f: &mut DVector<f64>, xi: f64, l: f64, py: f64, pz: f64) {
    let h = hermite(xi, l);
    f[1] += py * h[0];
    f[5] += py * h[1];
    f[7] += py * h[2];
    f[11] += py * h[3];
    f[2] += pz * h[0];
    f[4] -= pz * h[1];
    f[8] += pz * h[2];
    f[10] -= pz * h[3];
}

/// Work-equivalent nodal loads of one member load, local axes, 12-vector.
pub(super) fn equivalent_load(load: &LocalLoad, l: f64, f: &mut DVector<f64>) {
    match load {
        LocalLoad::PointForce { at_mm, force } => {
            let xi = at_mm / l;
            f[0] += force.x * (1. - xi);
            f[6] += force.x * xi;
            add_transverse(f, xi, l, force.y, force.z);
        }
        LocalLoad::PointMoment { at_mm, moment } => {
            let xi = at_mm / l;
            f[3] += moment.x * (1. - xi);
            f[9] += moment.x * xi;
            let hd = hermite_dxi(xi, l);
            // θz = dv/dx: work m_z·θz(a).
            f[1] += moment.z * hd[0] / l;
            f[5] += moment.z * hd[1] / l;
            f[7] += moment.z * hd[2] / l;
            f[11] += moment.z * hd[3] / l;
            // θy = −dw/dx with w = H·[w, −θy]: work m_y·θy(a).
            f[2] -= moment.y * hd[0] / l;
            f[4] += moment.y * hd[1] / l;
            f[8] -= moment.y * hd[2] / l;
            f[10] += moment.y * hd[3] / l;
        }
        LocalLoad::Uniform { w } => {
            for i in 0..4 {
                let (xi, weight) = (GAUSS_XI[i], GAUSS_W[i] * l);
                f[0] += w.x * (1. - xi) * weight;
                f[6] += w.x * xi * weight;
                add_transverse(f, xi, l, w.y * weight, w.z * weight);
            }
        }
        LocalLoad::Trapezoidal { w_a, w_b } => {
            for i in 0..4 {
                let (xi, weight) = (GAUSS_XI[i], GAUSS_W[i] * l);
                let p = *w_a + (*w_b - *w_a) * xi;
                f[0] += p.x * (1. - xi) * weight;
                f[6] += p.x * xi * weight;
                add_transverse(f, xi, l, p.y * weight, p.z * weight);
            }
        }
    }
}

pub(super) fn submatrix(k: &DMatrix<f64>, rows: &[usize], cols: &[usize]) -> DMatrix<f64> {
    DMatrix::from_fn(rows.len(), cols.len(), |i, j| k[(rows[i], cols[j])])
}

pub(super) fn subvector(f: &DVector<f64>, rows: &[usize]) -> DVector<f64> {
    DVector::from_iterator(rows.len(), rows.iter().map(|&i| f[i]))
}

/// Block-diagonal 12x12 local→global transform (R on all four 3-blocks).
pub(super) fn transformation(r: &Matrix3<f64>) -> DMatrix<f64> {
    let mut t = DMatrix::<f64>::zeros(12, 12);
    for block in 0..4 {
        for i in 0..3 {
            for j in 0..3 {
                t[(block * 3 + i, block * 3 + j)] = r[(i, j)];
            }
        }
    }
    t
}

/// Member local axes: x along the member, z from the hint, y = z × x.
pub(super) fn local_frame(x_axis: Vector3<f64>, hint: [f64; 3]) -> Result<Matrix3<f64>> {
    let hint = Vector3::from(hint);
    if !hint.iter().all(|v| v.is_finite()) || hint.norm() == 0. {
        return Err(invalid("Local z hint must be finite and nonzero"));
    }
    let mut z = hint - x_axis * hint.dot(&x_axis);
    if z.norm() <= 1e-12 * hint.norm() {
        // Hint parallel to the member: deterministic fallback.
        let fallback = if x_axis.y.abs() < 0.9 {
            Vector3::new(0., 1., 0.)
        } else {
            Vector3::new(1., 0., 0.)
        };
        z = fallback - x_axis * fallback.dot(&x_axis);
    }
    let z_axis = z.normalize();
    let y_axis = z_axis.cross(&x_axis);
    Ok(Matrix3::from_columns(&[x_axis, y_axis, z_axis]))
}
