// @rush/1
// Six faces of a 2 x 3 x 5 mm shell. The low faces have inward parameter normals.
face0 = bezier_surface(points:[[[0mm,0mm,0mm],[0mm,0mm,5mm]],[[0mm,3mm,0mm],[0mm,3mm,5mm]]])
face1 = bezier_surface(points:[[[2mm,0mm,0mm],[2mm,0mm,5mm]],[[2mm,3mm,0mm],[2mm,3mm,5mm]]])
face2 = bezier_surface(points:[[[0mm,0mm,0mm],[2mm,0mm,0mm]],[[0mm,0mm,5mm],[2mm,0mm,5mm]]])
face3 = bezier_surface(points:[[[0mm,3mm,0mm],[2mm,3mm,0mm]],[[0mm,3mm,5mm],[2mm,3mm,5mm]]])
face4 = bezier_surface(points:[[[0mm,0mm,0mm],[0mm,3mm,0mm]],[[2mm,0mm,0mm],[2mm,3mm,0mm]]])
face5 = bezier_surface(points:[[[0mm,0mm,5mm],[0mm,3mm,5mm]],[[2mm,0mm,5mm],[2mm,3mm,5mm]]])
volume = face0.signed_shell_volume(inputs:[face1,face2,face3,face4,face5],reversed_faces:[0,2,4],tolerance_mm3:0.00001mm*1mm*1mm,max_cells:1024)
show volume.shared_patch_mesh(inputs:[face1,face2,face3,face4,face5],seams:[[0,0,2,2,0],[1,0,2,3,0],[0,1,3,2,0],[1,1,3,3,0],[0,2,4,0,0],[1,2,4,1,0],[2,0,4,2,0],[3,0,4,3,0],[0,3,5,0,0],[1,3,5,1,0],[2,1,5,2,0],[3,1,5,3,0]],segments:4,tolerance_mm:0.5mm,max_cells:1000,max_agreement_cells:10000)
