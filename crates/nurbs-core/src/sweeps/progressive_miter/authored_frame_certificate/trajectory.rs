//! Original-law control trajectory jets. Local coordinate intervals are an
//! explicit premise; this does not prove retained endpoint rounding or caps.
use super::*;

/// Owning caller proves and charges a constant original relative pose once.
/// Every query still certifies the original path at its own traversal.
pub(crate) fn translated_constant_values(path:&Curve,pose:&[[[f64;2];3]],traversal:[f64;2],max_cells:usize)->Result<ControlValuesReport>{
    let p=vector_certificate::certify_values_traversal(path,traversal,max_cells,false)?;
    let mut out=ControlValuesReport {status:Status::Unresolved,cells:p.cells,values:None,reason:Some("path-value-unresolved")};
    let Some(value)=p.value else{return Ok(out);};
    out.values=Some(pose.iter().map(|q|add(decode(value)?,decode(*q)?).map(|v|v.map(|x|[x.lo,x.hi]))).collect::<Result<Vec<_>>>()?);
    out.status=Status::Certified;out.reason=None;Ok(out)
}

pub(crate) fn translated_constant_jets(path:&Curve,pose:&[[[f64;2];3]],traversal:[f64;2],max_cells:usize)->Result<TrajectoriesReport>{
    let p=vector_certificate::certify_traversal(path,traversal,max_cells,false)?;
    let mut out=TrajectoriesReport {traversal,status:Status::Unresolved,cells:p.cells,jets:None,single_span:false,reason:Some("path-law-enclosure-unresolved")};
    if p.status!=Status::Certified {return Ok(out);}
    let source=jet(path,&p)?;
    out.jets=Some(pose.iter().map(|q|Ok(FrameJet {value:add(source.v,decode(*q)?)?.map(|x|[x.lo,x.hi]),
        first:source.d.map(|x|[x.lo,x.hi]),second:source.dd.map(|x|[x.lo,x.hi])})).collect::<Result<Vec<_>>>()?);
    out.single_span=p.single_span;out.status=Status::Certified;out.reason=None;Ok(out)
}

