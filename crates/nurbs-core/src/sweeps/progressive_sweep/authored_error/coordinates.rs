//! Original profile coordinates and authored control trajectories.
use super::*;

#[derive(Clone, Debug)]
pub struct InitialCoordinatesReport {
    pub status: Status,
    pub cells: usize,
    pub coordinates: Option<Vec<[[f64; 2]; 3]>>,
    pub reason: Option<&'static str>,
}
pub(in crate::sweeps::progressive_sweep) fn initial_coordinates(
    sweep: &Sweep<'_>,
    max_cells: usize,
) -> Result<InitialCoordinatesReport> {
    check(
        max_cells <= 100000,
        "Initial coordinate work exceeds100000 cells",
    )?;
    let mut out = InitialCoordinatesReport {
        status: Status::Unresolved,
        cells: 0,
        coordinates: None,
        reason: Some("mode-not-authored"),
    };
    let Some((axis, normal)) = sweep.frame_laws else {
        return Ok(out);
    };
    out.reason = Some("path-start-enclosure-unresolved");
    let start =
        vector_certificate::certify_values_traversal(sweep.path, [0., 0.], max_cells, false)?;
    out.cells += start.cells;
    let Some(start) = start.value else {
        return Ok(out);
    };
    // The initial local basis precedes twist and affine transformation.
    let zero_twist = constant_vector_law([0.; 3])?;
    out.reason = Some("initial-frame-enclosure-unresolved");
    let frame = authored_frame_certificate::certify_twisted_values(
        axis,
        normal,
        &zero_twist,
        [0., 0.],
        max_cells - out.cells,
    )?;
    out.cells += frame.cells;
    if frame.status != Status::Certified {
        return Ok(out);
    }
    out.status = Status::Certified;
    out.reason = None;
    out.coordinates = Some(coordinates_in_basis(sweep.profile, start, frame)?);
    Ok(out)
}
pub(in crate::sweeps::progressive_sweep) fn fixed_initial_coordinates(sweep:&Sweep<'_>,max_cells:usize)->Result<InitialCoordinatesReport>{
    seed_initial_coordinates(sweep,max_cells,Orientation::Fixed)
}
pub(in crate::sweeps::progressive_sweep) fn fixed_normal_initial_coordinates(sweep:&Sweep<'_>,max_cells:usize)->Result<InitialCoordinatesReport>{
    seed_initial_coordinates(sweep,max_cells,Orientation::FixedNormal)
}
pub(super) fn seed_initial_coordinates(sweep:&Sweep<'_>,max_cells:usize,orientation:Orientation)->Result<InitialCoordinatesReport>{
    seed_initial_coordinates_as(sweep,max_cells,orientation,orientation)
}
pub(super) fn seed_initial_coordinates_as(sweep:&Sweep<'_>,max_cells:usize,orientation:Orientation,actual:Orientation)->Result<InitialCoordinatesReport>{
    check(max_cells<=100000,"Fixed initial coordinate work exceeds100000 cells")?;
    let mut out=InitialCoordinatesReport {status:Status::Unresolved,cells:0,coordinates:None,reason:Some("mode-not-fixed")};
    if sweep.options.orientation!=actual || sweep.frame_laws.is_some() || sweep.orientation_guide.is_some(){return Ok(out);}
    let start=vector_certificate::certify_values_traversal(sweep.path,[0.,0.],max_cells,false)?;
    out.cells+=start.cells;out.reason=Some("path-start-enclosure-unresolved");
    let Some(start)=start.value else {return Ok(out);};
    let zero=constant_vector_law([0.;3])?;
    let frame=if orientation==Orientation::Frenet {
        authored_frame_certificate::certify_frenet_path_values(sweep.path,&zero,[0.,0.],max_cells-out.cells)?
    } else {authored_frame_certificate::certify_fixed_path_values(sweep.path,sweep.options.normal,&zero,[0.,0.],max_cells-out.cells)?};
    out.cells+=frame.cells;out.reason=Some("fixed-initial-frame-unresolved");
    if frame.status!=Status::Certified {return Ok(out);}
    out.coordinates=Some(coordinates_in_basis(sweep.profile,start,frame)?);
    out.status=Status::Certified;out.reason=None;Ok(out)
}

pub(super) fn principal_initial_coordinates(sweep:&Sweep<'_>,phase:f64,max_cells:usize)->Result<InitialCoordinatesReport>{
    let start=vector_certificate::certify_values_traversal(sweep.path,[0.,0.],max_cells,false)?;
    let mut out=InitialCoordinatesReport {status:Status::Unresolved,cells:start.cells,coordinates:None,reason:Some("corrected-initial-phase-coordinates-unproved")};
    let Some(start)=start.value else{return Ok(out);};
    let zero=constant_vector_law([0.;3])?;
    let frame=authored_frame_certificate::certify_planar_principal_values(sweep.path,sweep.options.normal,phase,&zero,[0.,0.],[0.,0.],max_cells-out.cells)?;
    out.cells+=frame.cells;
    if frame.status==Status::Certified{out.coordinates=Some(coordinates_in_basis(sweep.profile,start,frame)?);out.status=Status::Certified;out.reason=None;}
    Ok(out)
}

