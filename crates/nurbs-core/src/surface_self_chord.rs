//! Sufficient whole-chart exclusion of chords aligned with endpoint normals.
//! Original rational derivative bounds cover every nonempty knot rectangle.
use crate::{check, distance_bounds::Interval as I, normal_alignment, surface::Surface, Result};

/// Algebraic self-chord exclusion from an enclosed Jacobian.
/// This does not issue a geometry certificate. The caller must establish that
/// these derivatives cover a continuous chart on the convex endpoint domain.
pub fn inspect_jacobian_self_chords(
    jacobian: [[[f64; 2]; 2]; 3],
    max_sine_squared: f64,
) -> Result<(bool, f64)> {
    check(max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared),
        "Self chord exclusion requires squared sine in [0,1)")?;
    let mut j = [[I::point(0.); 2]; 3];
    for k in 0..3 {
        for axis in 0..2 {
            let bounds = jacobian[k][axis];
            check(bounds.iter().all(|x| x.is_finite()) && bounds[0] <= bounds[1],
                "Self chord Jacobian requires finite ordered bounds")?;
            j[k][axis] = I::new(bounds[0], bounds[1])?;
        }
    }
    let upper = balanced_cosine_upper(j)?;
    let threshold = I::point(1.).sub(I::point(max_sine_squared))?;
    Ok((upper < threshold.lo, upper))
}

#[cfg(test)]
#[path="surface_self_chord/tests/expression_jacobian_tests.rs"]
mod expression_jacobian_tests;
/// Algebraic endpoint-specific mean-path exclusion. This diagnostic carries
/// no source authority: callers must prove continuity of the enclosed path
/// and bind the endpoint normal to the same immutable expression.
pub fn inspect_jacobian_endpoint_chords(
    jacobian: [[[f64;2];2];3], normal:[[f64;2];3], max_sine_squared:f64,
) -> Result<(bool,f64)> {
    check(max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared)
        && jacobian.iter().flatten().chain(normal.iter()).all(|d|
            d.iter().all(|x|x.is_finite()) && d[0]<=d[1]),
        "Bound mean Jacobian, endpoint normal and angular tolerance")?;
    let mut j=[[I::point(0.);2];3];
    let mut n=[I::point(0.);3];
    for k in 0..3 {
        n[k]=I::new(normal[k][0],normal[k][1])?;
        for axis in 0..2 {j[k][axis]=I::new(jacobian[k][axis][0],jacobian[k][axis][1])?;}
    }
    // Fixed positive parameter rescaling preserves every connecting path;
    // normal magnitude likewise has no effect on its unoriented angle.
    for axis in 0..2 {
        let scale=jacobian.iter().map(|row|row[axis][0].abs().max(row[axis][1].abs())).fold(0_f64,f64::max);
        if scale>=f64::MIN_POSITIVE {
            for row in &mut j {row[axis]=row[axis].div(I::point(scale))?;}
        }
    }
    let scale=normal.iter().flatten().map(|x|x.abs()).fold(0_f64,f64::max);
    if scale>=f64::MIN_POSITIVE {
        for value in &mut n {*value=value.div(I::point(scale))?;}
    }
    let upper=endpoint_matrix_cosine(j,n)?;
    let threshold=I::point(1.).sub(I::point(max_sine_squared))?.lo;
    Ok((upper<threshold,upper))
}
/// Endpoint-domain chord enclosure from an enclosed continuous-path matrix.
/// This diagnostic issues no source certificate; the caller owns continuity
/// and binds both parameter rectangles and the normal to that path.
pub fn inspect_jacobian_endpoint_pair_chords(
    jacobian:[[[f64;2];2];3],pair:[[[f64;2];2];2],normal:[[f64;2];3],sine:f64,
) -> Result<bool> {
    check(pair.iter().flatten().all(|d|d.iter().all(|x|x.is_finite()) && d[0]<=d[1])
        && jacobian.iter().flatten().all(|d|d.iter().all(|x|x.is_finite()) && d[0]<=d[1]),
        "Finite ordered chord domains and mean Jacobian")?;
    let mut delta=[I::point(0.);2];
    // Common positive scales preserve M*(B-A)'s direction. Columns must
    // not be scaled independently without also changing parameter deltas.
    let parameter_scale=pair.iter().flatten().flatten().map(|x|x.abs())
        .fold(0_f64,f64::max).max(f64::MIN_POSITIVE);
    let matrix_scale=jacobian.iter().flatten().flatten().map(|x|x.abs())
        .fold(0_f64,f64::max).max(f64::MIN_POSITIVE);
    for axis in 0..2 {
        delta[axis]=I::new(pair[1][axis][0],pair[1][axis][1])?
            .div(I::point(parameter_scale))?
            .sub(I::new(pair[0][axis][0],pair[0][axis][1])?.div(I::point(parameter_scale))?)?;
    }
    let mut chord=[[0.;2];3];
    for k in 0..3 {
        let mut value=I::point(0.);
        for axis in 0..2 {
            value=value.add(I::new(jacobian[k][axis][0],jacobian[k][axis][1])?
                .div(I::point(matrix_scale))?.mul(delta[axis])?)?;
        }
        chord[k]=[value.lo,value.hi];
    }
    Ok(normal_alignment::inspect_pair_bounds([normal,chord],sine)?.aligned==Some(false))
}

