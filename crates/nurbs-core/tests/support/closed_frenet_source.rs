//! Exact rational spatial C4 closed path; no sampled closure premise.
use super::nurbs;
use nurbs::curve::Curve;
pub fn path()->Curve {
    let points=[[0.,0.,0.],[1.,0.,0.],[2.,1.,0.],[3.,3.,1.],[4.,6.,4.],
        [2.,7.,2.],[-2.,7.,-2.],[-4.,6.,-4.],[-3.,3.,-1.],[-2.,1.,0.],[-1.,0.,0.],[0.,0.,0.]];
    Curve{degree:11,knots:[vec![-3.;12],vec![5.;12]].concat(),
        control_points:points.map(|p|vec![p[0],p[1],p[2]/64.]).to_vec(),
        weights:vec![1.,1.,1.,1.,1.,1.125,1.125,1.,1.,1.,1.,1.],periodic:false}
}
pub fn twice()->Curve {
    let mut p=path();let points=p.control_points.clone();let weights=p.weights.clone();
    p.control_points.extend(points.into_iter().skip(1));p.weights.extend(weights.into_iter().skip(1));
    p.knots=[vec![0.;12],vec![1.;11],vec![2.;12]].concat();p
}
/// Exact positive Mobius clock, preserving every original geometric point.
pub fn reclocked_path()->Curve {
    let mut p=path();for (i,w) in p.weights.iter_mut().enumerate(){*w*=2f64.powi(i as i32);}p
}
/// Exact dyadic half split of the polynomial source, then a positive clock
/// change on the second half. The original internal knot has a speed jump.
pub fn split_reclocked_path()->Curve {
    let mut p=path();let mut row=p.control_points.clone();let mut left=vec![row[0].clone()];let mut right=vec![row.last().unwrap().clone()];
    for width in (1..row.len()).rev(){for i in 0..width {for k in 0..3 {row[i][k]=(row[i][k]+row[i+1][k])*0.5;}}
        left.push(row[0].clone());right.push(row[width-1].clone());}
    right.reverse();left.extend(right.into_iter().skip(1));p.control_points=left;
    p.weights=[vec![1.;12],(1..12).map(|i|2f64.powi(i)).collect()].concat();
    p.knots=[vec![-3.;12],vec![1.;11],vec![5.;12]].concat();p
}

/// Uniform degree-five active basis with exact periodic endpoint jets.
pub fn periodic_path() -> Curve {
    let mut points = vec![
        vec![4., -3.5, 0.],
        vec![3., -0.5, 1. / 64.],
        vec![0., 0.5, 0.],
        vec![-3., -0.5, -1. / 64.],
        vec![-4., -3.5, 0.],
        vec![-3., -6.5, 1. / 64.],
        vec![0., -7.5, 0.],
        vec![3., -6.5, -1. / 64.],
    ];
    points.extend_from_within(..5);
    Curve {
        degree: 5,
        knots: (0..19).map(|i| (f64::from(i) - 5.) / 8.).collect(),
        control_points: points,
        weights: vec![1.; 13],
        periodic: true,
    }
}

/// The authored axis is a periodic degree-four spline, with a deliberate
/// axial tilt relative to the derivative of the degree-five source path.
pub fn periodic_authored_axis(path: &Curve) -> Curve {
    let control_points: Vec<_> = path
        .control_points
        .windows(2)
        .map(|p| vec![
            8. * (p[1][0] - p[0][0]),
            8. * (p[1][1] - p[0][1]),
            8. * (p[1][2] - p[0][2]) + 0.25,
        ])
        .collect();
    Curve {
        degree: 4,
        knots: path.knots[1..path.knots.len() - 1].to_vec(),
        weights: vec![1.; control_points.len()],
        control_points,
        periodic: true,
    }
}
