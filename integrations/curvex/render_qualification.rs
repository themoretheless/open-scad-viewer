//! Headless capture of the real Curvex paint pipeline, used unchanged in both builds.
use curvex::shape::{Shape, ShapeData, Contour, PathSegment, Vec2, FillRule, FillStyle, Gradient, Color};
use curvex::ui::canvas::CanvasTransform;
use curvex::ui::shape_render::{draw_shape, draw_shape_cached};
use egui::{Pos2, Rect, RawInput};
use serde_json::json;
fn ring(points:&[[f32;2]])->Contour {
    Contour{start:Vec2::new(points[0][0],points[0][1]),segments:points[1..].iter().map(|p|PathSegment::Line{to:Vec2::new(p[0],p[1])}).collect()}
}
fn region(name:&str,contours:Vec<Contour>,rule:FillRule)->Shape {
    let mut s=Shape::new(name.into(),ShapeData::Compound{contours});
    s.fill_rule=rule;s.fill=Some(FillStyle::Solid{color:Color::new(30,135,210,255)});s.stroke.width=0.;s
}
fn main() {
    let outer=ring(&[[10.,10.],[90.,10.],[90.,90.],[10.,90.]]);
    let inner=ring(&[[30.,30.],[70.,30.],[70.,70.],[30.,70.]]);
    let island=ring(&[[43.,43.],[57.,43.],[57.,57.],[43.,57.]]);
    let donut=region("evenodd nested islands",vec![outer.clone(),inner.clone(),island],FillRule::EvenOdd);
    let nonzero=region("nonzero same winding",vec![outer.clone(),inner.clone()],FillRule::Nonzero);
    let bowtie=region("self crossing bowtie",vec![ring(&[[10.,10.],[90.,90.],[10.,90.],[90.,10.]])],FillRule::EvenOdd);
    let mut gradient=region("linear gradient donut",vec![outer.clone(),inner.clone()],FillRule::EvenOdd);
    gradient.fill=Some(FillStyle::Gradient{gradient:Gradient::default_linear()});gradient.opacity=0.65;
    let mut radial=region("radial concave fill",vec![ring(&[[10.,10.],[90.,10.],[90.,45.],[45.,45.],[45.,90.],[10.,90.]])],FillRule::Nonzero);
    radial.fill=Some(FillStyle::Gradient{gradient:Gradient::default_radial()});
    let mut conic=radial.clone();conic.name="conic concave fill".into();conic.fill=Some(FillStyle::Gradient{gradient:Gradient::default_conic()});
    let mut repeated=radial.clone();repeated.name="repeated linear fill".into();
    let mut repeated_gradient=Gradient::default_linear();repeated_gradient.spread=curvex::shape::GradientSpread::Repeat;
    repeated_gradient.kind=curvex::shape::GradientKind::Linear{p1:Vec2::new(0.,0.),p2:Vec2::new(0.25,0.)};
    repeated.fill=Some(FillStyle::Gradient{gradient:repeated_gradient});
    let mut stroke=Shape::new("gradient cubic stroke".into(),ShapeData::BezierPath{start:Vec2::new(10.,75.),segments:vec![PathSegment::Cubic{c1:Vec2::new(15.,-20.),c2:Vec2::new(85.,120.),to:Vec2::new(90.,25.)}],closed:false});
    stroke.fill=None;stroke.stroke.width=7.;stroke.stroke.gradient=Some(Gradient::default_linear());
    let mut ribbon=region("closed gradient stroke",vec![outer],FillRule::Nonzero);ribbon.fill=None;ribbon.stroke.width=7.;ribbon.stroke.gradient=Some(Gradient::default_linear());
    let mut rectangle=Shape::new("rectangle".into(),ShapeData::Rectangle{top_left:Vec2::new(10.,10.),width:55.,height:80.});
    rectangle.fill=Some(FillStyle::Solid{color:Color::new(25,150,180,255)});
    let mut rounded=Shape::new("rounded protrusion".into(),ShapeData::Rectangle{top_left:Vec2::new(45.,30.),width:48.,height:40.});rounded.corner_radii=vec![18.;4];
    let merged=curvex::boolean_ops::boolean_union_for_bench(&[&rectangle,&rounded]);
    let curves=region("boolean curved protrusion",merged.into_iter().map(|(start,segments)|Contour{start,segments}).collect(),FillRule::Nonzero);
    let mut shadowed=curves.clone();shadowed.name="curved outer shadow ghost".into();
    shadowed.shadows.push(curvex::shape::DropShadow{dx:5.,dy:5.,color:Color::new(0,0,0,128),blur:0.});
    let mut inset=donut.clone();inset.name="compound inner shadow ghost".into();
    inset.inner_shadows.push(curvex::shape::InnerShadow{dx:7.,dy:3.,color:Color::new(0,0,0,128),blur:0.});
    let cases=[donut,nonzero,bowtie,gradient,radial,conic,repeated,stroke,ribbon,curves,shadowed,inset];
    let mut captures=Vec::new();
    for shape in cases {
        for zoom in [0.5,2.,8.] {
            let ctx=egui::Context::default();
            let size=110.*zoom;
            let mut frames=Vec::new();
            let mut doc=curvex::document::Document::new("qualification".into(),110.,110.);
            doc.add_shape(shape.clone());
            for cached in [false,true,true] {
                ctx.begin_pass(RawInput{screen_rect:Some(Rect::from_min_max(Pos2::ZERO,Pos2::new(size,size))),..Default::default()});
                let frame_started=std::time::Instant::now();
                    let painter=ctx.layer_painter(egui::LayerId::background());
                    let ct=CanvasTransform::new(Pos2::ZERO,egui::Vec2::ZERO,zoom);
                    if cached{draw_shape_cached(&painter,&shape,&ct,Some(&doc));}else{draw_shape(&painter,&shape,&ct);}
                let paint_micros=frame_started.elapsed().as_secs_f64()*1e6;
                let output=ctx.end_pass();
                let primitives=ctx.tessellate(output.shapes,output.pixels_per_point);
                let frame_micros=frame_started.elapsed().as_secs_f64()*1e6;
                let meshes:Vec<_>=primitives.into_iter().filter_map(|p|match p.primitive {
                    egui::epaint::Primitive::Mesh(m)=>Some(json!({"clip":[p.clip_rect.min.x,p.clip_rect.min.y,p.clip_rect.max.x,p.clip_rect.max.y],"indices":m.indices,"vertices":m.vertices.iter().map(|v|json!({"position":[v.pos.x/zoom,v.pos.y/zoom],"color":v.color.to_array()})).collect::<Vec<_>>()})),_=>None
                }).collect();
                frames.push(json!({"cached":cached,"meshes":meshes,"paint_micros":paint_micros,"frame_micros":frame_micros}));
            }
            let g=match &shape.fill{Some(FillStyle::Gradient{gradient})=>Some(gradient),_=>shape.stroke.gradient.as_ref()};
            let bbox=shape.bounding_box();
            captures.push(json!({"case":shape.name,"zoom":zoom,"frames":frames,"gradient":g,"opacity":shape.opacity,
                "bbox":[bbox.min.x,bbox.min.y,bbox.max.x,bbox.max.y]}));
        }
    }
    println!("{}",serde_json::to_string(&captures).unwrap());
}
