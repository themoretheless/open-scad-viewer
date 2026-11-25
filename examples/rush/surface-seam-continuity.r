// @rush/1
first = line_curve(start:[-1mm,0mm,0mm],end:[0mm,0mm,0mm])
second = line_curve(start:[0mm,0mm,0mm],end:[1mm,0mm,0mm])
a = first.surface_extrude(vector:[0mm,1mm,0mm])
b = second.surface_extrude(vector:[0mm,1mm,0mm])
position = a.surface_seam_g0(target_geometry:b,source_edge:"u_max",target_edge:"u_min",reverse:false,tolerance_mm:0.05mm,max_cells:256)
tangent = position.surface_seam_g1(target_geometry:b,source_edge:"u_max",target_edge:"u_min",reverse:false,tolerance_mm:0.05mm,normal_tolerance:0.001,max_cells:256)
curvature = tangent.surface_seam_g2(target_geometry:b,source_edge:"u_max",target_edge:"u_min",reverse:false,tolerance_mm:0.05mm,normal_tolerance:0.001,curvature_tolerance_per_mm:0.001/1mm,segments:2,max_cells:256)
show curvature.tessellate(8,8)
