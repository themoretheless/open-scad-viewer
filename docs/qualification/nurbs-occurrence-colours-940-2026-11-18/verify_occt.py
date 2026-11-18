import sys
from OCP.STEPCAFControl import STEPCAFControl_Reader
from OCP.TDocStd import TDocStd_Document
from OCP.TCollection import TCollection_ExtendedString
from OCP.XCAFDoc import XCAFDoc_DocumentTool,XCAFDoc_ColorSurf
from OCP.Quantity import Quantity_Color
from OCP.collections import Sequence_TDF_Label as TDF_LabelSequence
from OCP.IFSelect import IFSelect_RetDone
from OCP.TDF import TDF_Label
r=STEPCAFControl_Reader()
assert r.ReadFile(sys.argv[1])==IFSelect_RetDone
d=TDocStd_Document(TCollection_ExtendedString('MDTV-XCAF'));assert r.Transfer(d)
t=XCAFDoc_DocumentTool.ShapeTool_s(d.Main()); c=XCAFDoc_DocumentTool.ColorTool_s(d.Main())
roots=TDF_LabelSequence();t.GetFreeShapes(roots);assert roots.Length()==1
parts=TDF_LabelSequence();t.GetComponents_s(roots.Value(1),parts,False)
rows=[]
for i in range(1,parts.Length()+1):
    lab=parts.Value(i); shape=t.GetShape_s(lab); rgb=Quantity_Color()
    ok=c.GetInstanceColor(shape,XCAFDoc_ColorSurf,rgb) or c.GetColor_s(lab,XCAFDoc_ColorSurf,rgb)
    rows.append((shape.Location().Transformation().TranslationPart().X(),ok,(rgb.Red(),rgb.Green(),rgb.Blue())))
print(rows)
assert sorted(rows)==[(0.,True,(0.,0.,1.)),(5.,True,(1.,0.,0.))]