/// Algebraic exclusion for a three-leg path whose transverse coefficients
/// have the same sign. Columns are source transverse, free, bridge transverse.
/// The caller owns complete path bounds, continuity and the endpoint normal.
/// Only nonzero chords are candidates; this issues no geometry certificate.
pub fn inspect_same_sign_transverse_chords(
    columns: [[[f64;2];3];3], transverse: [[f64;2];2], source_scale:f64,
    normal:[[f64;2];3], sine:f64,
) -> Result<bool> {
    check(source_scale.is_finite() && source_scale>0.
        && sine.is_finite() && (0. ..1.).contains(&sine)
        && columns.iter().flatten().chain(transverse.iter()).chain(normal.iter())
            .all(|d|d.iter().all(|x|x.is_finite()) && d[0]<=d[1]),
        "Finite path columns, signed coefficients, positive scale and endpoint normal")?;
    if !(transverse.iter().all(|d|d[0]>=0.) || transverse.iter().all(|d|d[1]<=0.)) {
        return Ok(false);
    }
    // With a=delta_source/scale and b=delta_bridge, q=a+b has their
    // common sign. For q!=0, a/q and b/q are convex weights. For q=0
    // both transverse legs vanish. The free coefficient is unrestricted.
    let mut matrix=[[[0.;2];2];3];
    for k in 0..3 {
        let source=I::new(columns[0][k][0],columns[0][k][1])?.mul(I::point(source_scale))?;
        matrix[k][0]=columns[1][k];
        matrix[k][1]=[source.lo.min(columns[2][k][0]),source.hi.max(columns[2][k][1])];
    }
    Ok(inspect_jacobian_rank_bound(matrix)?
        && inspect_jacobian_endpoint_chords(matrix,normal,sine)?.0)
}

#[cfg(test)]
#[path="surface_self_chord/tests/transverse_cone_tests.rs"]
mod transverse_cone_tests;

/// Sufficient rank bound for every enclosed connecting-path differential.
/// False means this interval matrix cannot support the mean-path theorem,
/// not that the underlying geometry is singular.
pub fn inspect_jacobian_rank_bound(jacobian:[[[f64;2];2];3]) -> Result<bool> {
    check(jacobian.iter().flatten().all(|d|d.iter().all(|x|x.is_finite()) && d[0]<=d[1]),
        "Finite ordered mean Jacobian")?;
    let mut j=[[I::point(0.);2];3];
    for axis in 0..2 {
        let scale=jacobian.iter().map(|r|r[axis][0].abs().max(r[axis][1].abs())).fold(0_f64,f64::max);
        for k in 0..3 {
            j[k][axis]=I::new(jacobian[k][axis][0],jacobian[k][axis][1])?;
            if scale>=f64::MIN_POSITIVE {j[k][axis]=j[k][axis].div(I::point(scale))?;}
        }
    }
    Ok(mean_matrix_lower(j)?>0.)
}
#[cfg(test)]
#[path="surface_self_chord/tests/endpoint_jacobian_tests.rs"]
mod endpoint_jacobian_tests;

#[derive(Clone)]
pub struct Certificate<'a> {
    surface: &'a Surface,
    domain: [[f64; 2]; 2],
    max_sine_squared: f64,
    cosine_squared_upper: f64,
}
impl<'a> Certificate<'a> {
    pub fn surface(&self) -> &'a Surface {
        self.surface
    }
    pub fn domain(&self) -> [[f64; 2]; 2] {
        self.domain
    }
    pub fn max_sine_squared(&self) -> f64 {
        self.max_sine_squared
    }
    pub fn cosine_squared_upper(&self) -> f64 {
        self.cosine_squared_upper
    }
}
/// Original derivative hull on a completely covered continuous rectangle.
/// Private fields prevent rebinding a mean-path enclosure to another source.
#[derive(Clone)]
pub(crate) struct MeanMatrix<'a> {
    surface: &'a Surface,
    domain: [[f64; 2]; 2],
    jacobian: [[I; 2]; 3],
}
pub(crate) struct CoordinatePath<'a>{matrix:MeanMatrix<'a>,pair:[[[f64;2];2];2]}
impl CoordinatePath<'_>{
    pub(crate) fn chord_bounds(&self)->Result<Vec<[f64;2]>> {
        let delta=[I::new(self.pair[1][0][0],self.pair[1][0][1])?.sub(I::new(self.pair[0][0][0],self.pair[0][0][1])?)?,
            I::new(self.pair[1][1][0],self.pair[1][1][1])?.sub(I::new(self.pair[0][1][0],self.pair[0][1][1])?)?];
        (0..3).map(|k|{let d=self.matrix.jacobian[k][0].mul(delta[0])?.add(self.matrix.jacobian[k][1].mul(delta[1])?)?;Ok([d.lo,d.hi])}).collect()
    }
    pub(crate) fn exclude_pair(&self,pair:[[[f64;2];2];2],sine:f64,max_spans:usize,
        cached:Option<(&crate::factored_surface_normals::Certificate<'_>,&crate::factored_surface_normals::PairBounds<'_>)>,
        original:Option<&normal_alignment::OriginalPairBounds<'_>>)->Result<(bool,usize)> {
        check(pair==self.pair||pair==[self.pair[1],self.pair[0]],"Keep original coordinate path endpoints")?;
        self.matrix.exclude_pair(pair,sine,max_spans,cached,original)
    }
}

