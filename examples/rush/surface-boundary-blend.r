// @rush/1
a = line_curve(start:[0mm,0mm,0mm],end:[2mm,0mm,0mm]).surface_extrude([0mm,1mm,0mm])
b = line_curve(start:[0mm,2mm,1mm],end:[2mm,2mm,1mm]).surface_extrude([0mm,0mm,1mm])
joined = a.blend_surface_boundaries(target_geometry:b,source_edge:"v_max",target_edge:"v_min",continuity_class:"G2",gains:[1,1],reverse_u:false,reverse_target_u:false,max_patches:16)
show joined.nurbs_patches_tessellate(segments:8)
