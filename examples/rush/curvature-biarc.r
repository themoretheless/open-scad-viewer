// @rush/1
path = curve_from_curvature_samples(stations_mm:[3mm,4mm,5mm,6mm,7mm],curvatures_per_mm:[0/1mm,0/1mm,0/1mm,0/1mm,0/1mm],origin:[1mm,2mm],start_angle_rad:0,tolerance_mm:0.001mm)
piece = path.fit_biarc_piece(tolerance_mm:0.001mm,max_arcs:64,piece:0)
show piece.surface_extrude(vector:[0,0,1mm]).tessellate(8,8)
