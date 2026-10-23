import type {BondedSolidModel} from '../services/bondedSolidProtocol'
/** Synthetic two-tetrahedron verification fixture, never a calibrated plastic. */
export const bondedSolidExample:BondedSolidModel={
  nodesMm:[[0,0,0],[1,0,0],[0,1,0],[0,0,-1],[0,0,0],[1,0,0],[0,1,0],[0,0,1]],
  tets:[{nodes:[0,1,2,3],region:'shell',youngMpa:2000,poisson:.3},{nodes:[4,5,6,7],region:'infill',youngMpa:2000,poisson:.3}],
  bonds:[{shell:[0,1,2],infill:[4,5,6],normalStiffnessMpaPerMm:100,shearStiffnessMpaPerMm:50,tensionMpa:10,shearMpa:8,compressionMpa:30,propertySource:'Synthetic; not measured'}],
  restrained:[[true,true,true],[true,true,true],[true,true,true],[true,true,true],[false,false,false],[false,false,false],[false,false,false],[false,false,false]],
  forcesN:[[0,0,0],[0,0,0],[0,0,0],[0,0,0],[0,0,1],[0,0,1],[0,0,1],[0,0,0]],
  safetyFactor:1,
  profile:{material:'custom',grade:'Synthetic example',propertySource:'Synthetic; not measured',nozzleMm:.4,lineWidthMm:.45,layerHeightMm:.2,nozzleTempC:210,bedTempC:60},
  serviceTempC:20,
}