pub(super) fn coordinates_in_basis(
    profile: &Curve,
    start: [[f64; 2]; 3],
    frame: authored_frame_certificate::ValuesReport,
) -> Result<Vec<[[f64; 2]; 3]>> {
    let decode = |v: [[f64; 2]; 3]| -> Result<[I; 3]> {
        Ok([
            I::new(v[0][0], v[0][1])?,
            I::new(v[1][0], v[1][1])?,
            I::new(v[2][0], v[2][1])?,
        ])
    };
    let start = decode(start)?;
    let basis = [
        decode(frame.transverse.unwrap())?,
        decode(frame.binormal.unwrap())?,
        decode(frame.longitudinal.unwrap())?,
    ];
    let mut coordinates = Vec::with_capacity(profile.control_points.len());
    for pole in &profile.control_points {
        let offset = interval_sub(
            [I::point(pole[0]), I::point(pole[1]), I::point(pole[2])],
            start,
        )?;
        let q = [
            dot_tight(offset, basis[0])?,
            dot_tight(offset, basis[1])?,
            dot_tight(offset, basis[2])?,
        ];
        coordinates.push(q.map(|x| [x.lo, x.hi]));
    }
    Ok(coordinates)
}

pub(in crate::sweeps::progressive_sweep) fn guided_initial_coordinates(
    sweep: &Sweep<'_>,
    max_cells: usize,
) -> Result<InitialCoordinatesReport> {
    check(
        max_cells <= 100000,
        "Initial coordinate work exceeds100000 cells",
    )?;
    let mut out = InitialCoordinatesReport {
        status: Status::Unresolved,
        cells: 0,
        coordinates: None,
        reason: Some("mode-not-guided"),
    };
    let Some(guide) = sweep.orientation_guide else {
        return Ok(out);
    };
    out.reason = Some("guided-path-start-enclosure-unresolved");
    let start =
        vector_certificate::certify_values_traversal(sweep.path, [0., 0.], max_cells, false)?;
    out.cells += start.cells;
    let Some(start) = start.value else {
        return Ok(out);
    };
    let zero_twist = constant_vector_law([0.; 3])?;
    out.reason = Some("guided-initial-frame-enclosure-unresolved");
    let frame = authored_frame_certificate::certify_path_guide_values(
        sweep.path,
        guide,
        &zero_twist,
        [0., 0.],
        max_cells - out.cells,
    )?;
    out.cells += frame.cells;
    if frame.status != Status::Certified {
        return Ok(out);
    }
    out.coordinates = Some(coordinates_in_basis(sweep.profile, start, frame)?);
    out.status = Status::Certified;
    out.reason = None;
    Ok(out)
}

pub(in crate::sweeps::progressive_sweep) fn control_trajectory(
    sweep: &Sweep<'_>,
    profile_control: usize,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<authored_frame_certificate::TrajectoryReport> {
    check(
        profile_control < sweep.profile.control_points.len(),
        "Profile control index outside sweep",
    )?;
    check(
        max_cells <= 100000,
        "Control trajectory work exceeds100000 cells",
    )?;
    let unresolved = |cells, reason| authored_frame_certificate::TrajectoryReport {
        traversal,
        status: Status::Unresolved,
        cells,
        jet: None,
        single_span: false,
        reason: Some(reason),
    };
    if sweep.options.spacing != Spacing::Parameter && !original_line(sweep.path) {
        return Ok(unresolved(0, "arc-length-correspondence-unproved"));
    }
    let length_reference=(sweep.options.spacing != Spacing::Parameter).then(|| Curve {
        degree:1,knots:vec![0.,0.,1.,1.],control_points:sweep.path.control_points.clone(),
        weights:vec![1.,1.],periodic:false,
    });
    let reference_path=length_reference.as_ref().unwrap_or(sweep.path);
    let [a, b] = sweep.path.domain();
    let p = sweep.path.evaluate(a)?.point;
    let q = sweep.path.evaluate(b)?.point;
    // Fixed authored transport has no closed holonomy correction. This report
    // encloses the original trajectory; retained closing displacement is a
    // separate obligation handled by the section interpolation owner.
    if closed_extent(sweep.path, [p[0], p[1], p[2]], [q[0], q[1], q[2]])?
        && sweep.options.orientation!=Orientation::Fixed {
        return Ok(unresolved(0, "closed-frame-correction-unproved"));
    }
    let initial = initial_coordinates(sweep, max_cells)?;
    if initial.status != Status::Certified {
        return Ok(unresolved(initial.cells, initial.reason.unwrap()));
    }
    let (axis, normal) = sweep.frame_laws.unwrap();
    let mut report = authored_frame_certificate::certify_control_trajectory(
        reference_path,
        sweep.scale,
        sweep.twist,
        axis,
        normal,
        sweep.affine_laws,
        initial.coordinates.unwrap()[profile_control],
        traversal,
        max_cells - initial.cells,
    )?;
    report.cells += initial.cells;
    Ok(report)
}