#[derive(Clone, Debug)]
pub struct TrajectoryReport {
    pub traversal: [f64; 2],
    pub status: Status,
    pub cells: usize,
    pub jet: Option<FrameJet>,
    pub single_span: bool,
    pub reason: Option<&'static str>,
}
impl TrajectoryReport {
    /// Conditional first-derivative interpolation error. The caller must prove
    /// continuity on the open traversal span, and endpoint displacements must
    /// enclose its one-sided limits. No knot continuity is inferred from jets.
    pub(crate) fn linear_first_error_upper_on_continuous_span(&self,endpoints:[[f64;3];2],endpoint_displacements:[f64;2])->Result<Option<f64>> {
        check(endpoints.iter().flatten().all(|x|x.is_finite())&&endpoint_displacements.iter().all(|x|x.is_finite()&&*x>=0.),"Invalid retained interpolation endpoints")?;
        if self.status!=Status::Certified {return Ok(None);}
        let Some(jet)=&self.jet else{return Ok(None);};
        let width=I::point(self.traversal[1]).sub(I::point(self.traversal[0]))?;
        if width.lo<=0. {return Ok(None);}
        let mut defect=[I::point(0.);3];
        for k in 0..3 {
            let slope=I::point(endpoints[1][k]).sub(I::point(endpoints[0][k]))?.div(width)?;
            defect[k]=I::new(jet.first[k][0],jet.first[k][1])?.sub(slope)?;
        }
        let derivative=crate::numerics::interval_vec3::norm(defect)?;
        // Integrate from the nearer endpoint: distance <= width/2. Its
        // displacement plus the defect integral bounds every interior point.
        Ok(Some(I::point(endpoint_displacements[0].max(endpoint_displacements[1]))
            .add(derivative.mul(width)?.div(I::point(2.))?)?.hi))
    }
    /// Conditional h²/8 interpolation bound. The caller proves C2 on the
    /// open span and finite one-sided endpoint limits enclosed by the given
    /// displacements. Endpoint derivative agreement is not required: extend
    /// the interior Green-kernel inequality by its one-sided limits.
    pub(crate) fn linear_second_error_upper_on_open_span(
        &self, endpoint_displacements:[f64;2],
    )->Result<Option<f64>> {
        check(endpoint_displacements.iter().all(|x|x.is_finite()&&*x>=0.),
            "Endpoint displacement must be finite and nonnegative")?;
        if self.status!=Status::Certified {return Ok(None);}
        let Some(jet)=&self.jet else{return Ok(None);};
        let width=I::point(self.traversal[1]).sub(I::point(self.traversal[0]))?;
        if width.lo<=0. {return Ok(None);}
        let second=crate::numerics::interval_vec3::norm(decode(jet.second)?)?;
        Ok(Some(width.mul(width)?.mul(second)?.div(I::point(8.))?
            .add(I::point(endpoint_displacements[0].max(endpoint_displacements[1])))?.hi))
    }
    /// Original control-image bound already carried by the jet certificate.
    /// Covers knot transitions as well as a smooth single-span remainder.
    pub fn retained_segment_displacement_upper(&self, endpoints: [[f64;3];2]) -> Result<Option<f64>> {
        ControlValueReport {status:self.status,cells:self.cells,value:self.jet.as_ref().map(|jet|jet.value),reason:self.reason}
            .retained_segment_displacement_upper(endpoints)
    }
    /// Linear interpolation remainder on this report's own traversal interval.
    /// A single original span gives smooth original laws and a regular frame;
    /// unproved transitions across knots do not supply a remainder.
    pub fn linear_remainder_upper(&self) -> Result<Option<f64>> {
        if self.status != Status::Certified || !self.single_span {
            return Ok(None);
        }
        let Some(jet) = &self.jet else {
            return Ok(None);
        };
        let width = I::point(self.traversal[1]).sub(I::point(self.traversal[0]))?;
        let second = crate::numerics::interval_vec3::norm(decode(jet.second)?)?;
        Ok(Some(width.mul(width)?.mul(second)?.div(I::point(8.))?.hi))
    }
    /// Endpoint displacement contributes by convex interpolation of the two
    /// endpoint errors. The caller must establish retained endpoint ownership.
    pub fn linear_error_upper(&self, endpoint_displacements: [f64; 2]) -> Result<Option<f64>> {
        check(
            endpoint_displacements
                .iter()
                .all(|x| x.is_finite() && *x >= 0.),
            "Endpoint displacement must be finite and nonnegative",
        )?;
        let Some(remainder) = self.linear_remainder_upper()? else {
            return Ok(None);
        };
        Ok(Some(
            I::point(remainder)
                .add(I::point(
                    endpoint_displacements[0].max(endpoint_displacements[1]),
                ))?
                .hi,
        ))
    }
}
type ScalarJet = [I; 3];
#[derive(Clone, Debug)]
pub struct ControlValueReport {
    pub status: Status,
    pub cells: usize,
    pub value: Option<[[f64; 2]; 3]>,
    pub reason: Option<&'static str>,
}
impl ControlValueReport {
    /// Conservative pointwise bound against any convex interpolation of the
    /// retained endpoints. The caller owns the same traversal correspondence.
    /// Unlike a second-derivative remainder this also covers knot transitions.
    pub fn retained_segment_displacement_upper(
        &self,
        endpoints: [[f64; 3]; 2],
    ) -> Result<Option<f64>> {
        check(
            endpoints.iter().flatten().all(|x| x.is_finite()),
            "Retained segment must be finite",
        )?;
        if self.status != Status::Certified {
            return Ok(None);
        }
        let Some(value) = self.value else {
            return Ok(None);
        };
        let segment = [
            I::new(
                endpoints[0][0].min(endpoints[1][0]),
                endpoints[0][0].max(endpoints[1][0]),
            )?,
            I::new(
                endpoints[0][1].min(endpoints[1][1]),
                endpoints[0][1].max(endpoints[1][1]),
            )?,
            I::new(
                endpoints[0][2].min(endpoints[1][2]),
                endpoints[0][2].max(endpoints[1][2]),
            )?,
        ];
        Ok(Some(
            crate::numerics::interval_vec3::norm(sub(segment, decode(value)?)?)?.hi,
        ))
    }
    /// Outward Euclidean displacement from a retained binary64 pole to any
    /// ideal control value enclosed by this report. Unresolved reports supply
    /// no bound; this does not infer a correspondence between unrelated poles.
    pub fn retained_displacement_upper(&self, retained: [f64; 3]) -> Result<Option<f64>> {
        check(
            retained.iter().all(|x| x.is_finite()),
            "Retained control must be finite",
        )?;
        if self.status != Status::Certified {
            return Ok(None);
        }
        let Some(value) = self.value else {
            return Ok(None);
        };
        let delta = sub(retained.map(I::point), decode(value)?)?;
        Ok(Some(crate::numerics::interval_vec3::norm(delta)?.hi))
    }
}
/// Value-only restriction, including exact traversal endpoints. It avoids
/// derivative division on subnormal-width point restrictions. No jets are
/// inferred from these value enclosures.
pub fn certify_control_value(
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    longitudinal: &Curve,
    transverse: &Curve,
    affine: Option<(&Curve, &Curve)>,
    q: [[f64; 2]; 3],
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<ControlValueReport> {
    let frame = certify_twisted_values(longitudinal, transverse, twist, traversal, max_cells)?;
    control_value_from_frame(path, scale, affine, q, traversal, max_cells, frame)
}
/// Original authored source premises are shared once across all controls.
pub fn certify_control_values(
    path:&Curve,scale:&Curve,twist:&Curve,longitudinal:&Curve,transverse:&Curve,
    affine:Option<(&Curve,&Curve)>,qs:&[[[f64;2];3]],traversal:[f64;2],max_cells:usize,
)->Result<ControlValuesReport>{
    let frame=certify_twisted_values(longitudinal,transverse,twist,traversal,max_cells)?;
    control_values_with_fit(path,scale,affine,qs,traversal,max_cells,frame,None)
}
pub fn certify_control_trajectories(
    path:&Curve,scale:&Curve,twist:&Curve,longitudinal:&Curve,transverse:&Curve,
    affine:Option<(&Curve,&Curve)>,qs:&[[[f64;2];3]],traversal:[f64;2],max_cells:usize,
)->Result<TrajectoriesReport>{
    let frame=certify_twisted(longitudinal,transverse,twist,traversal,max_cells)?;
    control_trajectories_with_fit(path,scale,affine,qs,traversal,max_cells,frame,None)
}
/// Shared original-law composition; frame evidence is produced by native owners.
pub(super) fn control_value_from_frame(
    path: &Curve,
    scale: &Curve,
    affine: Option<(&Curve, &Curve)>,
    q: [[f64; 2]; 3],
    traversal: [f64; 2],
    max_cells: usize,
    frame: ValuesReport,
) -> Result<ControlValueReport> {
    control_value_with_fit(path, scale, affine, q, traversal, max_cells, frame, None)
}
#[derive(Clone,Debug)]
pub struct ControlValuesReport {
    pub status:Status,
    /// Shared original-source work, charged once for the whole batch.
    pub cells:usize,
    pub values:Option<Vec<[[f64;2];3]>>,
    pub reason:Option<&'static str>,
}
pub(super) fn control_value_with_fit(
    path:&Curve,scale:&Curve,affine:Option<(&Curve,&Curve)>,q:[[f64;2];3],
    traversal:[f64;2],max_cells:usize,frame:ValuesReport,transverse_fit:Option<I>,
)->Result<ControlValueReport>{
    let batch=control_values_with_fit(path,scale,affine,&[q],traversal,max_cells,frame,transverse_fit)?;
    Ok(ControlValueReport {status:batch.status,cells:batch.cells,value:batch.values.map(|v|v[0]),reason:batch.reason})
}
pub(super) fn control_values_with_fit(
    path:&Curve,scale:&Curve,affine:Option<(&Curve,&Curve)>,qs:&[[[f64;2];3]],
    traversal:[f64;2],max_cells:usize,frame:ValuesReport,transverse_fit:Option<I>,
)->Result<ControlValuesReport>{
    control_values_with_prepared_path(path,scale,affine,qs,traversal,max_cells,frame,transverse_fit,None)
}
/// Cached position must belong to this same original path and traversal; its
/// source work is already included in frame.cells by the owning fit caller.
pub(super) fn control_values_with_prepared_path(
    path:&Curve,scale:&Curve,affine:Option<(&Curve,&Curve)>,qs:&[[[f64;2];3]],
    traversal:[f64;2],max_cells:usize,frame:ValuesReport,transverse_fit:Option<I>,
    prepared_path:Option<[[f64;2];3]>,
)->Result<ControlValuesReport>{
    check(
        max_cells <= 100000 && frame.cells <= max_cells,
        "Control value work exceeds budget",
    )?;
    check(!qs.is_empty() && qs.len()<=32768,"Invalid control value batch size")?;
    let coordinates=qs.iter().copied().map(decode).collect::<Result<Vec<_>>>()?;
    super::super::validate_law(scale, true)?;
    let mut out = ControlValuesReport {
        status: Status::Unresolved,
        cells: frame.cells,
        values: None,
        reason: Some("frame-value-unresolved"),
    };
    if frame.status != Status::Certified {
        return Ok(out);
    }
    out.reason = Some("path-value-unresolved");
    let path_value=if let Some(value)=prepared_path {value} else {
        let source=vector_certificate::certify_values_traversal(path,traversal,max_cells-out.cells,false)?;
        out.cells+=source.cells;
        let Some(value)=source.value else {return Ok(out);};
        value
    };
    out.reason = Some("scale-value-unresolved");
    let charge = (scale.degree..scale.control_points.len())
        .filter(|&i| scale.knots[i] < scale.knots[i + 1])
        .count();
    if charge > max_cells - out.cells {
        return Ok(out);
    }
    let uniform = super::super::scalar_certificate::value_traversal(scale, traversal, charge)?;
    out.cells += charge;
    let Some(uniform) = uniform else {
        return Ok(out);
    };
    let uniform = I::new(uniform[0], uniform[1])?;
    let mut axes = [I::point(1.); 3];
    let mut center = [I::point(0.); 3];
    if let Some((axis_scale, center_law)) = affine {
        out.reason = Some("affine-axis-value-unresolved");
        let a = vector_certificate::certify_values_traversal(
            axis_scale,
            traversal,
            max_cells - out.cells,
            true,
        )?;
        out.cells += a.cells;
        let Some(a) = a.value else {
            return Ok(out);
        };
        axes = decode(a)?;
        out.reason = Some("affine-center-value-unresolved");
        let c = vector_certificate::certify_values_traversal(
            center_law,
            traversal,
            max_cells - out.cells,
            false,
        )?;
        out.cells += c.cells;
        let Some(c) = c.value else {
            return Ok(out);
        };
        center = decode(c)?;
    }
    let directions = [
        decode(frame.transverse.unwrap())?,
        decode(frame.binormal.unwrap())?,
        decode(frame.longitudinal.unwrap())?,
    ];
    let mut values=Vec::with_capacity(coordinates.len());
    for q in coordinates {
        let mut value = decode(path_value)?;
        for j in 0..3 {
            let mut coefficient = uniform.mul(axes[j])?.mul(q[j])?.add(center[j])?;
            if j == 0 {
                if let Some(fit) = transverse_fit { coefficient = coefficient.mul(fit)?; }
            }
            value = add(value,mul(directions[j], coefficient)?)?;
        }
        values.push(value.map(|x|[x.lo,x.hi]));
    }
    out.status = Status::Certified;
    out.reason = None;
    out.values = Some(values);
    Ok(out)
}
pub(super) fn product(a: ScalarJet, b: ScalarJet) -> Result<ScalarJet> {
    Ok([
        a[0].mul(b[0])?,
        a[1].mul(b[0])?.add(a[0].mul(b[1])?)?,
        a[2].mul(b[0])?
            .add(a[1].mul(b[1])?.mul(I::point(2.))?)?
            .add(a[0].mul(b[2])?)?,
    ])
}
/// x = path + sum_j (uniform_scale * axis_scale_j * q_j + center_j) frame_j.
/// Each law maps its own active domain to the same normalized traversal.
/// `q` must enclose the ideal initial-profile coordinates. Work is shared by
/// every original-law restriction; unresolved work publishes no partial jet.
pub fn certify_control_trajectory(
    path: &Curve,
    scale: &Curve,
    twist: &Curve,
    longitudinal: &Curve,
    transverse: &Curve,
    affine: Option<(&Curve, &Curve)>,
    q: [[f64; 2]; 3],
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<TrajectoryReport> {
    let frame = certify_twisted(longitudinal, transverse, twist, traversal, max_cells)?;
    control_trajectory_from_frame(path, scale, affine, q, traversal, max_cells, frame)
}
pub(super) fn control_trajectory_from_frame(
    path: &Curve,
    scale: &Curve,
    affine: Option<(&Curve, &Curve)>,
    q: [[f64; 2]; 3],
    traversal: [f64; 2],
    max_cells: usize,
    frame: Report,
) -> Result<TrajectoryReport> {
    control_trajectory_with_fit(path, scale, affine, q, traversal, max_cells, frame, None)
}
#[derive(Clone,Debug)]
pub struct TrajectoriesReport {
    pub traversal:[f64;2],
    pub status:Status,
    /// Original source jets are shared by all profile controls.
    pub cells:usize,
    pub jets:Option<Vec<FrameJet>>,
    pub single_span:bool,
    pub reason:Option<&'static str>,
}
pub(super) fn control_trajectory_with_fit(
    path:&Curve,scale:&Curve,affine:Option<(&Curve,&Curve)>,q:[[f64;2];3],
    traversal:[f64;2],max_cells:usize,frame:Report,transverse_fit:Option<[I;3]>,
)->Result<TrajectoryReport>{
    let batch=control_trajectories_with_fit(path,scale,affine,&[q],traversal,max_cells,frame,transverse_fit)?;
    Ok(TrajectoryReport {traversal,status:batch.status,cells:batch.cells,jet:batch.jets.map(|mut v|v.remove(0)),single_span:batch.single_span,reason:batch.reason})
}
pub(super) fn control_trajectories_with_fit(
    path:&Curve,scale:&Curve,affine:Option<(&Curve,&Curve)>,qs:&[[[f64;2];3]],
    traversal:[f64;2],max_cells:usize,frame:Report,transverse_fit:Option<[I;3]>,
)->Result<TrajectoriesReport>{
    check(
        max_cells <= 100000 && frame.cells <= max_cells,
        "Control trajectory work exceeds budget",
    )?;
    check(!qs.is_empty()&&qs.len()<=32768,"Invalid control trajectory batch size")?;
    let coordinates=qs.iter().copied().map(decode).collect::<Result<Vec<_>>>()?;
    super::super::validate_law(scale, true)?;
    let mut out = TrajectoriesReport {
        traversal,
        status: Status::Unresolved,
        cells: frame.cells,
        jets: None,
        single_span: false,
        reason: frame.reason,
    };
    if frame.status != Status::Certified {
        return Ok(out);
    }
    out.reason = Some("path-law-enclosure-unresolved");
    let p = vector_certificate::certify_traversal(path, traversal, max_cells - out.cells, false)?;
    out.cells += p.cells;
    if p.status != Status::Certified {
        return Ok(out);
    }
    let path_jet = jet(path, &p)?;
    out.reason = Some("scale-law-enclosure-unresolved");
    let s = super::super::scalar_certificate::certify_traversal(
        scale,
        traversal,
        max_cells - out.cells,
    )?;
    out.cells += s.cells;
    if s.status != Status::Certified {
        return Ok(out);
    }
    let [lo, hi] = scale.domain();
    let width = I::point(hi).sub(I::point(lo))?;
    let uniform = [
        I::new(s.value.unwrap()[0], s.value.unwrap()[1])?,
        I::new(s.first.unwrap()[0], s.first.unwrap()[1])?.mul(width)?,
        I::new(s.second.unwrap()[0], s.second.unwrap()[1])?.mul(width.mul(width)?)?,
    ];
    let zero = I::point(0.);
    let mut axes = Jet {
        v: [I::point(1.); 3],
        d: [zero; 3],
        dd: [zero; 3],
    };
    let mut center = Jet {
        v: [zero; 3],
        d: [zero; 3],
        dd: [zero; 3],
    };
    let mut single_span = frame.single_span && p.single_span && s.single_span;
    if let Some((axis_scale, center_law)) = affine {
        out.reason = Some("affine-axis-law-enclosure-unresolved");
        let a = vector_certificate::certify_traversal(
            axis_scale,
            traversal,
            max_cells - out.cells,
            true,
        )?;
        out.cells += a.cells;
        if a.status != Status::Certified {
            return Ok(out);
        }
        axes = jet(axis_scale, &a)?;
        out.reason = Some("affine-center-law-enclosure-unresolved");
        let c = vector_certificate::certify_traversal(
            center_law,
            traversal,
            max_cells - out.cells,
            false,
        )?;
        out.cells += c.cells;
        if c.status != Status::Certified {
            return Ok(out);
        }
        center = jet(center_law, &c)?;
        single_span &= a.single_span && c.single_span;
    }
    let directions = [
        frame.transverse.unwrap(),
        frame.binormal.unwrap(),
        frame.longitudinal.unwrap(),
    ];
    let mut results=Vec::with_capacity(coordinates.len());
    for q in coordinates {
        let mut result=path_jet;
    for j in 0..3 {
        let coefficient = product(uniform, [axes.v[j], axes.d[j], axes.dd[j]])?;
        let mut coefficient = [
            coefficient[0].mul(q[j])?.add(center.v[j])?,
            coefficient[1].mul(q[j])?.add(center.d[j])?,
            coefficient[2].mul(q[j])?.add(center.dd[j])?,
        ];
        if j == 0 {
            if let Some(fit) = transverse_fit {
                coefficient = product(coefficient, fit)?;
            }
        }
        result = add_jet(
            result,
            scalar_product(
                decode_jet(&directions[j])?,
                coefficient[0],
                coefficient[1],
                coefficient[2],
            )?,
        )?;
    }
        results.push(encode(result));
    }
    out.status = Status::Certified;
    out.single_span = single_span;
    out.reason = None;
    out.jets = Some(results);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn line(a: [f64; 3], b: [f64; 3], domain: [f64; 2]) -> Curve {
        Curve {
            degree: 1,
            knots: vec![domain[0], domain[0], domain[1], domain[1]],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        }
    }
    #[test]
    fn simultaneous_path_scale_affine_and_frame_jets_use_independent_domains() {
        let path = line([0.; 3], [0., 0., 10.], [2., 5.]);
        let axis = line([0., 0., 1.], [0., 0., 1.], [7., 9.]);
        let normal = line([1., 0., 0.], [1., 0., 0.], [31., 41.]);
        let twist = line([0.; 3], [0.; 3], [17., 19.]);
        let scale = line([1., 0., 0.], [2., 0., 0.], [23., 29.]);
        let axes = line([1.; 3], [2., 1., 1.], [43., 47.]);
        let center = line([0.; 3], [0.5, 0., 1.], [59., 61.]);
        let q = [[2., 2.], [3., 3.], [4., 4.]];
        let report = certify_control_trajectory(
            &path,
            &scale,
            &twist,
            &axis,
            &normal,
            Some((&axes, &center)),
            q,
            [0., 1.],
            100,
        )
        .unwrap();
        assert_eq!(report.status, Status::Certified);
        assert_eq!(report.cells, 17);
        assert_eq!(report.traversal, [0., 1.]);
        let remainder = report.linear_remainder_upper().unwrap().unwrap();
        assert!(remainder >= 0.5 && remainder < 0.5 + 1e-10);
        let combined = report.linear_error_upper([0., 0.125]).unwrap().unwrap();
        assert!(combined >= 0.625 && combined < 0.625 + 1e-10);
        assert!(report.linear_error_upper([-1., 0.]).is_err());
        let mut unproved_knots = report.clone();
        unproved_knots.single_span = false;
        assert!(unproved_knots.linear_remainder_upper().unwrap().is_none());
        // Here the independently specified original polynomial is C2 on
        // the open interval even if another source span touches an endpoint.
        let conditional=unproved_knots.linear_second_error_upper_on_open_span(
            [0.,0.125]).unwrap().unwrap();
        assert!(conditional>=0.625 && conditional<0.625+1e-10);
        assert!(unproved_knots.linear_remainder_upper().unwrap().is_none());
        // Piecewise-linear path has zero second derivative on each piece,
        // but its derivative jump gives a nonzero interpolation error. The
        // span premise prevents treating those separate jets as global C2.
        let mut kinked_path = path.clone();
        kinked_path.knots = vec![2., 2., 3.5, 5., 5.];
        kinked_path.control_points = vec![vec![0., 0., 0.], vec![0., 0., 4.], vec![0., 0., 10.]];
        kinked_path.weights = vec![1.; 3];
        let kinked = certify_control_trajectory(
            &kinked_path,
            &scale,
            &twist,
            &axis,
            &normal,
            Some((&axes, &center)),
            q,
            [0., 1.],
            100,
        )
        .unwrap();
        assert_eq!(kinked.status, Status::Certified);
        assert!(!kinked.single_span);
        assert!(kinked.linear_remainder_upper().unwrap().is_none());
        // Independent polynomial trajectory: Qx=2+4.5t+2t².
        // Retained endpoints are displaced in opposite directions; verify
        // the first-derivative bound against the actual chord error.
        let endpoints=[[2.125,3.,4.],[8.25,6.,19.]];
        let first=report.linear_first_error_upper_on_continuous_span(
            endpoints,[0.125,0.25]).unwrap().unwrap();
        for i in 0..=128 {
            let t=i as f64/128.;
            let original=[2.+4.5*t+2.*t*t,3.+3.*t,4.+15.*t];
            let error:[f64;3]=std::array::from_fn(|k|
                original[k]-((1.-t)*endpoints[0][k]+t*endpoints[1][k]));
            assert!(error.iter().map(|v|v*v).sum::<f64>().sqrt()<=first);
        }
        let mut refused=report.clone();
        refused.status=Status::Unresolved;
        assert!(refused.linear_first_error_upper_on_continuous_span(
            endpoints,[0.125,0.25]).unwrap().is_none());
        assert!(report.linear_first_error_upper_on_continuous_span(
            endpoints,[-1.,0.]).is_err());
        let jet = report.jet.unwrap();
        for t in [0., 0.375, 1.] {
            let endpoint = certify_control_value(
                &path,
                &scale,
                &twist,
                &axis,
                &normal,
                Some((&axes, &center)),
                q,
                [t, t],
                100,
            )
            .unwrap();
            assert_eq!(endpoint.status, Status::Certified);
            assert_eq!(endpoint.cells, 17);
            let value = endpoint.value.unwrap();
            let expected = [2. + 4.5 * t + 2. * t * t, 3. + 3. * t, 4. + 15. * t];
            for k in 0..3 {
                assert!(value[k][0] <= expected[k] && expected[k] <= value[k][1]);
                assert!(value[k][1] - value[k][0] < 1e-10);
            }
            assert!(
                endpoint
                    .retained_displacement_upper(expected)
                    .unwrap()
                    .unwrap()
                    < 1e-10
            );
            let mut damaged = expected;
            damaged[0] += 0.125;
            let upper = endpoint
                .retained_displacement_upper(damaged)
                .unwrap()
                .unwrap();
            assert!(upper >= 0.125 && upper < 0.125 + 1e-10);
        }
        for i in 0..=16 {
            let t = i as f64 / 16.;
            for (bounds, point) in [
                (
                    jet.value,
                    [2. + 4.5 * t + 2. * t * t, 3. + 3. * t, 4. + 15. * t],
                ),
                (jet.first, [4.5 + 4. * t, 3., 15.]),
                (jet.second, [4., 0., 0.]),
            ] {
                for k in 0..3 {
                    assert!(bounds[k][0] <= point[k] && point[k] <= bounds[k][1]);
                }
            }
        }
        for budget in [0, 16] {
            let report = certify_control_trajectory(
                &path,
                &scale,
                &twist,
                &axis,
                &normal,
                Some((&axes, &center)),
                q,
                [0., 1.],
                budget,
            )
            .unwrap();
            assert_eq!(report.status, Status::Unresolved);
            assert!(report.jet.is_none() && report.cells <= budget);
            assert!(report.linear_remainder_upper().unwrap().is_none());
            let endpoint = certify_control_value(
                &path,
                &scale,
                &twist,
                &axis,
                &normal,
                Some((&axes, &center)),
                q,
                [1., 1.],
                budget,
            )
            .unwrap();
            assert_eq!(endpoint.status, Status::Unresolved);
            assert!(endpoint.value.is_none() && endpoint.cells <= budget);
            assert!(
                endpoint
                    .retained_displacement_upper([0.; 3])
                    .unwrap()
                    .is_none()
            );
        }
    }
}

#[test]
fn value_enclosure_segment_bound_covers_knot_error_without_a_smoothness_premise() {
    let report = ControlValueReport {
        status: Status::Certified,
        cells: 0,
        value: Some([[0., 0.], [0., 0.], [3., 5.5]]),
        reason: None,
    };
    let bound = report
        .retained_segment_displacement_upper([[0., 0., 3.75], [0., 0., 6.25]])
        .unwrap()
        .unwrap();
    // A piecewise-linear ideal path at the middle knot is z=4 while
    // retained endpoint interpolation is z=5. This discrepancy is enclosed.
    assert!(bound >= 3.25 && bound < 3.25 + 1e-10);
    assert!(bound >= 1.);
    let unresolved = ControlValueReport {
        status: Status::Unresolved,
        ..report.clone()
    };
    assert!(
        unresolved
            .retained_segment_displacement_upper([[0.; 3]; 2])
            .unwrap()
            .is_none()
    );
    assert!(
        report
            .retained_segment_displacement_upper([[f64::NAN; 3]; 2])
            .is_err()
    );
}

#[cfg(test)]
mod authored_batch_equivalence_tests {
    use super::*;
    #[test]
    fn authored_batches_preserve_every_single_control_value_and_jet() {
        let path=crate::primitives::circle([0.;3],[0.,0.,1.],4.).unwrap();
        let mut axis=path.clone();
        for p in &mut axis.control_points {let x=p[0];p[0]=-p[1]/4.;p[1]=x/4.;}
        let constant=|p:[f64;3]|Curve {degree:1,knots:vec![0.,0.,1.,1.],
            control_points:vec![p.to_vec();2],weights:vec![1.;2],periodic:false};
        let normal=constant([0.,0.,1.]);
        let scale=crate::primitives::line([1.,0.,0.],[1.01,0.,0.]).unwrap();
        let twist=crate::primitives::line([0.;3],[0.02,0.,0.]).unwrap();
        let axes=constant([1.1,1.05,1.]);
        let center=constant([0.01,0.02,0.03]);
        let qs=[[[0.1,0.1],[0.2,0.2],[0.,0.]], [[-0.1,-0.09],[0.,0.01],[0.02,0.03]]];
        let interval=[0.01,0.02];let affine=Some((&axes,&center));
        let values=certify_control_values(&path,&scale,&twist,&axis,&normal,affine,&qs,interval,100000).unwrap();
        let jets=certify_control_trajectories(&path,&scale,&twist,&axis,&normal,affine,&qs,interval,100000).unwrap();
        assert_eq!(values.status,Status::Certified);assert_eq!(jets.status,Status::Certified);
        let mut single_value_cells=0;let mut single_jet_cells=0;
        for (i,&q) in qs.iter().enumerate() {
            let value=certify_control_value(&path,&scale,&twist,&axis,&normal,affine,q,interval,100000).unwrap();
            let jet=certify_control_trajectory(&path,&scale,&twist,&axis,&normal,affine,q,interval,100000).unwrap();
            assert_eq!(value.status,Status::Certified);assert_eq!(jet.status,Status::Certified);
            assert_eq!(value.value.unwrap(),values.values.as_ref().unwrap()[i]);
            let single=jet.jet.unwrap();let batch=&jets.jets.as_ref().unwrap()[i];
            assert_eq!(single.value,batch.value);assert_eq!(single.first,batch.first);assert_eq!(single.second,batch.second);
            assert_eq!(jet.single_span,jets.single_span);
            single_value_cells+=value.cells;single_jet_cells+=jet.cells;
        }
        assert!(values.cells<single_value_cells);assert!(jets.cells<single_jet_cells);
        assert_eq!(certify_control_values(&path,&scale,&twist,&axis,&normal,affine,&qs,interval,values.cells-1).unwrap().status,Status::Unresolved);
        assert_eq!(certify_control_trajectories(&path,&scale,&twist,&axis,&normal,affine,&qs,interval,jets.cells-1).unwrap().status,Status::Unresolved);
    }
}
