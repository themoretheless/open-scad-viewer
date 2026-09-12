// Installed beside shape_render.rs by prepare.py. Keep gradient interpolation
// independent of the triangulator's choice of diagonals, including cache hits.
fn osv_paint_sampled_gradient(
    painter: &Painter,
    geometry: &osv_geometry::render::tess::VertexBuffers<Pos2, u32>,
    g: &Gradient,
    bb: &MmRect,
    ct: &CanvasTransform,
    op: f32,
    revision: Option<u64>,
    key: Option<CacheKey>,
) {
    use osv_geometry::attribute_mesh::{SampleOptions, SampledMesh, sample_mesh_with_seams};
    use osv_geometry::tessellation::FillMesh;
    // Revision handles document mutations; the fingerprint also distinguishes
    // transient paint parameters and shadow ghosts within one revision.
    let mut fingerprint = 0;
    for byte in serde_json::to_vec(g).unwrap_or_default() {
        fingerprint = hash_pair(fingerprint, u64::from(byte));
    }
    for value in [bb.min.x, bb.min.y, bb.max.x, bb.max.y, op] {
        fingerprint = hash_pair(fingerprint, u64::from(value.to_bits()));
    }
    let sampled = key.as_ref().and_then(|key| {
        let revision = revision?;
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.set_revision(revision);
            match cache.get(key) {
                Some(CacheEntry::GradientSamples { fingerprint: stored, sampled, .. })
                    if stored == fingerprint => Some(sampled),
                _ => None,
            }
        })
    }).unwrap_or_else(|| {
        let base = FillMesh {
            positions: geometry.vertices.iter().map(|p| [f64::from(p.x), f64::from(p.y)]).collect(),
            indices: geometry.indices.clone(),
        };
        let extent = f64::from(bb.width().abs().max(bb.height().abs())).max(1e-9);
        let unit = f64::from(bb.width().abs().min(bb.height().abs())).max(1e-9);
        let feature = match &g.kind {
            crate::shape::GradientKind::Linear { p1, p2 } => {
                if g.spread == crate::shape::GradientSpread::Pad && g.stops.len() <= 2 {
                    f64::INFINITY
                } else {
                    f64::from((p2.x-p1.x).hypot(p2.y-p1.y)) * unit * 0.125
                }
            }
            crate::shape::GradientKind::Radial { radius, .. } => f64::from(radius.abs()) * unit * 0.25,
            crate::shape::GradientKind::Conic { .. } => unit * 0.125,
        };
        // Stop spacing also matters for radial/conic bands. Bound the extra
        // uniform seeding work: arbitrarily narrow bands cannot be guaranteed
        // by four probes, and remain subject to the explicit refinement budget.
        let mut offsets: Vec<_> = g.stops.iter().map(|s| f64::from(s.offset))
            .filter(|v| v.is_finite()).map(|v| v.clamp(0.,1.)).chain([0.,1.]).collect();
        offsets.sort_by(f64::total_cmp);
        let stop_gap = offsets.windows(2).map(|pair| pair[1]-pair[0])
            .filter(|&gap| gap > 0.).fold(1.,f64::min);
        let feature = if matches!(g.kind, crate::shape::GradientKind::Linear { .. }) {
            feature
        } else {
            feature.min((feature*stop_gap).max(extent/64.))
        };
        let options = SampleOptions {
            max_error: 1., max_edge_length: feature.max(extent / 4096.),
            min_edge_length: extent / 4096., ..Default::default()
        };
        // Normalize in binary64 before converting back to the renderer's f32:
        // an off-seam probe at x=1e7 otherwise rounds onto the seam itself.
        let unit_bbox = MmRect::from_min_max(0.,0.,1.,1.);
        let sample = |p: [f64; 2]| {
            let normalized = Vec2::new(((p[0]-f64::from(bb.min.x))/f64::from(bb.width())) as f32,
                                      ((p[1]-f64::from(bb.min.y))/f64::from(bb.height())) as f32);
            gradient_color_at(g, &unit_bbox, normalized, op).to_array().map(f64::from)
        };
        let seams = osv_gradient_seams(g, bb, &base.positions);
        let options = SampleOptions { max_edge_length: if matches!(g.kind, crate::shape::GradientKind::Linear { .. })
            && !seams.is_empty() && seams.len() < 1024 { f64::INFINITY } else { options.max_edge_length }, ..options };
        let mesh = sample_mesh_with_seams(&base, &options, &seams, sample).unwrap_or_else(|error| {
            // Preserve a visible coarse preview if an extreme gradient reaches
            // the explicit work bound. Normal document cases are refined.
            eprintln!("Gradient sampling: {error:?}");
            SampledMesh { values: base.positions.iter().map(|&p| sample(p)).collect(),
                          positions: base.positions, indices: base.indices }
        });
        let mesh = Rc::new(mesh);
        if let Some(key) = key {
            if let Some(base) = cache_get(revision, &key) {
                cache_put(revision, key, CacheEntry::GradientSamples {
                    base: Box::new(base), fingerprint, sampled: mesh.clone(),
                });
            }
        }
        mesh
    });
    let vertices = sampled.positions.iter().zip(&sampled.values).map(|(p, v)| {
        let [r,g,b,a] = v.map(|x| x.round().clamp(0.,255.) as u8);
        egui::epaint::Vertex {
            pos: to_screen(&Vec2::new(p[0] as f32, p[1] as f32), ct.origin, ct.offset, ct.zoom),
            uv: egui::epaint::WHITE_UV,
            color: egui::Color32::from_rgba_premultiplied(r,g,b,a),
        }
    }).collect();
    painter.add(egui::Shape::Mesh(std::sync::Arc::new(egui::epaint::Mesh {
        indices: sampled.indices.clone(), vertices, texture_id: egui::TextureId::default(),
    })));
}

