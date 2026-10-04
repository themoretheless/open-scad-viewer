//! Enclosures of the ideal polyline minimum-rotation transport construction.
//! Includes normalization, corner bisectors and the closing transport. The
//! distributed correction and rotated stations are separate
//! obligations; this report is not a whole-sweep certificate.
use super::scalar_certificate::Status;
use crate::sweep_support::interval_vec3::{cross, dot_tight as dot, scale as mul, sub};
use crate::{Result, check, distance_bounds::Interval as I};
type V = [I; 3];
pub type VectorEnclosure = [[f64; 2]; 3];
#[derive(Clone,Copy)]
pub(super) enum FrameSource<'a> {
    Authored(&'a crate::curve::Curve,&'a crate::curve::Curve),
    AuthoredGuide(&'a crate::curve::Curve,&'a crate::curve::Curve),
    Guide(&'a crate::curve::Curve),
}
#[derive(Clone, Debug)]
pub struct Report {
    pub status: Status,
    pub cells: usize,
    pub tangents: Option<Vec<VectorEnclosure>>,
    pub normals: Option<Vec<VectorEnclosure>>,
    pub planes: Option<Vec<Option<VectorEnclosure>>>,
    pub closing_normal: Option<VectorEnclosure>,
    /// Principal signed closing-to-initial rotation about the first tangent.
    pub holonomy_angle: Option<[f64; 2]>,
    pub cumulative_lengths: Option<Vec<[f64; 2]>>,
    pub reason: Option<&'static str>,
    sites: Option<Vec<[f64; 3]>>,
}
impl Report {
    /// Ideal polyline chord-length traversal, including outward interpolation.
    /// Returns no value when the construction was unresolved.
    pub fn traversal(&self, edge: usize, fraction: [f64; 2]) -> Result<Option<[f64; 2]>> {
        check(
            fraction[0].is_finite()
                && fraction[1].is_finite()
                && fraction[0] >= 0.
                && fraction[1] <= 1.
                && fraction[0] <= fraction[1],
            "Traversal fraction must be an ordered interval in [0,1]",
        )?;
        let Some(lengths) = &self.cumulative_lengths else {
            return Ok(None);
        };
        check(
            edge < lengths.len().saturating_sub(1),
            "Traversal edge outside certificate",
        )?;
        let a = I::new(lengths[edge][0], lengths[edge][1])?;
        let b = I::new(lengths[edge + 1][0], lengths[edge + 1][1])?;
        let total = lengths.last().unwrap();
        let value = a
            .add(I::new(fraction[0], fraction[1])?.mul(b.sub(a)?)?)?
            .div(I::new(total[0], total[1])?)?
            .intersect(0., 1.)?;
        Ok(Some([value.lo, value.hi]))
    }
    /// Ideal rotated normal and binormal for a certified scalar twist range.
    /// The caller must enclose the twist law over this traversal interval.
    /// A wide trigonometric range remains valid but may not prove a tolerance.
    pub fn rotated_frame(
        &self,
        edge: usize,
        fraction: [f64; 2],
        twist: [f64; 2],
    ) -> Result<Option<(VectorEnclosure, VectorEnclosure)>> {
        let twist = I::new(twist[0], twist[1])?;
        let Some(correction) = self.correction_at(edge, fraction)? else {
            return Ok(None);
        };
        let angle = twist.add(I::new(correction[0], correction[1])?)?;
        let tr = super::trigonometric_certificate::certify([angle.lo, angle.hi])?;
        let Some(tangents) = &self.tangents else {
            return Ok(None);
        };
        let Some(normals) = &self.normals else {
            return Ok(None);
        };
        let t = decode(tangents[edge])?;
        let n = decode(normals[edge])?;
        let b = cross(t, n)?;
        let cosine = mul(n, I::new(tr.cos[0], tr.cos[1])?)?;
        let sine = mul(b, I::new(tr.sin[0], tr.sin[1])?)?;
        let rotated = [
            cosine[0].add(sine[0])?,
            cosine[1].add(sine[1])?,
            cosine[2].add(sine[2])?,
        ];
        Ok(Some((encode(rotated), encode(cross(t, rotated)?))))
    }
    /// Encloses the miter shear of a certified world-space transverse offset.
    /// Translation by the authored path is separate. No sampled plane or
    /// rounded denominator is substituted for the interval construction.
    pub fn miter_offset(
        &self,
        edge: usize,
        fraction: [f64; 2],
        offset: VectorEnclosure,
    ) -> Result<Option<VectorEnclosure>> {
        let q = decode(offset)?;
        if self.traversal(edge, fraction)?.is_none() {
            return Ok(None);
        }
        let Some(tangents) = &self.tangents else {
            return Ok(None);
        };
        let Some(planes) = &self.planes else {
            return Ok(None);
        };
        let t = decode(tangents[edge])?;
        let shift = |plane: Option<VectorEnclosure>| -> Result<I> {
            match plane {
                None => Ok(I::point(0.)),
                Some(p) => {
                    let h = decode(p)?;
                    dot(q, h)?.div(dot(t, h)?)
                }
            }
        };
        let f = I::new(fraction[0], fraction[1])?;
        let axial = I::point(1.)
            .sub(f)?
            .mul(shift(planes[edge])?)?
            .add(f.mul(shift(planes[(edge + 1) % planes.len()])?)?)?;
        Ok(Some(encode(sub(q, mul(t, axial)?)?)))
    }
    /// Encloses one ideal control point of the station construction. Scale
    /// and twist ranges must be certified over this same traversal interval.
    /// This is a point/range enclosure, not the interpolation error certificate.
    pub fn station_point(
        &self,
        edge: usize,
        fraction: [f64; 2],
        authored: [f64; 3],
        scale: [f64; 2],
        twist: [f64; 2],
    ) -> Result<Option<VectorEnclosure>> {
        self.station_point_affine(edge,fraction,authored,scale,twist,[[1.,1.];3],[[0.,0.];3])
    }
    /// Local affine law convention: uniform scale multiplies axis-scaled
    /// profile offsets; the center is added in the rotated local frame before
    /// the same miter projection as the profile. Axial center uses the tangent.
    pub fn station_point_affine(
        &self,edge:usize,fraction:[f64;2],authored:[f64;3],scale:[f64;2],twist:[f64;2],
        axes:[[f64;2];3],center:[[f64;2];3],
    )->Result<Option<VectorEnclosure>> {
        check(
            authored.iter().all(|x| x.is_finite()),
            "Station profile point must be finite",
        )?;
        let scale = I::new(scale[0], scale[1])?;
        check(scale.lo > 0., "Station scale interval must be positive")?;
        let Some((n, b)) = self.rotated_frame(edge, fraction, twist)? else {
            return Ok(None);
        };
        let Some(sites) = &self.sites else {
            return Ok(None);
        };
        let normals = self.normals.as_ref().unwrap();
        let tangents = self.tangents.as_ref().unwrap();
        let initial = decode(normals[0])?;
        let initial_binormal = cross(decode(tangents[0])?, initial)?;
        let profile = sub(point(authored), point(sites[0]))?;
        let x = dot(profile, initial)?;
        let y = dot(profile, initial_binormal)?;
        let q=if axes==[[1.,1.];3] && center==[[0.,0.];3] {
            // Preserve the original scalar-only arithmetic and enclosures.
            let nx=mul(decode(n)?,x)?;let by=mul(decode(b)?,y)?;
            mul([nx[0].add(by[0])?,nx[1].add(by[1])?,nx[2].add(by[2])?],scale)?
        } else {
        let axes=axes.map(|v|I::new(v[0],v[1])).into_iter().collect::<Result<Vec<_>>>()?;
        check(axes.iter().all(|v|v.lo>0.),"Station axis scale intervals must be positive")?;
        let center=center.map(|v|I::new(v[0],v[1])).into_iter().collect::<Result<Vec<_>>>()?;
        let nx = mul(decode(n)?, x.mul(axes[0])?.mul(scale)?.add(center[0])?)?;
        let by = mul(decode(b)?, y.mul(axes[1])?.mul(scale)?.add(center[1])?)?;
        let tz=mul(decode(tangents[edge])?,center[2])?;
        [nx[0].add(by[0])?.add(tz[0])?,nx[1].add(by[1])?.add(tz[1])?,nx[2].add(by[2])?.add(tz[2])?]
        };
        let Some(sheared) = self.miter_offset(edge, fraction, encode(q))? else {
            return Ok(None);
        };
        let sheared = decode(sheared)?;
        let f = I::new(fraction[0], fraction[1])?;
        let a = mul(point(sites[edge]), I::point(1.).sub(f)?)?;
        let b = mul(point(sites[(edge + 1) % sites.len()]), f)?;
        Ok(Some(encode([
            a[0].add(b[0])?.add(sheared[0])?,
            a[1].add(b[1])?.add(sheared[1])?,
            a[2].add(b[2])?.add(sheared[2])?,
        ])))
    }
    /// Integrated law-to-station enclosure. Each law has an independent
    /// max_cells bound; unresolved scalar or frame work yields no point.
    pub fn station_with_laws(
        &self,
        edge: usize,
        fraction: [f64; 2],
        authored: [f64; 3],
        scale: &crate::curve::Curve,
        twist: &crate::curve::Curve,
        max_cells: usize,
    ) -> Result<Option<VectorEnclosure>> {
        self.station_with_affine_laws(edge,fraction,authored,scale,twist,None,max_cells)
    }
    /// Optional XYZ axis/center laws use their own authored domains and share
    /// one component-cell budget. Scalar scale/twist budgets remain separate.
    pub fn station_with_affine_laws(
        &self,edge:usize,fraction:[f64;2],authored:[f64;3],scale:&crate::curve::Curve,
        twist:&crate::curve::Curve,affine:Option<(&crate::curve::Curve,&crate::curve::Curve)>,max_cells:usize,
    )->Result<Option<VectorEnclosure>> {
        let Some(traversal) = self.traversal(edge, fraction)? else {
            return Ok(None);
        };
        let s = super::scalar_certificate::value_traversal(scale, traversal, max_cells)?;
        let t = super::scalar_certificate::value_traversal(twist, traversal, max_cells)?;
        let (Some(s), Some(t)) = (s, t) else {
            return Ok(None);
        };
        if s[0] <= 0. {
            return Ok(None);
        }
        if let Some((axes,center))=affine {
            let axes=super::vector_certificate::certify_values_traversal(axes,traversal,max_cells,true)?;
            let center=super::vector_certificate::certify_values_traversal(center,traversal,max_cells-axes.cells,false)?;
            let (Some(axes),Some(center))=(axes.value,center.value) else {return Ok(None);};
            self.station_point_affine(edge,fraction,authored,s,t,axes,center)
        } else {
            self.station_point(edge, fraction, authored, s, t)
        }
    }
    fn edge_path_enclosures(&self,edge:usize,fraction:[f64;2])->Result<(VectorEnclosure,VectorEnclosure,VectorEnclosure)> {
        let sites=self.sites.as_ref().unwrap();let lengths=self.cumulative_lengths.as_ref().unwrap();
        let f=I::new(fraction[0],fraction[1])?;
        let a=point(sites[edge]);let b=point(sites[(edge+1)%sites.len()]);
        let delta=sub(b,a)?;let part=mul(delta,f)?;
        let path=encode([a[0].add(part[0])?,a[1].add(part[1])?,a[2].add(part[2])?]);
        let interval=|v:[f64;2]|I::new(v[0],v[1]);
        let rate=interval(*lengths.last().unwrap())?.div(interval(lengths[edge+1])?.sub(interval(lengths[edge])?)?)?;
        Ok((self.tangents.as_ref().unwrap()[edge],path,encode(mul(delta,rate)?)))
    }
    /// Original profile coordinates in the certified initial path frame.
    /// Interval projection preserves construction rounding before authored-frame mapping.
    pub fn profile_local_enclosure(&self,authored:[f64;3])->Result<Option<VectorEnclosure>> {
        check(authored.iter().all(|x|x.is_finite()),"Profile point must be finite")?;
        let (Some(normals),Some(tangents),Some(sites))=(&self.normals,&self.tangents,&self.sites) else {return Ok(None);};
        let n=decode(normals[0])?;let b=cross(decode(tangents[0])?,n)?;
        let p=sub(point(authored),point(sites[0]))?;
        let x=dot(p,n)?;let y=dot(p,b)?;
        Ok(Some([[x.lo,x.hi],[y.lo,y.hi],[0.,0.]]))
    }
    /// Station enclosure for an independently authored frame. Local XY is
    /// expressed in the initial profile frame; local Z must be zero. Frame,
    /// twist and scale laws share one cell budget. Miter projection continues
    /// to use the path tangent and its endpoint planes, not the frame axis.
    pub fn station_local_authored_frame(&self,edge:usize,fraction:[f64;2],local:VectorEnclosure,longitudinal:&crate::curve::Curve,transverse:&crate::curve::Curve,scale:&crate::curve::Curve,twist:&crate::curve::Curve,max_cells:usize)->Result<Option<VectorEnclosure>> {
        self.station_local_authored_frame_affine(edge,fraction,local,longitudinal,transverse,scale,twist,None,max_cells)
    }
    pub fn station_local_authored_frame_affine(&self,edge:usize,fraction:[f64;2],local:VectorEnclosure,longitudinal:&crate::curve::Curve,transverse:&crate::curve::Curve,scale:&crate::curve::Curve,twist:&crate::curve::Curve,affine:Option<(&crate::curve::Curve,&crate::curve::Curve)>,max_cells:usize)->Result<Option<VectorEnclosure>> {
        self.station_frame_affine(edge,fraction,local,FrameSource::Authored(longitudinal,transverse),scale,twist,affine,max_cells)
    }
    pub(super) fn station_frame_affine(
        &self, edge:usize, fraction:[f64;2], local:VectorEnclosure,
        source:FrameSource<'_>,
        scale:&crate::curve::Curve, twist:&crate::curve::Curve,affine:Option<(&crate::curve::Curve,&crate::curve::Curve)>,max_cells:usize,
    )->Result<Option<VectorEnclosure>> {
        check(local.iter().flatten().all(|x|x.is_finite()) && local[2]==[0.,0.],"Authored miter local profile must be finite and planar")?;
        let Some(traversal)=self.traversal(edge,fraction)? else {return Ok(None);};
        let frame=match source {
            FrameSource::Authored(longitudinal,transverse)=>super::authored_frame_certificate::certify_twisted_values(longitudinal,transverse,twist,traversal,max_cells)?,
            FrameSource::AuthoredGuide(axis,guide)=>{let (_,p,_)=self.edge_path_enclosures(edge,fraction)?;super::authored_frame_certificate::certify_authored_guide_values(axis,guide,twist,traversal,p,max_cells)?},
            FrameSource::Guide(guide)=>{let (t,p,_)=self.edge_path_enclosures(edge,fraction)?;super::authored_frame_certificate::certify_guide_values(guide,twist,traversal,t,p,max_cells)?},
        };
        if frame.status!=Status::Certified {return Ok(None);}
        let scale_charge=(scale.degree..scale.control_points.len()).filter(|&i|scale.knots[i]<scale.knots[i+1]).count();
        if scale_charge>max_cells-frame.cells {return Ok(None);}
        let scale=super::scalar_certificate::value_traversal(scale,traversal,scale_charge)?;
        let Some(scale)=scale else {return Ok(None);};
        if scale[0]<=0. {return Ok(None);}
        let s=I::new(scale[0],scale[1])?;
        let n=decode(frame.transverse.unwrap())?;
        let b=decode(frame.binormal.unwrap())?;
        let mut u=[I::new(local[0][0],local[0][1])?.mul(s)?,I::new(local[1][0],local[1][1])?.mul(s)?,I::point(0.)];
        if let Some((axes,center))=affine {
            let a=super::vector_certificate::certify_values_traversal(axes,traversal,max_cells-frame.cells-scale_charge,true)?;
            let c=super::vector_certificate::certify_values_traversal(center,traversal,max_cells-frame.cells-scale_charge-a.cells,false)?;
            let (Some(av),Some(cv))=(a.value,c.value) else {return Ok(None);};
            for k in 0..3 {u[k]=u[k].mul(I::new(av[k][0],av[k][1])?)?.add(I::new(cv[k][0],cv[k][1])?)?;}
        }
        let nx=mul(n,u[0])?;let by=mul(b,u[1])?;
        let tz=mul(decode(frame.longitudinal.unwrap())?,u[2])?;
        let q=[nx[0].add(by[0])?.add(tz[0])?,nx[1].add(by[1])?.add(tz[1])?,nx[2].add(by[2])?.add(tz[2])?];
        let Some(sheared)=self.miter_offset(edge,fraction,encode(q))? else {return Ok(None);};
        let Some(sites)=&self.sites else {return Ok(None);};
        let q=decode(sheared)?;let f=I::new(fraction[0],fraction[1])?;
        let a=mul(point(sites[edge]),I::point(1.).sub(f)?)?;
        let b=mul(point(sites[(edge+1)%sites.len()]),f)?;
        Ok(Some(encode([a[0].add(b[0])?.add(q[0])?,a[1].add(b[1])?.add(q[1])?,a[2].add(b[2])?.add(q[2])?])))
    }
    /// Authored-frame interpolation remainder over an entire edge subinterval.
    /// Includes moving frame, twist, scale and the full path-miter field.
    /// Derivative rates map normalized traversal to the local interval.
    pub fn interpolation_upper_authored_frame(&self,edge:usize,fraction:[f64;2],local:VectorEnclosure,longitudinal:&crate::curve::Curve,transverse:&crate::curve::Curve,scale:&crate::curve::Curve,twist:&crate::curve::Curve,max_cells:usize)->Result<Option<f64>> {
        self.interpolation_upper_authored_frame_affine(edge,fraction,local,longitudinal,transverse,scale,twist,None,max_cells)
    }
    pub fn interpolation_upper_authored_frame_affine(&self,edge:usize,fraction:[f64;2],local:VectorEnclosure,longitudinal:&crate::curve::Curve,transverse:&crate::curve::Curve,scale:&crate::curve::Curve,twist:&crate::curve::Curve,affine:Option<(&crate::curve::Curve,&crate::curve::Curve)>,max_cells:usize)->Result<Option<f64>> {
        self.interpolation_upper_frame_affine(edge,fraction,local,FrameSource::Authored(longitudinal,transverse),scale,twist,affine,max_cells)
    }
    pub(super) fn interpolation_upper_frame_affine(
        &self,edge:usize,fraction:[f64;2],local:VectorEnclosure,
        source:FrameSource<'_>,
        scale:&crate::curve::Curve,twist:&crate::curve::Curve,affine:Option<(&crate::curve::Curve,&crate::curve::Curve)>,max_cells:usize,
    )->Result<Option<f64>> {
        check(local.iter().flatten().all(|x|x.is_finite()) && local[2]==[0.,0.],"Authored miter local profile must be finite and planar")?;
        let Some(traversal)=self.traversal(edge,fraction)? else {return Ok(None);};
        let frame=match source {
            FrameSource::Authored(longitudinal,transverse)=>super::authored_frame_certificate::certify_twisted(longitudinal,transverse,twist,traversal,max_cells)?,
            FrameSource::AuthoredGuide(axis,guide)=>{let (_,p,v)=self.edge_path_enclosures(edge,fraction)?;super::authored_frame_certificate::certify_authored_guide(axis,guide,twist,traversal,p,v,max_cells)?},
            FrameSource::Guide(guide)=>{let (t,p,v)=self.edge_path_enclosures(edge,fraction)?;super::authored_frame_certificate::certify_guide(guide,twist,traversal,t,p,v,max_cells)?},
        };
        if frame.status!=Status::Certified {return Ok(None);}
        let s=super::scalar_certificate::certify_traversal(scale,traversal,max_cells-frame.cells)?;
        let (Some(sv),Some(sd),Some(sdd))=(s.value,s.first,s.second) else {return Ok(None);};
        if sv[0]<=0. {return Ok(None);}
        let interval=|v:[f64;2]|I::new(v[0],v[1]);
        let width=I::point(fraction[1]).sub(I::point(fraction[0]))?;
        let lengths=self.cumulative_lengths.as_ref().unwrap();
        let rate=interval(lengths[edge+1])?.sub(interval(lengths[edge])?)?.div(interval(*lengths.last().unwrap())?)?.mul(width)?;
        let [a,b]=scale.domain();let sr=I::point(b).sub(I::point(a))?.mul(rate)?;
        let s0=interval(sv)?;let s1=interval(sd)?.mul(sr)?;let s2=interval(sdd)?.mul(sr.mul(sr)?)?;
        let n=frame.transverse.as_ref().unwrap();let b=frame.binormal.as_ref().unwrap();
        let combine=|n:VectorEnclosure,b:VectorEnclosure|->Result<V> {
            let nx=mul(decode(n)?,I::new(local[0][0],local[0][1])?)?;let by=mul(decode(b)?,I::new(local[1][0],local[1][1])?)?;
            Ok([nx[0].add(by[0])?,nx[1].add(by[1])?,nx[2].add(by[2])?])
        };
        let v=combine(n.value,b.value)?;
        let d=mul(combine(n.first,b.first)?,rate)?;
        let dd=mul(combine(n.second,b.second)?,rate.mul(rate)?)?;
        let q0=length(mul(v,s0)?)?;
        let vd=mul(v,s1)?;let ds=mul(d,s0)?;
        let q1=length([vd[0].add(ds[0])?,vd[1].add(ds[1])?,vd[2].add(ds[2])?])?;
        let vdd=mul(v,s2)?;let dds=mul(dd,s0)?;let mixed=mul(d,s1.mul(I::point(2.))?)?;
        let q2=length([vdd[0].add(dds[0])?.add(mixed[0])?,vdd[1].add(dds[1])?.add(mixed[1])?,vdd[2].add(dds[2])?.add(mixed[2])?])?;
        let mut smooth=frame.single_span && s.single_span;
        let (q0,q1,q2)=if let Some((axes,center))=affine {
            let a=super::vector_certificate::certify_traversal(axes,traversal,max_cells-frame.cells-s.cells,true)?;
            let c=super::vector_certificate::certify_traversal(center,traversal,max_cells-frame.cells-s.cells-a.cells,false)?;
            let (Some(av),Some(ad),Some(add),Some(cv),Some(cd),Some(cdd))=(a.value,a.first,a.second,c.value,c.first,c.second) else {return Ok(None);};
            smooth &= a.single_span && c.single_span;
            let domain_rate=|curve:&crate::curve::Curve|->Result<I>{let [a,b]=curve.domain();I::point(b).sub(I::point(a))?.mul(rate)};
            let ar=domain_rate(axes)?;let cr=domain_rate(center)?;
            let bases=[frame.transverse.as_ref().unwrap(),frame.binormal.as_ref().unwrap(),frame.longitudinal.as_ref().unwrap()];
            let mut q=[ [I::point(0.);3];3 ];
            for k in 0..3 {
                let offset=interval(local[k])?;
                let a0=interval(av[k])?;let a1=interval(ad[k])?.mul(ar)?;let a2=interval(add[k])?.mul(ar.mul(ar)?)?;
                let u0=offset.mul(s0)?.mul(a0)?.add(interval(cv[k])?)?;
                let u1=offset.mul(s1.mul(a0)?.add(s0.mul(a1)?)?)?.add(interval(cd[k])?.mul(cr)?)?;
                let u2=offset.mul(s2.mul(a0)?.add(I::point(2.).mul(s1)?.mul(a1)?)?.add(s0.mul(a2)?)?)?.add(interval(cdd[k])?.mul(cr.mul(cr)?)?)?;
                let v=decode(bases[k].value)?;let d=mul(decode(bases[k].first)?,rate)?;let dd=mul(decode(bases[k].second)?,rate.mul(rate)?)?;
                let v0=mul(v,u0)?;let v1=mul(v,u1)?;let d0=mul(d,u0)?;
                let v2=mul(v,u2)?;let d1=mul(d,u1.mul(I::point(2.))?)?;let dd0=mul(dd,u0)?;
                for j in 0..3 {q[0][j]=q[0][j].add(v0[j])?;q[1][j]=q[1][j].add(v1[j])?.add(d0[j])?;q[2][j]=q[2][j].add(v2[j])?.add(d1[j])?.add(dd0[j])?;}
            }
            (length(q[0])?,length(q[1])?,length(q[2])?)
        } else {(q0,q1,q2)};
        let tangent=decode(self.tangents.as_ref().unwrap()[edge])?;
        let planes=self.planes.as_ref().unwrap();
        let projection=|p:Option<VectorEnclosure>|->Result<V> {
            match p {None=>Ok([I::point(0.);3]),Some(p)=>{let h=decode(p)?;mul(h,I::point(1.).div(dot(tangent,h)?)?)}}
        };
        let v0=projection(planes[edge])?;let v1=projection(planes[(edge+1)%planes.len()])?;
        let h=I::point(length(v0)?.hi.max(length(v1)?.hi));let delta=length(sub(v1,v0)?)?.mul(width)?;
        let error=if smooth {
            I::point(1.).add(h)?.mul(q2)?.add(I::point(2.).mul(delta)?.mul(q1)?)?.div(I::point(8.))?
        } else {
            I::point(1.).add(h)?.mul(q1)?.add(delta.mul(q0)?)?.div(I::point(2.))?
        };
        Ok(Some(error.hi))
    }
    /// Outward interpolation remainder of the ideal control-point trajectory
    /// over one edge subinterval. Affine path translation cancels exactly.
    /// Both scalar certificates have independent max_cells work budgets.
    pub fn interpolation_upper(
        &self,
        edge: usize,
        fraction: [f64; 2],
        authored: [f64; 3],
        scale: &crate::curve::Curve,
        twist: &crate::curve::Curve,
        max_cells: usize,
    ) -> Result<Option<f64>> {
        self.interpolation_upper_affine(edge,fraction,authored,scale,twist,None,max_cells)
    }
    /// Outward derivative remainder including XYZ axis scale and local center.
    pub fn interpolation_upper_affine(
        &self,edge:usize,fraction:[f64;2],authored:[f64;3],scale:&crate::curve::Curve,
        twist:&crate::curve::Curve,affine:Option<(&crate::curve::Curve,&crate::curve::Curve)>,max_cells:usize,
    )->Result<Option<f64>> {
        check(
            authored.iter().all(|x| x.is_finite()),
            "Profile point must be finite",
        )?;
        let Some(traversal) = self.traversal(edge, fraction)? else {
            return Ok(None);
        };
        let s = super::scalar_certificate::certify_traversal(scale, traversal, max_cells)?;
        let t = super::scalar_certificate::certify_traversal(twist, traversal, max_cells)?;
        let (Some(sv), Some(sd), Some(sdd), Some(td), Some(tdd)) =
            (s.value, s.first, s.second, t.first, t.second)
        else {
            return Ok(None);
        };
        if sv[0] <= 0. {
            return Ok(None);
        }
        let scalar = |v: [f64; 2]| I::point(v[0].abs().max(v[1].abs()));
        let lengths = self.cumulative_lengths.as_ref().unwrap();
        let interval = |v: [f64; 2]| I::new(v[0], v[1]);
        let width = I::point(fraction[1]).sub(I::point(fraction[0]))?;
        let rate = interval(lengths[edge + 1])?
            .sub(interval(lengths[edge])?)?
            .div(interval(*lengths.last().unwrap())?)?
            .mul(width)?;
        let domain_rate = |c: &crate::curve::Curve| -> Result<I> {
            let [a, b] = c.domain();
            I::point(b).sub(I::point(a))?.mul(rate)
        };
        let sr = domain_rate(scale)?;
        let tr = domain_rate(twist)?;
        let s1 = scalar(sd).mul(sr)?;
        let s2 = scalar(sdd).mul(sr)?.mul(sr)?;
        let theta1 = scalar(td)
            .mul(tr)?
            .add(scalar(self.holonomy_angle.unwrap_or([0., 0.])).mul(rate)?)?;
        let theta2 = scalar(tdd).mul(tr)?.mul(tr)?;
        let normals = self.normals.as_ref().unwrap();
        let tangents = self.tangents.as_ref().unwrap();
        let initial = decode(normals[0])?;
        let profile = sub(point(authored), point(self.sites.as_ref().unwrap()[0]))?;
        let x = dot(profile, initial)?;
        let y = dot(profile, cross(decode(tangents[0])?, initial)?)?;
        let mut smooth=s.single_span && t.single_span;
        let (q0,q1,q2)=if let Some((axes,center))=affine {
            let a=super::vector_certificate::certify_traversal(axes,traversal,max_cells,true)?;
            let c=super::vector_certificate::certify_traversal(center,traversal,max_cells-a.cells,false)?;
            let (Some(av),Some(ad),Some(add),Some(cv),Some(cd),Some(cdd))=(a.value,a.first,a.second,c.value,c.first,c.second) else {return Ok(None);};
            smooth &= a.single_span && c.single_span;
            let ar=domain_rate(axes)?;let cr=domain_rate(center)?;
            let offsets=[scalar([x.lo,x.hi]),scalar([y.lo,y.hi]),I::point(0.)];
            let mut u0=[I::point(0.);3];let mut u1=u0;let mut u2=u0;
            for k in 0..3 {
                let a0=scalar(av[k]);let a1=scalar(ad[k]).mul(ar)?;let a2=scalar(add[k]).mul(ar)?.mul(ar)?;
                u0[k]=offsets[k].mul(scalar(sv))?.mul(a0)?.add(scalar(cv[k]))?;
                u1[k]=offsets[k].mul(s1.mul(a0)?.add(scalar(sv).mul(a1)?)?)?.add(scalar(cd[k]).mul(cr)?)?;
                u2[k]=offsets[k].mul(s2.mul(a0)?.add(I::point(2.).mul(s1)?.mul(a1)?)?.add(scalar(sv).mul(a2)?)?)?.add(scalar(cdd[k]).mul(cr)?.mul(cr)?)?;
            }
            let u0=length(u0)?;let u1=length(u1)?;let u2=length(u2)?;
            (u0,u1.add(u0.mul(theta1)?)?,u2.add(I::point(2.).mul(u1)?.mul(theta1)?)?.add(u0.mul(theta2.add(theta1.mul(theta1)?)?)?)?)
        } else {
            let radius = length([scalar([x.lo, x.hi]), scalar([y.lo, y.hi]), I::point(0.)])?;
            (radius.mul(scalar(sv))?,radius.mul(s1.add(scalar(sv).mul(theta1)?)?)?,radius.mul(
                s2.add(I::point(2.).mul(s1)?.mul(theta1)?)?
                    .add(scalar(sv).mul(theta2.add(theta1.mul(theta1)?)?)?)?,
            )?)
        };
        let tangent = decode(tangents[edge])?;
        let planes = self.planes.as_ref().unwrap();
        let transverse = |p: Option<VectorEnclosure>| -> Result<V> {
            match p {
                None => Ok([I::point(0.); 3]),
                Some(p) => {
                    let h = decode(p)?;
                    let projection=mul(h, I::point(1.).div(dot(tangent, h)?)?)?;
                    // With axial center, retain the full projection field:
                    // an open endpoint (no plane) has zero field, not t.
                    if affine.is_some(){Ok(projection)}else{sub(projection,tangent)}
                }
            }
        };
        let v0 = transverse(planes[edge])?;
        let v1 = transverse(planes[(edge + 1) % planes.len()])?;
        let h = I::point(length(v0)?.hi.max(length(v1)?.hi));
        let d = length(sub(v1, v0)?)?.mul(width)?;
        let error = if smooth {
            I::point(1.)
                .add(h)?
                .mul(q2)?
                .add(I::point(2.).mul(d)?.mul(q1)?)?
                .div(I::point(8.))?
        } else {
            I::point(1.)
                .add(h)?
                .mul(q1)?
                .add(d.mul(q0)?)?
                .div(I::point(2.))?
        };
        Ok(Some(error.hi))
    }
    /// Distributed closing correction; the scalar twist law is separate.
    pub fn correction_at(&self, edge: usize, fraction: [f64; 2]) -> Result<Option<[f64; 2]>> {
        let Some(t) = self.traversal(edge, fraction)? else {
            return Ok(None);
        };
        let a = self.holonomy_angle.unwrap_or([0., 0.]);
        let v = I::new(a[0], a[1])?.mul(I::new(t[0], t[1])?)?;
        Ok(Some([v.lo, v.hi]))
    }
}
/// Maximum Euclidean distance from a stored binary64 point to every point
/// in an ideal enclosure. This bound must be added to interpolation error.
pub fn stored_point_error_upper(stored: [f64; 3], ideal: VectorEnclosure) -> Result<f64> {
    check(
        stored.iter().all(|x| x.is_finite()),
        "Stored point must be finite",
    )?;
    let delta = sub(point(stored), decode(ideal)?)?;
    let mut square = I::point(0.);
    for v in delta {
        let radius = I::point(v.lo.abs().max(v.hi.abs()));
        square = square.add(radius.mul(radius)?)?;
    }
    let upper = square.hi.sqrt().next_up();
    check(
        upper.is_finite(),
        "Stored point error exceeds numeric range",
    )?;
    Ok(upper)
}
fn length(a: V) -> Result<I> {
    let square = dot(a, a)?;
    I::new(
        square.lo.max(0.).sqrt().next_down().max(0.),
        square.hi.max(0.).sqrt().next_up(),
    )
}
fn unit(a: V) -> Result<V> {
    let length = length(a)?;
    Ok([a[0].div(length)?, a[1].div(length)?, a[2].div(length)?])
}
fn corner(a: V, b: V, n: V, limit: f64) -> Result<(V, V)> {
    let h = unit([a[0].add(b[0])?, a[1].add(b[1])?, a[2].add(b[2])?])?;
    let denominator = dot(a, h)?;
    let amplification = I::point(1.).div(denominator)?;
    check(
        amplification.hi <= limit,
        "Miter limit not proved over the interval transport",
    )?;
    let r = sub(n, mul(h, dot(n, b)?.div(denominator)?)?)?;
    Ok((h, unit(sub(r, mul(b, dot(r, b)?)?)?)?))
}
fn encode(a: V) -> VectorEnclosure {
    a.map(|x| [x.lo, x.hi])
}
fn decode(a: VectorEnclosure) -> Result<V> {
    Ok([
        I::new(a[0][0], a[0][1])?,
        I::new(a[1][0], a[1][1])?,
        I::new(a[2][0], a[2][1])?,
    ])
}
fn point(a: [f64; 3]) -> V {
    a.map(I::point)
}
fn unresolved(cells: usize, reason: &'static str) -> Report {
    Report {
        status: Status::Unresolved,
        cells,
        tangents: None,
        normals: None,
        planes: None,
        closing_normal: None,
        holonomy_angle: None,
        cumulative_lengths: None,
        reason: Some(reason),
        sites: None,
    }
}
/// `cells` counts edge direction enclosures, the initial normal, and each
/// transported corner. A numeric/limit ambiguity is unresolved, never accepted.
pub fn certify(
    points: &[[f64; 3]],
    normal: [f64; 3],
    closed: bool,
    miter_limit: f64,
    max_cells: usize,
) -> Result<Report> {
    check(
        if closed {
            (3..=16).contains(&points.len())
        } else {
            (2..=17).contains(&points.len())
        },
        "Frame certificate needs2..17 open or3..16 closed sites",
    )?;
    check(
        points
            .iter()
            .flatten()
            .chain(normal.iter())
            .all(|x| x.is_finite()),
        "Frame certificate inputs must be finite",
    )?;
    check(
        miter_limit.is_finite() && miter_limit >= 1. && max_cells <= 100000,
        "Invalid frame certificate limit/budget",
    )?;
    let mut cells = 0;
    let mut tick = || -> Result<()> {
        if cells == max_cells {
            return Err(crate::resource("Frame certificate cell budget exhausted"));
        }
        cells += 1;
        Ok(())
    };
    let result = (|| -> Result<(Vec<V>, Vec<V>, Vec<Option<V>>, Option<V>, Option<[f64; 2]>, Vec<I>)> {
        let edges = points.len() - usize::from(!closed);
        let mut tangents = Vec::new();
        let mut lengths = vec![I::point(0.)];
        for i in 0..edges {
            tick()?;
            let delta = sub(point(points[(i + 1) % points.len()]), point(points[i]))?;
            lengths.push(lengths[i].add(length(delta)?)?);
            tangents.push(unit(delta)?);
        }
        tick()?;
        let authored = unit(point(normal))?;
        let initial = unit(sub(
            authored,
            mul(tangents[0], dot(authored, tangents[0])?)?,
        )?)?;
        let mut normals = vec![initial];
        let mut planes = vec![None; points.len()];
        for i in 1..edges {
            tick()?;
            let (plane, n) = corner(tangents[i - 1], tangents[i], normals[i - 1], miter_limit)?;
            planes[i] = Some(plane);
            normals.push(n);
        }
        let closing = if closed {
            tick()?;
            let (plane, n) = corner(
                tangents[edges - 1],
                tangents[0],
                normals[edges - 1],
                miter_limit,
            )?;
            planes[0] = Some(plane);
            Some(n)
        } else {
            None
        };
        let angle = if let Some(end) = closing {
            tick()?;
            let sine = dot(tangents[0], cross(end, initial)?)?;
            let cosine = dot(end, initial)?;
            Some(
                super::trigonometric_certificate::certify_atan2(
                    [sine.lo, sine.hi],
                    [cosine.lo, cosine.hi],
                )?
                .ok_or_else(|| crate::resource("Holonomy angle enclosure unresolved"))?,
            )
        } else {
            None
        };
        Ok((tangents, normals, planes, closing, angle, lengths))
    })();
    match result {
        Ok((tangents, normals, planes, closing, angle, lengths)) => Ok(Report {
            status: Status::Certified,
            cells,
            tangents: Some(tangents.into_iter().map(encode).collect()),
            normals: Some(normals.into_iter().map(encode).collect()),
            planes: Some(planes.into_iter().map(|p| p.map(encode)).collect()),
            closing_normal: closing.map(encode),
            holonomy_angle: angle,
            cumulative_lengths: Some(lengths.into_iter().map(|v| [v.lo, v.hi]).collect()),
            reason: None,
            sites: Some(points.to_vec()),
        }),
        Err(error) => Ok(unresolved(
            cells,
            if error.code == crate::RESOURCE_LIMIT && cells == max_cells {
                "cell-budget-exhausted"
            } else {
                "transport-enclosure-unresolved"
            },
        )),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn contains(v: VectorEnclosure, p: [f64; 3]) {
        for (range, x) in v.into_iter().zip(p) {
            assert!(range[0] <= x && x <= range[1], "{x} outside {range:?}");
        }
    }
    #[test]
    fn authored_remainder_covers_rotation_and_axial_miter_scale_product() {
        let constant=|v:[f64;3]|crate::paths::bezier(vec![v.to_vec(),v.to_vec()],None).unwrap();
        let scale=constant([1.,0.,0.]);let longitudinal=constant([0.,0.,1.]);let transverse=constant([1.,0.,0.]);
        let twist=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,0.,0.]],None).unwrap();
        let straight=certify(&[[0.,0.,0.],[0.,0.,10.]],[1.,0.,0.],false,2.,10).unwrap();
        let bound=straight.interpolation_upper_authored_frame(0,[0.,1.],[[1.,1.],[0.,0.],[0.,0.]],&longitudinal,&transverse,&scale,&twist,8).unwrap().unwrap();
        let actual=((0.5_f64.cos()-(1.+1_f64.cos())/2.).powi(2)+(0.5_f64.sin()-1_f64.sin()/2.).powi(2)).sqrt();
        assert!(bound>=actual && bound<0.18);
        let refined=straight.interpolation_upper_authored_frame(0,[0.,0.5],[[1.,1.],[0.,0.],[0.,0.]],&longitudinal,&transverse,&scale,&twist,8).unwrap().unwrap();
        assert!(refined<bound*0.3);
        let kinked=crate::curve::Curve {degree:1,knots:vec![0.,0.,0.5,1.,1.],control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![0.,0.,0.]],weights:vec![1.;3],periodic:false};
        let frame=super::super::authored_frame_certificate::certify_twisted(&longitudinal,&transverse,&kinked,[0.,1.],8).unwrap();
        assert!(!frame.single_span);
        let kink_bound=straight.interpolation_upper_authored_frame(0,[0.,1.],[[1.,1.],[0.,0.],[0.,0.]],&longitudinal,&transverse,&scale,&kinked,9).unwrap().unwrap();
        // The midpoint has angle1 while both endpoints have angle0; crossing
        // the derivative jump requires the first-derivative remainder.
        assert!(kink_bound>=2.*0.5_f64.sin());
        let corner=certify(&[[0.,0.,0.],[0.,0.,10.],[10.,0.,10.]],[1.,0.,0.],false,2.,10).unwrap();
        let longitudinal=constant([1.,0.,0.]);let transverse=constant([0.,0.,1.]);let twist=constant([0.,0.,0.]);
        let scale=crate::paths::bezier(vec![vec![1.,0.,0.],vec![2.,0.,0.]],None).unwrap();
        let bound=corner.interpolation_upper_authored_frame(0,[0.,1.],[[1.,1.],[0.,0.],[0.,0.]],&longitudinal,&transverse,&scale,&twist,8).unwrap().unwrap();
        assert!(bound>=0.125 && bound<(2_f64.sqrt()/8.+1e-10),"{bound}");
        assert!(corner.interpolation_upper_authored_frame(0,[0.,1.],[[1.,1.],[0.,0.],[0.,0.]],&longitudinal,&transverse,&scale,&twist,7).unwrap().is_none());
    }
    #[test]
    fn authored_station_keeps_path_projection_and_one_law_budget() {
        let r=certify(&[[0.,0.,0.],[0.,0.,10.],[10.,0.,10.]],[1.,0.,0.],false,2.,10).unwrap();
        let constant=|v:[f64;3]|crate::paths::bezier(vec![v.to_vec(),v.to_vec()],None).unwrap();
        let longitudinal=constant([1.,0.,0.]);
        let transverse=constant([0.,0.,1.]);
        let scale=constant([1.,0.,0.]);let twist=constant([0.,0.,0.]);
        let p=r.station_local_authored_frame(0,[0.5,0.5],[[1.,1.],[0.,0.],[0.,0.]],&longitudinal,&transverse,&scale,&twist,8).unwrap().unwrap();
        // q is axial to the path but transverse to the authored frame.
        // At the halfway station the end-plane miter subtracts half of q.
        contains(p,[0.,0.,5.5]);
        assert!(r.station_local_authored_frame(0,[0.5,0.5],[[1.,1.],[0.,0.],[0.,0.]],&longitudinal,&transverse,&scale,&twist,7).unwrap().is_none());
        assert!(r.station_local_authored_frame(0,[0.5,0.5],[[1.,1.],[0.,0.],[0.1,0.1]],&longitudinal,&transverse,&scale,&twist,8).is_err());
    }
    #[test]
    fn independent_right_angle_minimum_rotation() {
        let r = certify(
            &[[0., 0., 0.], [0., 0., 10.], [10., 0., 10.]],
            [1., 0., 0.],
            false,
            2.,
            10,
        )
        .unwrap();
        assert_eq!(r.status, Status::Certified);
        contains(r.tangents.as_ref().unwrap()[0], [0., 0., 1.]);
        contains(r.tangents.as_ref().unwrap()[1], [1., 0., 0.]);
        contains(r.normals.as_ref().unwrap()[0], [1., 0., 0.]);
        contains(r.normals.as_ref().unwrap()[1], [0., 0., -1.]);
        contains(
            r.planes.as_ref().unwrap()[1].unwrap(),
            [
                std::f64::consts::FRAC_1_SQRT_2,
                0.,
                std::f64::consts::FRAC_1_SQRT_2,
            ],
        );
    }
    #[test]
    fn skew_closed_transport_encloses_holonomy_angle() {
        let points = [
            [0., 0., 0.],
            [10., 0., 0.],
            [10., 10., 4.],
            [0., 10., 1.],
            [0., 5., -2.],
        ];
        let r = certify(&points, [0., 0., 1.], true, 4., 32).unwrap();
        assert_eq!(r.status, Status::Certified);
        assert!(r.closing_normal.is_some());
        let angle = r.holonomy_angle.unwrap();
        assert!(angle[0].abs() > 1e-3 && angle[1] - angle[0] < 1e-8);
        assert_eq!(r.tangents.unwrap().len(), 5);
        assert_eq!(r.normals.unwrap().len(), 5);
        assert!(r.planes.unwrap()[0].is_some());
        let exhausted = certify(&points, [0., 0., 1.], true, 4., 11).unwrap();
        assert_eq!(exhausted.status, Status::Unresolved);
        assert_eq!(exhausted.reason, Some("cell-budget-exhausted"));
        assert!(exhausted.holonomy_angle.is_none() && exhausted.closing_normal.is_none());
    }
    #[test]
    fn degeneracy_limits_and_budget_discard_partial_frames() {
        for (points, normal, limit, budget) in [
            (vec![[0., 0., 0.], [0., 0., 0.]], [1., 0., 0.], 4., 10),
            (vec![[0., 0., 0.], [0., 0., 1.]], [0., 0., 1.], 4., 10),
            (
                vec![[0., 0., 0.], [0., 0., 1.], [0., 0., 0.]],
                [1., 0., 0.],
                4.,
                10,
            ),
            (
                vec![[0., 0., 0.], [0., 0., 1.], [1., 0., 1.]],
                [1., 0., 0.],
                1.1,
                10,
            ),
            (vec![[0., 0., 0.], [0., 0., 1.]], [1., 0., 0.], 4., 0),
        ] {
            let r = certify(&points, normal, false, limit, budget).unwrap();
            assert_eq!(r.status, Status::Unresolved);
            assert!(r.tangents.is_none() && r.normals.is_none() && r.planes.is_none());
        }
    }
    #[test]
    fn traversal_encloses_exact_rational_chord_ratios() {
        let r = certify(
            &[[0., 0., 0.], [3., 0., 0.], [3., 4., 0.]],
            [0., 0., 1.],
            false,
            2.,
            10,
        )
        .unwrap();
        let t = r.traversal(1, [0.5, 0.5]).unwrap().unwrap();
        assert!(t[0] <= 5. / 7. && t[1] >= 5. / 7. && t[1] - t[0] < 1e-12);
        let c = r.correction_at(1, [0., 1.]).unwrap().unwrap();
        assert!(c[0] <= 0. && c[1] >= 0.);
        assert!(r.traversal(2, [0., 1.]).is_err());
        assert!(r.traversal(usize::MAX, [0., 1.]).is_err());
        assert!(r.traversal(0, [-0.1, 1.]).is_err());
        let u = certify(&[[0., 0., 0.], [3., 0., 0.]], [0., 0., 1.], false, 2., 0).unwrap();
        assert!(u.correction_at(0, [0., 1.]).unwrap().is_none());
    }
    #[test]
    fn rotated_frame_encloses_exact_axes_and_continuous_twist_range() {
        let r = certify(&[[0., 0., 0.], [0., 0., 10.]], [1., 0., 0.], false, 4., 10).unwrap();
        let (n, b) = r.rotated_frame(0, [0.25, 0.75], [0., 0.]).unwrap().unwrap();
        contains(n, [1., 0., 0.]);
        contains(b, [0., 1., 0.]);
        let (n, b) = r.rotated_frame(0, [0., 1.], [-0.2, 0.3]).unwrap().unwrap();
        for i in 0..101 {
            let a: f64 = -0.2 + 0.5 * i as f64 / 100.;
            contains(n, [a.cos(), a.sin(), 0.]);
            contains(b, [-a.sin(), a.cos(), 0.]);
        }
        let u = certify(&[[0., 0., 0.], [0., 0., 10.]], [1., 0., 0.], false, 4., 0).unwrap();
        assert!(u.rotated_frame(0, [0., 1.], [0., 1.]).unwrap().is_none());
    }
    #[test]
    fn miter_shear_encloses_exact_corner_plane_and_open_endpoint() {
        let r = certify(
            &[[0., 0., 0.], [0., 0., 10.], [10., 0., 10.]],
            [1., 0., 0.],
            false,
            2.,
            10,
        )
        .unwrap();
        let q = [[1., 1.], [0., 0.], [0., 0.]];
        contains(
            r.miter_offset(0, [0., 0.], q).unwrap().unwrap(),
            [1., 0., 0.],
        );
        contains(
            r.miter_offset(0, [1., 1.], q).unwrap().unwrap(),
            [1., 0., -1.],
        );
        let range = r.miter_offset(0, [0.2, 0.8], q).unwrap().unwrap();
        for i in 0..101 {
            let f = 0.2 + 0.6 * i as f64 / 100.;
            contains(range, [1., 0., -f]);
        }
        // The outgoing transported offset reaches exactly the same plane.
        contains(
            r.miter_offset(1, [0., 0.], [[0., 0.], [0., 0.], [-1., -1.]])
                .unwrap()
                .unwrap(),
            [1., 0., -1.],
        );
        let u = certify(&[[0., 0., 0.], [0., 0., 10.]], [1., 0., 0.], false, 2., 0).unwrap();
        assert!(u.miter_offset(0, [0., 1.], q).unwrap().is_none());
    }
    #[test]
    fn station_composition_preserves_translated_profile_and_corner() {
        let r = certify(
            &[[100., 200., 300.], [100., 200., 310.], [110., 200., 310.]],
            [1., 0., 0.],
            false,
            2.,
            10,
        )
        .unwrap();
        contains(
            r.station_point(0, [1., 1.], [101., 202., 300.], [2., 2.], [0., 0.])
                .unwrap()
                .unwrap(),
            [102., 204., 308.],
        );
        let box_range = r
            .station_point(0, [0.2, 0.8], [101., 202., 300.], [1., 2.], [0., 0.])
            .unwrap()
            .unwrap();
        for f in [0.2, 0.5, 0.8] {
            for scale in [1., 1.5, 2.] {
                contains(
                    box_range,
                    [100. + scale, 200. + 2. * scale, 300. + 10. * f - f * scale],
                );
            }
        }
        assert!(
            r.station_point(0, [0., 0.], [101., 202., 300.], [0., 1.], [0., 0.])
                .is_err()
        );
    }
    #[test]
    fn stored_error_bound_covers_ideal_box_and_displaced_point() {
        let ideal = [[0., 0.], [0., 0.], [0., 0.]];
        let bound = stored_point_error_upper([3., 4., 0.], ideal).unwrap();
        assert!(bound >= 5. && bound < 5.000000000001);
        let bound =
            stored_point_error_upper([0., 0., 0.], [[-3., 2.], [-4., 1.], [0., 0.]]).unwrap();
        assert!(bound >= 5. && bound < 5.000000000001);
        assert!(stored_point_error_upper([f64::NAN, 0., 0.], ideal).is_err());
    }
    #[test]
    fn affine_station_projects_the_same_axis_scale_and_center_into_corner_plane(){
        let r=certify(&[[0.,0.,0.],[0.,0.,10.],[10.,0.,10.]],[1.,0.,0.],false,2.,10).unwrap();
        let axes=[[2.,2.],[3.,3.],[1.,1.]];
        let center=[[0.5,0.5],[-1.,-1.],[2.,2.]];
        // q=(4.5,11,2), projection onto x+z=10 removes q.x+q.z.
        contains(r.station_point_affine(0,[1.,1.],[1.,2.,0.],[2.,2.],[0.,0.],axes,center).unwrap().unwrap(),[4.5,11.,5.5]);
        // At the open endpoint there is no miter projection.
        contains(r.station_point_affine(0,[0.,0.],[1.,2.,0.],[2.,2.],[0.,0.],axes,center).unwrap().unwrap(),[4.5,11.,2.]);
        assert!(r.station_point_affine(0,[0.,0.],[1.,2.,0.],[2.,2.],[0.,0.],[[0.,1.];3],center).is_err());
        let vector=|value:[f64;3]|crate::curve::Curve {degree:1,knots:vec![2.,2.,5.,5.],control_points:vec![value.to_vec();2],weights:vec![1.;2],periodic:false};
        let scalar=|value:f64|vector([value,0.,0.]);
        let axes=vector([2.,3.,1.]);let center=vector([0.5,-1.,2.]);
        contains(r.station_with_affine_laws(0,[1.,1.],[1.,2.,0.],&scalar(2.),&scalar(0.),Some((&axes,&center)),6).unwrap().unwrap(),[4.5,11.,5.5]);
        assert!(r.station_with_affine_laws(0,[1.,1.],[1.,2.,0.],&scalar(2.),&scalar(0.),Some((&axes,&center)),5).unwrap().is_none());
    }
    #[test]
    fn ordinary_affine_endpoint_uses_values_without_zero_width_jets() {
        let scalar=|v|crate::curve::Curve{degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![v,0.,0.];2],weights:vec![1.;2],periodic:false};
        let axes=crate::curve::Curve{control_points:vec![vec![2.,1.,1.],vec![3.,2.,1.]],..scalar(1.)};
        let center=crate::curve::Curve{control_points:vec![vec![0.125,0.,0.],vec![0.5,0.25,0.125]],..scalar(0.)};
        let frame=certify(&[[0.,0.,0.],[0.,0.,10.]],[1.,0.,0.],false,2.,10).unwrap();
        for (t,expected) in [(0.,[2.125,2.,0.]),(1.,[3.5,4.25,10.125])] {
            contains(frame.station_with_affine_laws(0,[t,t],[1.,2.,0.],&scalar(1.),&scalar(0.),Some((&axes,&center)),6).unwrap().unwrap(),expected);
            assert!(frame.station_with_affine_laws(0,[t,t],[1.,2.,0.],&scalar(1.),&scalar(0.),Some((&axes,&center)),5).unwrap().is_none());
        }
    }
    #[test]
    fn affine_remainder_covers_center_curvature_and_axial_projection_derivative(){
        let scalar=|value:f64|crate::curve::Curve {degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![value,0.,0.];2],weights:vec![1.;2],periodic:false};
        let axes=crate::curve::Curve {control_points:vec![vec![1.,1.,1.];2],..scalar(1.)};
        let center=crate::curve::Curve {degree:2,knots:vec![2.,2.,2.,5.,5.,5.],control_points:vec![vec![0.,0.,0.],vec![0.,0.,0.],vec![0.,0.,2.]],weights:vec![1.;3],periodic:false};
        let r=certify(&[[0.,0.,0.],[0.,0.,10.]],[1.,0.,0.],false,2.,10).unwrap();
        let bound=r.interpolation_upper_affine(0,[0.,1.],[1.,0.,0.],&scalar(1.),&scalar(0.),Some((&axes,&center)),6).unwrap().unwrap();
        assert!(bound>=0.5 && bound<0.50000000001);
        for i in 0..=100 {let f=i as f64/100.;assert!((2.*f*f-2.*f).abs()<=bound);}
        assert!(r.interpolation_upper_affine(0,[0.,1.],[1.,0.,0.],&scalar(1.),&scalar(0.),Some((&axes,&center)),5).unwrap().is_none());
        // Collinear interior station still has a normal miter plane. A varying
        // axial center is projected away there but remains at the open site.
        let r=certify(&[[0.,0.,0.],[0.,0.,10.],[0.,0.,20.]],[1.,0.,0.],false,2.,10).unwrap();
        let center=crate::curve::Curve {control_points:vec![vec![0.,0.,0.],vec![0.,0.,4.]],..scalar(0.)};
        let bound=r.interpolation_upper_affine(0,[0.,1.],[1.,0.,0.],&scalar(1.),&scalar(0.),Some((&axes,&center)),6).unwrap().unwrap();
        assert!(bound>=0.5 && bound<0.50000000001);
        for i in 0..=100 {let f=i as f64/100.;assert!((2.*f*(1.-f)).abs()<=bound);}
    }
    #[test]
    fn integrated_station_laws_preserve_budget_refusal() {
        let r = certify(&[[0., 0., 0.], [0., 0., 10.]], [1., 0., 0.], false, 4., 10).unwrap();
        let scale = crate::paths::bezier(vec![vec![1., 0., 0.], vec![2., 0., 0.]], None).unwrap();
        let twist = crate::paths::bezier(vec![vec![0., 0., 0.], vec![0., 0., 0.]], None).unwrap();
        let point = r
            .station_with_laws(0, [0.5, 0.5], [1., 2., 0.], &scale, &twist, 1)
            .unwrap()
            .unwrap();
        contains(point, [1.5, 3., 5.]);
        assert!(
            r.station_with_laws(0, [0.5, 0.5], [1., 2., 0.], &scale, &twist, 0)
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn interpolation_certificate_bounds_sinusoidal_midpoint_error() {
        let r = certify(&[[0., 0., 0.], [0., 0., 10.]], [1., 0., 0.], false, 4., 10).unwrap();
        let scale = crate::paths::bezier(vec![vec![1., 0., 0.], vec![1., 0., 0.]], None).unwrap();
        let twist = crate::paths::bezier(vec![vec![0., 0., 0.], vec![1., 0., 0.]], None).unwrap();
        let e = r
            .interpolation_upper(0, [0., 1.], [1., 0., 0.], &scale, &twist, 1)
            .unwrap()
            .unwrap();
        let actual = ((0.5_f64.cos() - (1. + 1_f64.cos()) / 2.).powi(2)
            + (0.5_f64.sin() - 1_f64.sin() / 2.).powi(2))
        .sqrt();
        assert!(e >= actual && e < 0.126);
        assert!(
            r.interpolation_upper(0, [0., 1.], [1., 0., 0.], &scale, &twist, 0)
                .unwrap()
                .is_none()
        );
    }
}