#[derive(Clone)]
pub struct Report<'a> {
    pub certificate: Option<Certificate<'a>>,
    pub(crate) mean_matrix: Option<MeanMatrix<'a>>,
    pub spans: usize,
    pub complete: bool,
    pub cosine_squared_upper: Option<f64>,
    pub reason: &'static str,
}
fn separated_square(x: I) -> Result<Option<I>> {
    let lower = if x.lo > 0. {
        x.lo
    } else if x.hi < 0. {
        -x.hi
    } else {
        return Ok(None);
    };
    let upper = x.lo.abs().max(x.hi.abs());
    Ok(Some(I::new(
        (lower * lower).next_down().max(0.),
        (upper * upper).next_up(),
    )?))
}
fn gram_bounds(j: [[I; 2]; 3]) -> Result<([I; 2], [I; 2], I)> {
    let mut upper = [I::point(0.); 2];
    let mut lower = [I::point(0.); 2];
    let mut off = I::point(0.);
    for row in j {
        for a in 0..2 {
            if let Some(square) = separated_square(row[a])? { lower[a] = lower[a].add(square)?; }
            let magnitude = row[a].lo.abs().max(row[a].hi.abs());
            upper[a] = upper[a].add(I::point(magnitude).mul(I::point(magnitude))?)?;
        }
        off = off.add(row[0].mul(row[1])?)?;
    }
    Ok((upper, lower, off))
}
fn direct_gram_lower(lower: [I; 2], off_upper: f64) -> Result<f64> {
    let d = lower.map(|x| x.lo.max(0.));
    // Gram entries already contain squared lengths. Normalize before another
    // square so valid large coordinate units cannot overflow the discriminant.
    let scale = d[0].max(d[1]).max(off_upper).max(f64::MIN_POSITIVE);
    let d = [I::point(d[0]).div(I::point(scale))?.lo.max(0.),
        I::point(d[1]).div(I::point(scale))?.lo.max(0.)];
    let off = I::point(off_upper).div(I::point(scale))?.hi;
    let difference = I::point(d[0]).sub(I::point(d[1]))?;
    let magnitude = difference.lo.abs().max(difference.hi.abs());
    let discriminant = I::point(magnitude).mul(I::point(magnitude))?
        .add(I::point(off).mul(I::point(off))?.mul(I::point(4.))?)?;
    let root_upper = discriminant.hi.sqrt().next_up();
    Ok(I::point(d[0]).add(I::point(d[1]))?.sub(I::point(root_upper))?
        .mul(I::point(0.5))?.mul(I::point(scale))?.lo.max(0.))
}


