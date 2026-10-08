//! Same-invocation original pose cover. Only certified entries are retained;
//! owner validates identical source/laws before sharing between profiles.
use super::*;
use std::collections::HashMap;
#[derive(Default)]
pub(crate) struct SharedArcPose {
    entries: HashMap<[u64; 4], Vec<[[f64; 2]; 3]>>,
    pub(super) regular_certified: bool,
    speed_lower: Option<f64>,
    jet_entries: HashMap<[u64;7],Vec<authored_frame_certificate::FrameJet>>,
    curvatures: HashMap<[u64;3],f64>,
}
impl SharedArcPose {
    pub(super) fn regular(
        &mut self,
        path: &Curve,
        budget: usize,
    ) -> Result<super::corrected_regular::Report> {
        if self.regular_certified {
            return Ok(super::corrected_regular::Report {
                certified: true,
                cells: 0,
                reason: None,
            });
        }
        let report = super::corrected_regular::certify(path, budget)?;
        self.regular_certified = report.certified;
        Ok(report)
    }

    pub(super) fn speed(&mut self,path:&Curve,budget:usize)->Result<(usize,Option<f64>)>{
        if let Some(speed)=self.speed_lower{return Ok((0,Some(speed)));}
        let (cover,speed)=crate::curve_regularity::inspect_speed(path,budget)?;
        self.speed_lower=speed;Ok((cover.cells,speed))
    }
    pub(super) fn curvature(&mut self,path:&Curve,source:[f64;2],speed:f64,budget:usize)->Result<(usize,Option<f64>)>{
        let key=[source[0].to_bits(),source[1].to_bits(),speed.to_bits()];
        if let Some(value)=self.curvatures.get(&key){return Ok((0,Some(*value)));}
        let report=vector_certificate::certify_traversal(path,source,budget,false)?;
        if report.status!=Status::Certified{return Ok((report.cells,None));}
        let a=report.second.unwrap().map(|v|I{lo:v[0],hi:v[1]});
        let upper=crate::numerics::interval_vec3::norm(a)?.div(I::point(speed).mul(I::point(speed))?)?.hi;
        self.curvatures.insert(key,upper);Ok((report.cells,Some(upper)))
    }
    pub(super) fn arc_jets(&mut self,sweep:&Sweep<'_>,qs:&[[[f64;2];3]],source:[f64;2],law:[f64;2],length:[f64;2],speed:f64,budget:usize)->Result<authored_frame_certificate::TrajectoriesReport>{
        let key=[source[0].to_bits(),source[1].to_bits(),law[0].to_bits(),law[1].to_bits(),length[0].to_bits(),length[1].to_bits(),speed.to_bits()];
        let mut cells=0;
        if !self.jet_entries.contains_key(&key){
            let mut basis=vec![[[0.;2];3];4];for j in 0..3{basis[j+1][j]=[1.;2];}
            let report=authored_frame_certificate::certify_c4_frenet_arc_relative_trajectories(sweep.path,sweep.scale,sweep.twist,sweep.affine_laws,&basis,source,law,length,speed,budget)?;
            cells=report.cells;
            if report.status!=Status::Certified{return Ok(report);}
            if !report.single_span{return Ok(authored_frame_certificate::TrajectoriesReport{traversal:law,status:Status::Unresolved,cells,jets:None,single_span:false,reason:Some("arc-station-law-second-remainder-unproved")});}
            self.jet_entries.insert(key,report.jets.unwrap());
        }
        let pose=&self.jet_entries[&key];let mut jets=Vec::with_capacity(qs.len());
        for q in qs {
            let mut fields=[[[0.;2];3];3];
            for (field,result) in fields.iter_mut().enumerate(){
                let read=|j:usize|match field{0=>pose[j].value,1=>pose[j].first,_=>pose[j].second};
                for k in 0..3 {
                    let offset=I::new(read(0)[k][0],read(0)[k][1])?;let mut value=offset;
                    for j in 0..3{let direction=I::new(read(j+1)[k][0],read(j+1)[k][1])?.sub(offset)?;value=value.add(direction.mul(I::new(q[j][0],q[j][1])?)?)?;}
                    result[k]=[value.lo,value.hi];
                }
            }
            jets.push(authored_frame_certificate::FrameJet{value:fields[0],first:fields[1],second:fields[2]});
        }
        Ok(authored_frame_certificate::TrajectoriesReport{traversal:law,status:Status::Certified,cells,jets:Some(jets),single_span:true,reason:None})
    }

