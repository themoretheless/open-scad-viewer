// @rush/1
left_edge = line_curve(start:[0mm,0mm,0mm],end:[1mm,0mm,0mm])
right_edge = line_curve(start:[1mm,0mm,0mm],end:[2mm,0mm,0mm])
left = left_edge.surface_extrude(vector:[0,1mm,0])
right = right_edge.surface_extrude(vector:[0,1mm,0])
// Each seam identifies two patch/side pairs and a reversal flag. Side order is U0, U1, V0, V1.
show left.shared_patch_mesh(inputs:[right],seams:[[0,1,1,0,0]],segments:8,tolerance_mm:0.3mm,max_cells:1000,max_agreement_cells:10000)