fn endpoint_matrix_cosine(j: [[I; 2]; 3], n: [I; 3]) -> Result<f64> {
    let singular_lower=mean_matrix_lower(j)?;
    if singular_lower<=0. {return Ok(1.);}
    let mut normal_squared=I::point(0.);
    for value in n {
        if let Some(square)=separated_square(value)? {normal_squared=normal_squared.add(square)?;}
    }
    if normal_squared.lo<=0. {return Ok(1.);}
    let mut numerator = I::point(0.);
    for a in 0..2 {
        let mut dot = I::point(0.);
        for k in 0..3 {
            dot = dot.add(n[k].mul(j[k][a])?)?;
        }
        let upper = dot.lo.abs().max(dot.hi.abs());
        numerator = numerator.add(I::point(upper).mul(I::point(upper))?)?;
    }
    Ok(numerator
        .div(normal_squared.mul(I::point(singular_lower))?)?
        .hi
        .min(1.))
}
/// Pair-specific exclusion, not an intrinsic certificate for the convex union.
/// Each attempted matrix charges its executed net restrictions; local paths
/// reserve six, other paths five. Endpoint normal restrictions are reused.
#[derive(Default)]
pub(crate) struct PairExclusion {
    pub excluded: bool,
    pub spans: usize,
    pub coordinate_path_excluded: bool,
    pub coordinate_path_attempted: bool,
    pub coordinate_path_available: bool,
    pub local_path_excluded: bool,
    pub local_path_margin: Option<f64>,
    pub local_path_attempted: bool,
    pub local_path_available: bool,
}
#[cfg(test)]
pub(crate) fn rational_pole_pair_exclusion(
    s: &Surface,
    pair: [[[f64; 2]; 2]; 2],
    differential: &crate::rational_pole_differential::Certificate<'_>,
    normals: &crate::factored_surface_normals::Certificate<'_>,
    sine: f64,
    max_spans: usize,
) -> Result<(bool, usize)> {
    let r = rational_pole_pair_report(s, pair, differential, normals, sine, max_spans)?;
    Ok((r.excluded, r.spans))
}
pub(crate) fn rational_pole_pair_report(
    s: &Surface,
    pair: [[[f64; 2]; 2]; 2],
    differential: &crate::rational_pole_differential::Certificate<'_>,
    normals: &crate::factored_surface_normals::Certificate<'_>,
    sine: f64,
    max_spans: usize,
) -> Result<PairExclusion> {
    rational_pole_pair_report_cached(s, pair, differential, normals, sine, max_spans, None)
}


/// If J ranges in H, the chord is M*delta with M in H by integration along
/// the straight parameter segment. A uniformly nonsingular coordinate minor
/// gives |chord| >= |det(M)|/||M||F * |delta|. At either endpoint n.J=0, so
/// |n.chord| <= |n| * ||J-J(endpoint)||F * |delta|. The squared ratio below
/// bounds cosine to every endpoint normal. This suffices to exclude aligned
/// chords, including arbitrarily close pairs, without dividing by UV distance.
pub fn qualify<'a>(s: &'a Surface, max_sine_squared: f64, max_spans: usize) -> Result<Report<'a>> {
    s.validate()?;
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    qualify_rectangle(s, domain, max_sine_squared, max_spans)
}


/// Fresh original-chart endpoint exclusion over the convex connecting domain.
/// False leaves the product unproven, including continuity or budget refusal.
/// Complete rectangle reports are reusable only on this immutable source and
/// angular policy. Partial calculations and zero-budget queries never admit.
pub struct OriginalChordWorkspace<'a> {
    surface:&'a Surface,
    sine:f64,
    rectangles:std::collections::HashMap<[[u64;2];2],Report<'a>>,
    normals:std::collections::HashMap<[[u64;2];2],[[f64;2];3]>,
    hits:usize,
}
impl<'a> OriginalChordWorkspace<'a> {
    pub fn new(surface:&'a Surface,sine:f64)->Result<Self> {
        surface.validate()?;
        check(sine.is_finite()&&(0. ..1.).contains(&sine),"Valid original chord workspace angle")?;
        Ok(Self{surface,sine,rectangles:std::collections::HashMap::new(),normals:std::collections::HashMap::new(),hits:0})
    }
    pub fn cache_hits(&self)->usize {self.hits}
    /// Whole-rectangle report on the same immutable surface and angle policy.
    /// As in the uncached API, the span budget must be positive and bounded.
    pub fn qualify_rectangle(&mut self,domain:[[f64;2];2],budget:usize)->Result<Report<'a>> {
        check(domain.iter().all(|d|d[0]<d[1]),"Original cached rectangle requires positive widths")?;
        normal_alignment::validate_rectangle(self.surface,domain,budget)?;
        let key=domain.map(|d|d.map(f64::to_bits));
        if let Some(report)=self.rectangles.get(&key) {
            self.hits+=1;let mut report=report.clone();report.spans=0;return Ok(report);
        }
        let report=qualify_rectangle(self.surface,domain,self.sine,budget)?;
        if report.complete&&self.rectangles.len()<100000 {self.rectangles.insert(key,report.clone());}
        Ok(report)
    }
    fn cached_normal(&mut self,domain:[[f64;2];2],budget:usize)->Result<(Option<[[f64;2];3]>,usize)> {
        normal_alignment::validate_rectangle(self.surface,domain,budget)?;
        let key=domain.map(|d|d.map(f64::to_bits));
        if let Some(bounds)=self.normals.get(&key) {self.hits+=1;return Ok((Some(*bounds),0));}
        let result=normal_alignment::normal_bounds(self.surface,domain,budget)?;
        if let Some(bounds)=result.0 {if self.normals.len()<100000 {self.normals.insert(key,bounds);}}
        Ok(result)
    }
    pub fn inspect_pair(&mut self,pair:[[[f64;2];2];2],max_spans:usize)->Result<(bool,usize)> {
        let surface=self.surface;let sine=self.sine;
        // Normal hulls are reusable only on this immutable source. Partial
        // queries are never cached; the caller still enforces positive work.
        let mut normals=std::mem::take(&mut self.normals);
        let mut normal_hits=0;
        let result=inspect_original_endpoint_pair_with(surface,pair,sine,max_spans,
            &mut |domain,budget|self.qualify_rectangle(domain,budget),
            &mut |domain,budget| {
                normal_alignment::validate_rectangle(surface,domain,budget)?;
                let key=domain.map(|d|d.map(f64::to_bits));
                if let Some(bounds)=normals.get(&key) {normal_hits+=1;return Ok((Some(*bounds),0));}
                let result=normal_alignment::normal_bounds(surface,domain,budget)?;
                if let Some(bounds)=result.0 {if normals.len()<100000 {normals.insert(key,bounds);}}
                Ok(result)
            });
        self.normals=normals;self.hits+=normal_hits;
        result
    }
}
pub fn inspect_original_endpoint_pair(
    s:&Surface, pair:[[[f64;2];2];2], sine:f64, max_spans:usize,
) -> Result<(bool,usize)> {
    inspect_original_endpoint_pair_with(s,pair,sine,max_spans,&mut |domain,budget|qualify_rectangle(s,domain,sine,budget),
        &mut |domain,budget|normal_alignment::normal_bounds(s,domain,budget))
}