fn osv_gradient_seams(g: &Gradient, bb: &MmRect, positions: &[[f64;2]]) -> Vec<[f64;3]> {
    use crate::shape::{GradientKind, GradientSpread};
    match &g.kind {
        GradientKind::Linear { p1,p2 } => {
            let (dx,dy) = (f64::from(p2.x-p1.x),f64::from(p2.y-p1.y));
            let denominator = dx*dx+dy*dy;
            if denominator < 1e-12 { return Vec::new(); }
            let (a,b) = (dx / f64::from(bb.width()) / denominator,dy / f64::from(bb.height()) / denominator);
            let c = -(f64::from(p1.x)*dx+f64::from(p1.y)*dy)/denominator-a*f64::from(bb.min.x)-b*f64::from(bb.min.y);
            let (min,max) = positions.iter().fold((f64::INFINITY,f64::NEG_INFINITY), |(lo,hi),p| {
                let t=a*p[0]+b*p[1]+c;(lo.min(t),hi.max(t))
            });
            let mut breaks = Vec::new();
            match g.spread {
                GradientSpread::Pad => breaks.extend(g.stops.iter().map(|s| f64::from(s.offset))),
                GradientSpread::Repeat | GradientSpread::Reflect if max-min <= 128. => {
                    for k in (min.floor() as i32-1)..=(max.ceil() as i32+1) {
                        breaks.push(f64::from(k));
                        for stop in &g.stops {
                            let value=f64::from(stop.offset);
                            breaks.push(f64::from(k)+if g.spread == GradientSpread::Reflect && k.rem_euclid(2)==1 {1.-value} else {value});
                        }
                    }
                }
                _ => {}
            }
            breaks.sort_by(f64::total_cmp);breaks.dedup_by(|x,y| (*x-*y).abs()<1e-12);
            breaks.into_iter().filter(|&t| t>=min-1e-9 && t<=max+1e-9).take(1024).map(|t| [a,b,c-t]).collect()
        }
        GradientKind::Conic { center,angle } => {
            let (a,b)=(-f64::from(angle.sin())/f64::from(bb.width()),f64::from(angle.cos())/f64::from(bb.height()));
            vec![[a,b,-a*f64::from(bb.min.x+center.x*bb.width())-b*f64::from(bb.min.y+center.y*bb.height())]]
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod osv_gradient_sampling_tests {
    use super::*;
    #[test]
    fn repeated_gradient_at_large_document_coordinates_keeps_sharp_seams() {
        RENDER_CACHE.with(|cache| *cache.borrow_mut()=RenderCache::new());
        let ctx=egui::Context::default();ctx.begin_pass(egui::RawInput::default());
        let painter=ctx.layer_painter(egui::LayerId::background());
        let origin=1e7;
        let shape=Shape::new("translated repeat".into(), ShapeData::Rectangle {
            top_left:Vec2::new(origin,origin),width:128.,height:128.,
        });
        let geometry=osv_geometry::render::tess::VertexBuffers {
            vertices:vec![Pos2::new(origin,origin),Pos2::new(origin+128.,origin),
                          Pos2::new(origin+128.,origin+128.),Pos2::new(origin,origin+128.)],
            indices:vec![0,1,2,0,2,3],
        };
        let mut gradient=Gradient::default_linear();
        gradient.kind=crate::shape::GradientKind::Linear{p1:Vec2::new(0.,0.),p2:Vec2::new(0.25,0.)};
        gradient.spread=crate::shape::GradientSpread::Repeat;
        let key=CacheKey{id:shape.id.clone(),kind:KIND_GRADIENT_FILL,discrim:0,zoom_bucket:0};
        cache_put(Some(800),key.clone(),CacheEntry::GradientFill(Rc::new(GradientFillTessellation {
            geometry:geometry.clone(),bbox:shape.bounding_box(),
        })));
        let ct=CanvasTransform::new(Pos2::new(-origin,-origin),egui::Vec2::ZERO,1.);
        osv_paint_sampled_gradient(&painter,&geometry,&gradient,&shape.bounding_box(),&ct,1.,Some(800),Some(key));
        RENDER_CACHE.with(|cache| {
            let cache=cache.borrow();
            let mesh=cache.entries.values().find_map(|entry| match entry {
                CacheEntry::GradientSamples { sampled, .. }=>Some(sampled),_=>None,
            }).unwrap();
            assert!(mesh.indices.len()/3 <= 16, "repeat fell back to coarse sampling or exhausted refinement");
            let seam_values:Vec<_>=mesh.positions.iter().zip(&mesh.values).filter(|(p,_)|
                (p[0]-f64::from(origin+32.)).abs()<1e-6).map(|(_,v)| v[0]).collect();
            assert!(seam_values.iter().any(|&v| v<101.) && seam_values.iter().any(|&v| v>254.));
        });
        let _=ctx.end_pass();
    }
    #[test]
    fn radial_interior_and_recolor_cache_survive_diagonal_changes() {
        let ctx = egui::Context::default();
        ctx.begin_pass(egui::RawInput::default());
        let painter = ctx.layer_painter(egui::LayerId::background());
        let shape = Shape::new("sampling test".into(), ShapeData::Rectangle {
            top_left: Vec2::new(0.,0.), width: 10., height: 10.,
        });
        let bb = shape.bounding_box();
        let ct = CanvasTransform::new(Pos2::ZERO, egui::Vec2::ZERO, 1.);
        let key = CacheKey { id: shape.id.clone(), kind: KIND_GRADIENT_FILL, discrim: 0, zoom_bucket: 0 };
        for indices in [vec![0,1,2,0,2,3], vec![0,1,3,1,2,3]] {
            let geometry = osv_geometry::render::tess::VertexBuffers {
                vertices: vec![Pos2::new(0.,0.),Pos2::new(10.,0.),Pos2::new(10.,10.),Pos2::new(0.,10.)], indices,
            };
            osv_paint_sampled_gradient(&painter, &geometry, &Gradient::default_radial(), &bb, &ct, 1., None, None);
        }
        let output = ctx.end_pass();
        let meshes = ctx.tessellate(output.shapes, output.pixels_per_point);
        let vertices: Vec<_> = meshes.iter().flat_map(|primitive| match &primitive.primitive {
            egui::epaint::Primitive::Mesh(mesh) => mesh.vertices.as_slice(), _ => &[],
        }).collect();
        assert!(vertices.iter().any(|v| v.pos.distance(Pos2::new(5.,5.)) < 1e-5 && v.color.r() == 255));
        // A style-only transient change with the same document revision must
        // not reuse the first gradient's colored vertices.
        ctx.begin_pass(egui::RawInput::default());
        let geometry = osv_geometry::render::tess::VertexBuffers {
            vertices: vec![Pos2::new(0.,0.),Pos2::new(10.,0.),Pos2::new(0.,10.)], indices: vec![0,1,2],
        };
        let mut gradient = Gradient::default_linear();
        cache_put(Some(500),key.clone(),CacheEntry::GradientFill(Rc::new(GradientFillTessellation {
            geometry:geometry.clone(),bbox:shape.bounding_box(),
        })));
        osv_paint_sampled_gradient(&painter, &geometry, &gradient, &bb, &ct, 1., Some(500), Some(key.clone()));
        for stop in &mut gradient.stops { stop.color = crate::shape::Color::new(255,0,0,255); }
        osv_paint_sampled_gradient(&painter, &geometry, &gradient, &bb, &ct, 1., Some(500), Some(key));
        let output = ctx.end_pass();
        let meshes = ctx.tessellate(output.shapes, output.pixels_per_point);
        assert!(meshes.iter().any(|p| match &p.primitive {
            egui::epaint::Primitive::Mesh(m) => m.vertices.iter().any(|v| v.color == egui::Color32::RED), _ => false,
        }));
    }

    #[test]
    fn three_hundred_gradient_shapes_keep_geometry_and_sample_cache_hits() {
        RENDER_CACHE.with(|cache| *cache.borrow_mut()=RenderCache::new());
        let ctx=egui::Context::default();
        let shapes:Vec<_>=(0..300).map(|i| Shape::new(format!("gradient {i}"),ShapeData::Rectangle {
            top_left:Vec2::new(0.,0.),width:10.,height:10.,
        })).collect();
        let keys:Vec<_>=shapes.iter().map(|shape| CacheKey {
            id:shape.id.clone(),kind:KIND_GRADIENT_FILL,discrim:0,zoom_bucket:0,
        }).collect();
        let ct=CanvasTransform::new(Pos2::ZERO,egui::Vec2::ZERO,1.);
        let gradient=Gradient::default_linear();
        let mut first_samples=Vec::new();
        let mut first_geometry=Vec::new();
        for frame in 0..3 {
            ctx.begin_pass(egui::RawInput::default());
            let painter=ctx.layer_painter(egui::LayerId::background());
            for (index,(shape,key)) in shapes.iter().zip(&keys).enumerate() {
                draw_gradient_mesh(&painter,shape,&gradient,&ct,1.,Some(901),0);
                let sample=RENDER_CACHE.with(|cache| match cache.borrow().entries.get(key) {
                    Some(CacheEntry::GradientSamples { sampled, .. })=>sampled.clone(),
                    _=>panic!("missing sampled mesh for shape {index}"),
                });
                let geometry=match cache_get(Some(901),key) {
                    Some(CacheEntry::GradientFill(geometry))=>geometry,
                    _=>panic!("geometry wrapper changed the cache_get contract"),
                };
                if frame==0 {
                    first_samples.push(sample);first_geometry.push(geometry);
                } else {
                    assert!(Rc::ptr_eq(&sample,&first_samples[index]),"sample cache missed for shape {index}");
                    assert!(Rc::ptr_eq(&geometry,&first_geometry[index]),"geometry cache missed for shape {index}");
                }
            }
            RENDER_CACHE.with(|cache| assert_eq!(cache.borrow().entries.len(),300));
            let _=ctx.end_pass();
        }
    }
}