    pub(super) fn values(
        &mut self,
        sweep: &Sweep<'_>,
        qs: &[[[f64; 2]; 3]],
        path: [f64; 2],
        law: [f64; 2],
        budget: usize,
    ) -> Result<authored_frame_certificate::ControlValuesReport> {
        let key = [
            path[0].to_bits(),
            path[1].to_bits(),
            law[0].to_bits(),
            law[1].to_bits(),
        ];
        let mut cells = 0;
        if !self.entries.contains_key(&key) {
            let mut basis = vec![[[0.; 2]; 3]; 4];
            for j in 0..3 {
                basis[j + 1][j] = [1.; 2];
            }
            let report = authored_frame_certificate::certify_frenet_relative_values(
                sweep.path,
                sweep.scale,
                sweep.twist,
                sweep.affine_laws,
                &basis,
                path,
                law,
                budget,
            )?;
            cells = report.cells;
            if report.status != Status::Certified {
                return Ok(report);
            }
            self.entries.insert(key, report.values.unwrap());
        }
        let pose = &self.entries[&key];
        let mut values = Vec::with_capacity(qs.len());
        for q in qs {
            let mut value = [I::point(0.); 3];
            for k in 0..3 {
                let offset = I::new(pose[0][k][0], pose[0][k][1])?;
                value[k] = offset;
                for j in 0..3 {
                    let direction = I::new(pose[j + 1][k][0], pose[j + 1][k][1])?.sub(offset)?;
                    value[k] = value[k].add(direction.mul(I::new(q[j][0], q[j][1])?)?)?;
                }
            }
            values.push(value.map(|x| [x.lo, x.hi]));
        }
        Ok(authored_frame_certificate::ControlValuesReport {
            status: Status::Certified,
            cells,
            values: Some(values),
            reason: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_pose_encloses_independent_signed_affine_controls_and_charges_first_query() {
        let path = crate::paths::bezier(
            vec![
                vec![0., 0., 0.],
                vec![1., 0., 0.],
                vec![2., 1., 0.],
                vec![3., 1., 1.],
            ],
            None,
        )
        .unwrap();
        let profile = crate::primitives::line([0., 0.1, 0.], [0., 0.2, 0.]).unwrap();
        let scale = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        let twist = crate::primitives::line([0.; 3], [0.25, 0., 0.]).unwrap();
        let axes = crate::primitives::line([1.; 3], [2., 1., 1.]).unwrap();
        let center = crate::primitives::line([0.; 3], [0.125, 0., 0.]).unwrap();
        let sweep = Sweep::new(
            &profile,
            &path,
            &scale,
            &twist,
            Options {
                normal: [0., 0., 1.],
                orientation: Orientation::CorrectedFrenet,
                spacing: Spacing::ArcLength {
                    tolerance: 1e-6,
                    max_cells: 100000,
                },
                initial_sections: 3,
                max_sections: 129,
                max_deviation: 0.05,
            },
        )
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap();
        let qs = [[-0.2, 0.3, -0.1], [0.1, -0.2, 0.3]].map(|q| q.map(|x| [x, x]));
        let mut eligibility = SharedArcPose::default();
        assert!(!eligibility.regular(&path, 0).unwrap().certified);
        assert!(!eligibility.regular_certified);
        let first = eligibility.regular(&path, 10000).unwrap();
        assert!(first.certified && first.cells > 0);
        let reused = eligibility.regular(&path, 0).unwrap();
        assert!(reused.certified);
        assert_eq!(reused.cells, 0);
        let mut cache = SharedArcPose::default();
        assert_eq!(
            cache
                .values(&sweep, &qs, [0.2, 0.3], [0.4, 0.5], 0)
                .unwrap()
                .status,
            Status::Unresolved
        );
        assert!(cache.entries.is_empty());
        let fresh = cache
            .values(&sweep, &qs, [0.2, 0.3], [0.4, 0.5], 1000)
            .unwrap();
        assert_eq!(fresh.status, Status::Certified);
        assert!(fresh.cells > 0);
        let reused = cache
            .values(&sweep, &qs, [0.2, 0.3], [0.4, 0.5], 0)
            .unwrap();
        assert_eq!(reused.cells, 0);
        assert_eq!(fresh.values, reused.values);
        for u in [0.2, 0.25, 0.3] {
            for v in [0.4_f64, 0.45, 0.5] {
                let t = unit([3., 6. * u - 6. * u * u, 3. * u * u]).unwrap();
                let a = [0., 6. - 12. * u, 6. * u];
                let b = unit(cross(t, a)).unwrap();
                let n = cross(b, t);
                let angle = 0.25 * v;
                let normal = std::array::from_fn(|k| n[k] * angle.cos() + b[k] * angle.sin());
                let binormal = cross(t, normal);
                for (j, q) in qs.iter().enumerate() {
                    let c = [
                        (1. + v).powi(2) * q[0][0] + 0.125 * v,
                        (1. + v) * q[1][0],
                        (1. + v) * q[2][0],
                    ];
                    for k in 0..3 {
                        let x = c[0] * normal[k] + c[1] * binormal[k] + c[2] * t[k];
                        let bound = fresh.values.as_ref().unwrap()[j][k];
                        assert!(bound[0] <= x && x <= bound[1]);
                    }
                }
            }
        }
    }
}


#[cfg(test)]
mod arc_jet_cache_tests {
 use super::*;
 #[test]
 fn certified_arc_basis_jets_share_work_but_failed_or_changed_keys_do_not(){
  let path=crate::paths::bezier(vec![vec![0.;3],vec![0.5,0.,0.],vec![1.,1.,0.]],None).unwrap();
  let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
  let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
  let sweep=Sweep::new(&profile,&path,&scale,&twist,Options{orientation:Orientation::CorrectedFrenet,normal:[0.,1.,0.],spacing:Spacing::ArcLength{tolerance:1e-6,max_cells:10000},initial_sections:3,max_sections:17,max_deviation:0.2}).unwrap();
  let length=5f64.sqrt()/2.+2f64.asinh()/4.;let bounds=[length.next_down(),length.next_up()];let qs=[[[1.;2],[0.;2],[0.;2]]];
  let mut cache=SharedArcPose::default();assert!(cache.regular(&path,10000).unwrap().certified);
  let refused=cache.arc_jets(&sweep,&qs,[0.2,0.3],[0.1,0.4],bounds,1.,0).unwrap();assert_eq!(refused.status,Status::Unresolved);assert!(cache.jet_entries.is_empty());
  let fresh=cache.arc_jets(&sweep,&qs,[0.2,0.3],[0.1,0.4],bounds,1.,10000).unwrap();assert_eq!(fresh.status,Status::Certified);assert!(fresh.cells>0);
  let warm=cache.arc_jets(&sweep,&qs,[0.2,0.3],[0.1,0.4],bounds,1.,0).unwrap();assert_eq!(warm.status,Status::Certified);assert_eq!(warm.cells,0);
  let jet=&warm.jets.as_ref().unwrap()[0];
  for u in [0.2_f64,0.25,0.3]{let g=(1.+4.*u*u).sqrt();let expected=[32.*u*length*length/g.powi(7),(-4.+48.*u*u)*length*length/g.powi(7),0.];for k in 0..3{assert!(jet.second[k][0]<=expected[k]&&expected[k]<=jet.second[k][1]);}}
  let changed=cache.arc_jets(&sweep,&qs,[0.2,0.3],[0.1,0.4],[bounds[0],bounds[1].next_up()],1.,0).unwrap();assert_eq!(changed.status,Status::Unresolved);assert_eq!(cache.jet_entries.len(),1);
  let changed=cache.arc_jets(&sweep,&qs,[0.2,0.3],[0.1,0.4],bounds,1f64.next_down(),0).unwrap();assert_eq!(changed.status,Status::Unresolved);assert_eq!(cache.jet_entries.len(),1);
 }
}