/// Source-bound exclusion for chords between two original rectangles connected
/// through exactly equal clamped chart corners. This proves direction only.
pub struct CornerChordCertificate<'a> {
    sources: [&'a Surface;2], rectangles: [[[f64;2];2];2], max_sine_squared:f64,
}
/// Exact original Bezier-chart anchor; approximate evaluated points are only
/// proposals until homogeneous composition proves their coordinate identity.
pub struct SurfaceAnchorCertificate<'a> {
    surface:&'a Surface, uv:[f64;2], world:[f64;3],
}
impl<'a> SurfaceAnchorCertificate<'a> {
    pub fn surface(&self)->&'a Surface{self.surface}
    pub fn uv(&self)->[f64;2]{self.uv}
    pub fn world(&self)->[f64;3]{self.world}
}
pub fn qualify_surface_anchor<'a>(s:&'a Surface,uv:[f64;2],world:[f64;3],max_work:u64)
    ->Result<(Option<SurfaceAnchorCertificate<'a>>,u64)> {
    s.validate()?;
    // Exact clamped tensor corners equal their authored control pole, for any
    // degree or interior knot count. Charge the direct identity decision.
    if max_work>0 && !s.periodic_u && !s.periodic_v {
        let counts=[s.control_points.len(),s.control_points[0].len()];
        let degrees=[s.degree_u,s.degree_v];let knots=[&s.knots_u,&s.knots_v];
        let mut indices=[0;2];let mut corner=true;
        for axis in 0..2 {
            let k=knots[axis];let low=k[degrees[axis]];let high=k[counts[axis]];
            if uv[axis]==low && k[..=degrees[axis]].iter().all(|x|*x==low) {
                indices[axis]=0;
            } else if uv[axis]==high && k[counts[axis]..].iter().all(|x|*x==high) {
                indices[axis]=counts[axis]-1;
            } else {corner=false;}
        }
        if corner {
            let pole=&s.control_points[indices[0]][indices[1]];
            return Ok((if pole.as_slice()==world {Some(SurfaceAnchorCertificate{surface:s,uv,world})}else{None},1));
        }
    }
    let constant=|point:Vec<f64>|crate::curve::Curve{degree:1,knots:vec![0.,0.,1.,1.],
        control_points:vec![point.clone(),point],weights:vec![1.,1.],periodic:false};
    let world_curve=constant(world.to_vec());let pcurve=constant(uv.to_vec());
    let Some(report)=crate::curve_surface_agreement::verify_exact(&world_curve,&pcurve,s,false,max_work.min(cad_predicates::MAX_WORK))?
        else{return Ok((None,0));};
    let work=report.work_used;
    let certificate=if report.outcome==cad_predicates::BezierIdentity::Equal {
        Some(SurfaceAnchorCertificate{surface:s,uv,world})
    }else{None};
    Ok((certificate,work))
}
impl<'a> CornerChordCertificate<'a> {
    pub fn sources(&self)->[&'a Surface;2]{self.sources}
    pub fn rectangles(&self)->[[[f64;2];2];2]{self.rectangles}
    pub fn max_sine_squared(&self)->f64{self.max_sine_squared}
}
/// The corner-to-endpoint paths integrate original derivative enclosures.
/// No sampled coincidence, approximate anchors or external normal boxes enter
/// the proof. Work exhaustion and cones permitting cancellation remain unproved.
pub fn qualify_corner_chords<'a>(
    sources:[&'a Surface;2], high_corners:[[bool;2];2], rectangles:[[[f64;2];2];2],
    projection:[f64;3], sine:f64, max_spans:usize,
)->Result<(Option<CornerChordCertificate<'a>>,usize)> {
    qualify_corner_chords_with(sources,high_corners,rectangles,projection,sine,max_spans,None)
}
fn qualify_corner_chords_with<'a>(sources:[&'a Surface;2],high_corners:[[bool;2];2],
    rectangles:[[[f64;2];2];2],projection:[f64;3],sine:f64,max_spans:usize,
    cache:Option<&mut [OriginalChordWorkspace<'a>;2]>,
)->Result<(Option<CornerChordCertificate<'a>>,usize)> {
    check((1..=100000).contains(&max_spans)&&projection.iter().all(|x|x.is_finite())
        &&sine.is_finite()&&(0. ..1.).contains(&sine),"Choose bounded original corner path work and a finite projection")?;
    let mut anchors=[[0.;2];2];let mut poles=[Vec::new(),Vec::new()];
    for side in 0..2 {
        let s=sources[side];s.validate()?;
        check(!s.periodic_u&&!s.periodic_v,"Use nonperiodic clamped corners")?;
        normal_alignment::validate_rectangle(s,rectangles[side],max_spans)?;
        let counts=[s.control_points.len(),s.control_points[0].len()];
        let degrees=[s.degree_u,s.degree_v];let knots=[&s.knots_u,&s.knots_v];
        let mut indices=[0;2];
        for axis in 0..2 {
            let high=high_corners[side][axis];let k=knots[axis];
            let value=if high{k[counts[axis]]}else{k[degrees[axis]]};
            let ends=if high{&k[counts[axis]..]}else{&k[..=degrees[axis]]};
            check(ends.iter().all(|x|*x==value),"Original corner must be exactly clamped")?;
            anchors[side][axis]=value;indices[axis]=if high{counts[axis]-1}else{0};
        }
        poles[side]=s.control_points[indices[0]][indices[1]].clone();
    }
    check(poles[0].len()==3&&poles[0]==poles[1],"Require exactly equal original corner coordinates")?;
    qualify_verified_anchor_paths_with(sources,anchors,rectangles,projection,sine,max_spans,cache)
}
/// Try only exactly shared authored clamped corners. Projection is a proposal;
/// the continuous path/cone gate recomputes all geometric evidence.
pub fn inspect_shared_corner_chords(
    sources:[&Surface;2],rectangles:[[[f64;2];2];2],projection:[f64;3],sine:f64,max_spans:usize,
) -> Result<(bool,usize)> {
    inspect_shared_corner_chords_with(sources,rectangles,projection,sine,max_spans,None)
}
/// Cache complete interval evidence on these two immutable originals only.
/// Directions and exact common-corner ownership are rechecked on every call.
pub struct SharedCornerWorkspace<'a> {
    sources:[&'a Surface;2],sine:f64,cache:[OriginalChordWorkspace<'a>;2],
}
impl<'a> SharedCornerWorkspace<'a> {
    pub fn new(sources:[&'a Surface;2],sine:f64)->Result<Self> {
        Ok(Self{sources,sine,cache:[OriginalChordWorkspace::new(sources[0],sine)?,OriginalChordWorkspace::new(sources[1],sine)?]})
    }
    pub fn cache_hits(&self)->usize {self.cache.iter().map(|c|c.cache_hits()).sum()}
    pub fn inspect(&mut self,rectangles:[[[f64;2];2];2],projection:[f64;3],budget:usize)->Result<(bool,usize)> {
        inspect_shared_corner_chords_with(self.sources,rectangles,projection,self.sine,budget,Some(&mut self.cache))
    }
}


/// Connect only anchors already certified on their own immutable originals.
pub fn qualify_anchored_chords<'a>(
    anchors:[&SurfaceAnchorCertificate<'a>;2],rectangles:[[[f64;2];2];2],
    projection:[f64;3],sine:f64,max_spans:usize,
)->Result<(Option<CornerChordCertificate<'a>>,usize)> {
    check(anchors[0].world==anchors[1].world,"Require identical certified world anchors")?;
    qualify_verified_anchor_paths(anchors.map(|a|a.surface),anchors.map(|a|a.uv),rectangles,projection,sine,max_spans)
}
/// Original polar inverse-coordinate paths use world-axis displacement and a
/// positively weighted V leg. Prepared factors are accepted only for their
/// own source and pole end; preparation work remains the caller's budget.
pub fn qualify_polar_anchored_chords<'a>(
    anchors:[&SurfaceAnchorCertificate<'a>;2],rectangles:[[[f64;2];2];2],
    differential:[Option<&crate::rational_pole_differential::Certificate<'_>>;2],
    normals:[Option<&crate::factored_surface_normals::Certificate<'_>>;2],
    projection:[f64;3],sine:f64,max_spans:usize,
)->Result<(Option<CornerChordCertificate<'a>>,usize)> {
    qualify_polar_anchored_chords_impl(anchors,rectangles,differential,normals,&[projection],sine,max_spans,false)
}
/// Enclose original paths and normals once, then test a bounded direction set.
/// Projection attempts do not repeat geometric span work. One additional
/// derivative-cone direction proposal is checked by the same interval gate.
pub fn qualify_polar_anchored_chords_with_projections<'a>(
    anchors:[&SurfaceAnchorCertificate<'a>;2],rectangles:[[[f64;2];2];2],
    differential:[Option<&crate::rational_pole_differential::Certificate<'_>>;2],
    normals:[Option<&crate::factored_surface_normals::Certificate<'_>>;2],
    projections:&[[f64;3]],sine:f64,max_spans:usize,
)->Result<(Option<CornerChordCertificate<'a>>,usize)> {
    qualify_polar_anchored_chords_impl(anchors,rectangles,differential,normals,projections,sine,max_spans,true)
}

fn qualify_verified_anchor_paths<'a>(sources:[&'a Surface;2],anchors:[[f64;2];2],
    rectangles:[[[f64;2];2];2],projection:[f64;3],sine:f64,max_spans:usize,
)->Result<(Option<CornerChordCertificate<'a>>,usize)> {
    qualify_verified_anchor_paths_with(sources,anchors,rectangles,projection,sine,max_spans,None)
}
fn qualify_verified_anchor_paths_with<'a>(sources:[&'a Surface;2],anchors:[[f64;2];2],
    rectangles:[[[f64;2];2];2],projection:[f64;3],sine:f64,max_spans:usize,
    mut cache:Option<&mut [OriginalChordWorkspace<'a>;2]>,
)->Result<(Option<CornerChordCertificate<'a>>,usize)> {
    check((1..=100000).contains(&max_spans)&&projection.iter().all(|x|x.is_finite())
        &&sine.is_finite()&&(0. ..1.).contains(&sine),"Choose bounded original anchor path work")?;
    for side in 0..2 {normal_alignment::validate_rectangle(sources[side],rectangles[side],max_spans)?;}
    let mut signs=[[0.;2];2];
    for side in 0..2 {for axis in 0..2 {
        signs[side][axis]=if rectangles[side][axis][0]>=anchors[side][axis] {1.}
            else if rectangles[side][axis][1]<=anchors[side][axis] {-1.}
            else{return Ok((None,0));};
    }}
    let mut spans=0;let mut generators=Vec::new();
    for side in 0..2 {
        if spans==max_spans{return Ok((None,spans));}
        let domain=std::array::from_fn(|a|[anchors[side][a].min(rectangles[side][a][0]),anchors[side][a].max(rectangles[side][a][1])]);
        let report=if let Some(c)=cache.as_deref_mut() {c[side].qualify_rectangle(domain,max_spans-spans)?}
            else {qualify_rectangle(sources[side],domain,sine,max_spans-spans)?};
        spans+=report.spans;
        let Some(matrix)=report.mean_matrix else{return Ok((None,spans));};
        for axis in 0..2 {
            let sign=(if side==0{-1.}else{1.})*signs[side][axis];
            let mut g=[[0.;2];3];
            for k in 0..3 {let value=matrix.jacobian[k][axis].mul(I::point(sign))?;g[k]=[value.lo,value.hi];}
            generators.push(g);
        }
    }
    for side in 0..2 {
        if spans==max_spans {break;}
        let (normal,used)=if let Some(c)=cache.as_deref_mut() {c[side].cached_normal(rectangles[side],max_spans-spans)?}
            else {normal_alignment::normal_bounds(sources[side],rectangles[side],max_spans-spans)?};
        spans+=used;
        if let Some(n)=normal {
            if normal_alignment::positive_chord_cone_excluded(&generators,n,projection,sine)? {
                return Ok((Some(CornerChordCertificate{sources,rectangles,max_sine_squared:sine}),spans));
            }
        }
    }
    Ok((None,spans))
}

#[cfg(test)]
#[path="surface_self_chord/tests/original_corner_chord_tests.rs"]
mod original_corner_chord_tests;
/// Excludes nonzero geometric chords using S_v=t² B on an exactly verified
/// collapsed Bezier rail. Along a straight UV segment, averaged S_u and the
/// positive t²-weighted average B lie in the same bounded differential hull.
/// Thus chord=M*(delta_u,delta_v integral(t²)); the ordinary singular-value
/// and endpoint-normal variation argument applies to this weighted vector.
/// If that vector is zero, the geometric chord is zero and is not a wall chord.
pub fn qualify_polar_rectangle<'a>(
    s: &'a Surface,
    domain: [[f64; 2]; 2],
    end: bool,
    max_sine_squared: f64,
    max_spans: usize,
) -> Result<Report<'a>> {
    s.validate()?;
    check(
        max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared),
        "Polar chord exclusion requires squared sine in [0,1)",
    )?;
    normal_alignment::validate_rectangle(s, domain, max_spans)?;
    let mut out = Report {
        certificate: None,
        mean_matrix: None,
        spans: 0,
        complete: false,
        cosine_squared_upper: None,
        reason: "polar-chord-work-limit",
    };
    if max_spans < 3 {
        return Ok(out);
    }
    let Some(j) = crate::polar_surface_differential::bounds(s, end, domain)? else {
        out.reason = "polar-chord-factor-unproven";
        return Ok(out);
    };
    out.spans = 3;
    out.complete = true;
    let best = cosine_upper(j)?;
    out.cosine_squared_upper = Some(best);
    out.reason = "polar-chord-unproven";
    if best < I::point(1.).sub(I::point(max_sine_squared))?.lo {
        out.certificate = Some(Certificate {
            surface: s,
            domain,
            max_sine_squared,
            cosine_squared_upper: best,
        });
        out.reason = "polar-chord-normal-excluded";
    }
    Ok(out)
}
/// Uses the exact source-owned rational decomposition. Preparation work is
/// recorded separately; each query restricts five bounded Bernstein nets.
pub fn qualify_rational_polar_rectangle<'a>(
    s: &'a Surface,
    domain: [[f64; 2]; 2],
    proof: &crate::rational_pole_differential::Certificate<'a>,
    max_sine_squared: f64,
    max_spans: usize,
) -> Result<Report<'a>> {
    check(
        std::ptr::eq(s, proof.surface()),
        "Use the same original rational pole surface",
    )?;
    check(
        max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared),
        "Rational chord requires squared sine in [0,1)",
    )?;
    normal_alignment::validate_rectangle(s, domain, max_spans)?;
    let mut out = Report {
        certificate: None,
        mean_matrix: None,
        spans: 0,
        complete: false,
        cosine_squared_upper: None,
        reason: "rational-polar-chord-work-limit",
    };
    if max_spans < 5 {
        return Ok(out);
    }
    out.spans = 5;
    let Some(j) = proof.bounds(domain)? else {
        out.reason = "rational-polar-longitudinal-unproven";
        return Ok(out);
    };
    let best = balanced_cosine_upper(j)?;
    out.complete = true;
    out.cosine_squared_upper = Some(best);
    out.reason = "rational-polar-chord-unproven";
    if best < I::point(1.).sub(I::point(max_sine_squared))?.lo {
        out.certificate = Some(Certificate {
            surface: s,
            domain,
            max_sine_squared,
            cosine_squared_upper: best,
        });
        out.reason = "rational-polar-chord-normal-excluded";
    }
    Ok(out)
}
#[cfg(test)]
#[path = "surface_self_chord/tests/mod.rs"]
mod tests;

