// @rush/1
path = line_curve(start:[0mm,0mm,0mm],end:[0mm,0mm,20mm])
radii = line_curve(start:[2mm,0mm,0mm],end:[4mm,0mm,0mm])
envelope = path.sphere_envelope_surface(target_geometry:radii,normal:[1,0,0],tolerance_mm:0.001mm,max_patches:16,max_domain_cells:512)
show envelope.nurbs_patches_tessellate(segments:16)
