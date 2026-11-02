//! Frozen original-source recipe plus exact geometry; restore always re-proves.
use super::*;
use crate::Model;
use nurbs_core::{curve::Curve,surface::Surface};
use value_codec::{Value,json};
pub const FORMAT:&str="native-convex-graph-volume/1";
const PROFILE:&str="convex-core-six/1";
pub struct RootCut {pub root:usize,pub parameter:Q}
pub struct VolumeRecipe {
 pub source:Model,pub supports:[usize;2],pub widths:Vec<Curve>,pub cuts:Vec<RootCut>,
 pub tolerance_mm:f64,pub bounds:VolumeBounds,
}
pub struct FrozenGraphVolume {recipe:VolumeRecipe,body:NativeVolume}
fn fail(s:impl Into<String>)->Error{Error::new("BREP_GRAPH_TRANSPORT",s)}
fn fields(v:&Value,keys:&[&str])->Result<()>{
 let o=v.as_object().ok_or_else(||fail("Expected graph transport object"))?;
 require(o.len()==keys.len()&&o.keys().all(|k|keys.contains(&k.as_str())),"Unexpected or missing graph transport field")
}
fn field<T:for<'a> value_codec::Deserialize<'a>>(v:&Value,k:&str)->Result<T>{
 value_codec::from_value(v.get(k).ok_or_else(||fail("Missing graph transport field"))?.clone()).map_err(|e|fail(e.to_string()))
}
fn q(x:&Q)->Value{json!([x.numer().to_string(),x.denom().to_string()])}
fn rational(v:&Value)->Result<Q>{
 let s:[String;2]=value_codec::from_value(v.clone()).map_err(|e|fail(e.to_string()))?;
 require(s.iter().all(|s|!s.is_empty()&&s.len()<=2500),"Graph rational text exceeds limit")?;
 let n=s[0].parse::<num_bigint::BigInt>().map_err(|_|fail("Invalid graph numerator"))?;
 let d=s[1].parse::<num_bigint::BigInt>().map_err(|_|fail("Invalid graph denominator"))?;
 require(d.is_positive()&&n.bits()<=8192&&d.bits()<=8192,"Invalid graph rational")?;
 let r=Q::new(n,d);require(q(&r)==*v,"Noncanonical graph rational")?;Ok(r)
}
fn points<const N:usize>(a:&[[Q;N]])->Value{json!(a.iter().map(|p|p.iter().map(q).collect::<Vec<_>>()).collect::<Vec<_>>())}
fn surface(s:&Surface)->Value{
 // Original binary64 inputs have exact dyadic values; retain their bit patterns.
 json!({"degrees":[s.degree_u,s.degree_v],"knotsU":s.knots_u.iter().map(|x|format!("{:016x}",x.to_bits())).collect::<Vec<_>>(),"knotsV":s.knots_v.iter().map(|x|format!("{:016x}",x.to_bits())).collect::<Vec<_>>(),"points":s.control_points.iter().map(|r|r.iter().map(|p|p.iter().map(|x|format!("{:016x}",x.to_bits())).collect::<Vec<_>>()).collect::<Vec<_>>()).collect::<Vec<_>>(),"weights":s.weights.iter().map(|r|r.iter().map(|x|format!("{:016x}",x.to_bits())).collect::<Vec<_>>()).collect::<Vec<_>>(),"periodic":[s.periodic_u,s.periodic_v]})
}
fn geometry(body:&NativeVolume)->Value{
 let m=body.topology();
 let faces=m.faces.iter().map(|f|{
  let support=match &f.surface{
   FaceGeometry::Transition(p)=>json!({"kind":"transition","coefficients":p.coefficients().iter().map(|r|points(r)).collect::<Vec<_>>()}),
   FaceGeometry::Retained(r)=>json!({"kind":"retained","source":surface(r.source()),"side":r.side(),"uses":r.uses().iter().map(|(u,reversed)|json!({"uv":points(u.uv_points()),"reversed":reversed})).collect::<Vec<_>>()}),
   FaceGeometry::Imported(r)=>json!({"kind":"original","source":surface(r.source()),"sourceId":r.source_id().to_string(),"uses":r.uses.iter().map(|u|json!({"uv":points(&u.uv),"edge":u.edge,"reversed":u.reversed,"root":u.root.id,"range":u.range.iter().map(q).collect::<Vec<_>>()})).collect::<Vec<_>>()})
  };json!({"surface":support,"outer":f.outer,"holes":f.holes})
 }).collect::<Vec<_>>();
 json!({"vertices":m.vertices.iter().map(|v|points(std::slice::from_ref(&v.point))).collect::<Vec<_>>(),
 "edges":m.edges.iter().map(|e|json!({"vertices":e.vertices,"degenerate":e.degenerate,"points":points(e.curve.points()),"root":e.curve.root_id(),"rootPoints":points(e.curve.root_points()),"range":e.curve.root_parameters().iter().map(q).collect::<Vec<_>>()})).collect::<Vec<_>>(),
 "loops":m.loops.iter().map(|l|l.coedges.iter().map(|c|{
  let uv=match &c.pcurve{FaceUseGeometry::Transition(p)=>json!({"kind":"transition","points":points(p.points())}),FaceUseGeometry::Retained(i)=>json!({"kind":"retained","index":i}),FaceUseGeometry::Imported(i)=>json!({"kind":"original","index":i})};
  json!({"edge":c.edge,"reversed":c.reversed,"uv":uv,"trim":c.trim().map(|t|json!({"curve":t.curve_parameter.map(|x|format!("{:016x}",x.to_bits())),"pcurve":t.pcurve_parameter.map(|x|format!("{:016x}",x.to_bits())),"periodicLift":t.periodic_lift}))})
 }).collect::<Vec<_>>()).collect::<Vec<_>>(),"faces":faces,
 "shells":m.shells.iter().map(|s|json!({"closed":s.closed,"faces":s.faces.iter().map(|f|json!([f.face,f.reversed])).collect::<Vec<_>>()})).collect::<Vec<_>>(),
 "bodies":m.bodies.iter().map(|b|json!({"outer":b.outer_shell,"inner":b.inner_shells})).collect::<Vec<_>>(),
 "identityLinks":{"edges":body.original_shell().source_edge_links().iter().map(|l|json!({"edge":l.roof_edge,"sourceIndex":l.source_edge,"sourceId":l.source_id.to_string(),"sourceRange":l.source_range.iter().map(q).collect::<Vec<_>>(),"reversed":l.reversed_from_source})).collect::<Vec<_>>(),"vertices":body.original_shell().source_vertex_links().iter().map(|(id,index)|json!([id.to_string(),index])).collect::<Vec<_>>(),"faces":body.original_shell().source_face_links().iter().map(|(id,index)|json!([id.to_string(),index])).collect::<Vec<_>>()},
 "toleranceBits":format!("{:016x}",m.tolerance_mm.to_bits()),"signedVolume":q(body.volume()),"radiusLawCertified":false,"applicationAdmitted":false})
}
fn bounded(v:&Value,max_bytes:usize)->Result<()>{
 require(max_bytes>0&&max_bytes<=4<<20,"Invalid graph transport byte allowance")?;
 let data=value_codec::encode_binary(v).map_err(|e|fail(e.to_string()))?;
 require(data.len()<=max_bytes,"Graph transport exceeds byte allowance")
}
impl FrozenGraphVolume {
 pub fn body(&self)->&NativeVolume{&self.body}
 pub fn construct(recipe:VolumeRecipe,max_pairs:usize,b:&mut Budget)->Result<Self>{
  require(recipe.tolerance_mm.is_finite()&&recipe.tolerance_mm>0.&&recipe.tolerance_mm<=1e-7&&!recipe.widths.is_empty()&&recipe.widths.len()<=128&&recipe.cuts.len()<=4096&&recipe.supports[0]!=recipe.supports[1]&&recipe.supports.iter().all(|&i|i<recipe.source.faces.len()),"Invalid frozen graph recipe dimensions")?;
  let supports=recipe.supports.map(|i|&recipe.source.faces[i].surface);
  let patches=recipe.widths.iter().map(|w|nurbs_core::original_graph_transition::build_convex_g2(supports,w,b)).collect::<Result<Vec<_>>>()?;
  // Fixed recipe version must not silently follow future profile edits.
  let expected=[Q::one(),Q::new(2.into(),3.into()),Q::new(1.into(),3.into()),Q::new(7.into(),15.into()),Q::new(1.into(),3.into()),Q::new(2.into(),3.into()),Q::one()];
  require(patches.iter().all(|p|p.profile_coefficients()==expected),"Frozen graph profile version mismatch")?;
  let mut strip=assemble(patches,recipe.tolerance_mm,b)?;
  for cut in &recipe.cuts{
   require(cut.parameter>Q::zero()&&cut.parameter<Q::one()&&cut.parameter.numer().bits()<=8192&&cut.parameter.denom().bits()<=8192,"Invalid root split parameter")?;
   let edge=strip.topology().edges.iter().position(|e|e.curve.root_id()==cut.root&&e.curve.root_parameters()[0]<cut.parameter&&cut.parameter<e.curve.root_parameters()[1]).ok_or_else(||fail("Root cut is duplicate or outside the current root interval"))?;
   let range=strip.topology().edges[edge].curve.root_parameters();
   let local=(&cut.parameter-&range[0])/(&range[1]-&range[0]);strip.split_edge(edge,local,b)?;
  }
  for split in strip.source_end_splits()?{strip.apply_source_end_split(&split,b)?;}
  let shell=strip.sew_retained(b)?.close_original_shell(&recipe.source,recipe.supports,b)?;
  let body=shell.admit_volume(VolumeBounds::new(recipe.bounds.minimum().clone(),recipe.bounds.maximum().clone())?,max_pairs,b)?;
  Ok(Self{recipe,body})
 }
 pub fn export(&self,max_bytes:usize)->Result<Value>{
  let r=&self.recipe;
  let v=json!({"format":FORMAT,"recipe":{"profile":PROFILE,"source":r.source,"supports":r.supports,"widths":r.widths,"cuts":r.cuts.iter().map(|c|json!({"root":c.root,"at":q(&c.parameter)})).collect::<Vec<_>>(),"toleranceMm":r.tolerance_mm,"volumeBounds":[q(r.bounds.minimum()),q(r.bounds.maximum())]},"geometry":geometry(&self.body)});
  bounded(&v,max_bytes)?;Ok(v)
 }
 pub fn restore(v:&Value,max_bytes:usize,max_pairs:usize,b:&mut Budget)->Result<Self>{
  bounded(v,max_bytes)?;fields(v,&["format","recipe","geometry"])?;
  require(v["format"].as_str()==Some(FORMAT),"Unknown frozen graph format")?;
  let r=&v["recipe"];fields(r,&["profile","source","supports","widths","cuts","toleranceMm","volumeBounds"])?;
  require(r["profile"].as_str()==Some(PROFILE),"Unknown frozen graph profile")?;
  let cuts=r["cuts"].as_array().ok_or_else(||fail("Invalid root cuts"))?;
  require(cuts.len()<=4096,"Too many root cuts")?;
  let mut decoded=Vec::new();for c in cuts{fields(c,&["root","at"])?;decoded.push(RootCut{root:field(c,"root")?,parameter:rational(&c["at"])?});}
  let bounds=r["volumeBounds"].as_array().ok_or_else(||fail("Invalid volume bounds"))?;require(bounds.len()==2,"Invalid volume bounds")?;
  let result=Self::construct(VolumeRecipe{source:field(r,"source")?,supports:field(r,"supports")?,widths:field(r,"widths")?,cuts:decoded,tolerance_mm:field(r,"toleranceMm")?,bounds:VolumeBounds::new(rational(&bounds[0])?,rational(&bounds[1])?)?},max_pairs,b)?;
  require(geometry(result.body())==v["geometry"],"Frozen graph geometry differs from its freshly proven source recipe")?;Ok(result)
 }
}