#[path="surface_self_chord/methods_MeanMatrix____coordinate_path.rs"]
mod methods_MeanMatrix____coordinate_path;
#[path="surface_self_chord/algorithm_cosine_upper.rs"]
mod algorithm_cosine_upper;
use algorithm_cosine_upper::cosine_upper;
#[path="surface_self_chord/algorithm_mean_matrix_lower.rs"]
mod algorithm_mean_matrix_lower;
use algorithm_mean_matrix_lower::mean_matrix_lower;
#[path="surface_self_chord/algorithm_rational_pole_pair_report_cached.rs"]
mod algorithm_rational_pole_pair_report_cached;
pub(crate) use algorithm_rational_pole_pair_report_cached::rational_pole_pair_report_cached;
#[path="surface_self_chord/algorithm_balanced_cosine_upper.rs"]
mod algorithm_balanced_cosine_upper;
use algorithm_balanced_cosine_upper::balanced_cosine_upper;
#[path="surface_self_chord/algorithm_qualify_rectangle.rs"]
mod algorithm_qualify_rectangle;
pub use algorithm_qualify_rectangle::qualify_rectangle;
#[path="surface_self_chord/algorithm_inspect_original_endpoint_pair_with.rs"]
mod algorithm_inspect_original_endpoint_pair_with;
use algorithm_inspect_original_endpoint_pair_with::inspect_original_endpoint_pair_with;
#[path="surface_self_chord/algorithm_inspect_shared_corner_chords_with.rs"]
mod algorithm_inspect_shared_corner_chords_with;
use algorithm_inspect_shared_corner_chords_with::inspect_shared_corner_chords_with;
#[path="surface_self_chord/algorithm_qualify_polar_anchored_chords_impl.rs"]
mod algorithm_qualify_polar_anchored_chords_impl;
use algorithm_qualify_polar_anchored_chords_impl::qualify_polar_anchored_chords_impl;
