// @rush/1
base = line_curve(start:[0mm,0mm,0mm],end:[4mm,0mm,0mm])
panel = base.surface_extrude([0mm,3mm,0mm])
edge = line_curve(start:[0mm,1.5mm,0mm],end:[4mm,1.5mm,0mm])
uv = edge.pullback_curve_to_uv(target_geometry:panel,tolerance_mm:0.000001mm,max_segments:64,max_iterations:32,agreement_cells:100000)
lifted = uv.lift_uv_edge(target_geometry:panel,piece:0,max_work:1000000)
show lifted.surface_extrude([0mm,0mm,1mm]).tessellate(4,2)
