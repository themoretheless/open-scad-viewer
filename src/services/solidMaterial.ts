import type {DirectDocument,DirectMaterial} from './directModeling'

export interface EngineeringMaterialSpec {
  id: string
  name: string
  category: 'polymer' | 'metal' | 'composite'
  densityGPerCm3: number
  youngsModulusGpa: number
  yieldStrengthMpa: number
  thermalConductivityWPerMK: number
  defaultHexColor: string
}

export const SOLID_ENGINEERING_MATERIALS: Readonly<Record<string, EngineeringMaterialSpec>> = {
  pla: {
    id: 'pla',
    name: 'PLA',
    category: 'polymer',
    densityGPerCm3: 1.24,
    youngsModulusGpa: 3.5,
    yieldStrengthMpa: 50,
    thermalConductivityWPerMK: 0.13,
    defaultHexColor: '#60a5fa',
  },
  petg: {
    id: 'petg',
    name: 'PETG',
    category: 'polymer',
    densityGPerCm3: 1.27,
    youngsModulusGpa: 2.1,
    yieldStrengthMpa: 48,
    thermalConductivityWPerMK: 0.2,
    defaultHexColor: '#38bdf8',
  },
  abs: {
    id: 'abs',
    name: 'ABS',
    category: 'polymer',
    densityGPerCm3: 1.04,
    youngsModulusGpa: 2.2,
    yieldStrengthMpa: 42,
    thermalConductivityWPerMK: 0.17,
    defaultHexColor: '#f59e0b',
  },
  nylon_pa12: {
    id: 'nylon_pa12',
    name: 'Nylon PA12',
    category: 'polymer',
    densityGPerCm3: 1.01,
    youngsModulusGpa: 1.7,
    yieldStrengthMpa: 45,
    thermalConductivityWPerMK: 0.25,
    defaultHexColor: '#e2e8f0',
  },
  aluminum_6061: {
    id: 'aluminum_6061',
    name: 'Aluminum 6061',
    category: 'metal',
    densityGPerCm3: 2.7,
    youngsModulusGpa: 68.9,
    yieldStrengthMpa: 276,
    thermalConductivityWPerMK: 167,
    defaultHexColor: '#cbd5e1',
  },
  steel_304: {
    id: 'steel_304',
    name: 'Steel 304',
    category: 'metal',
    densityGPerCm3: 8.0,
    youngsModulusGpa: 193,
    yieldStrengthMpa: 215,
    thermalConductivityWPerMK: 16.2,
    defaultHexColor: '#94a3b8',
  },
  brass_c360: {
    id: 'brass_c360',
    name: 'Brass C360',
    category: 'metal',
    densityGPerCm3: 8.5,
    youngsModulusGpa: 97,
    yieldStrengthMpa: 310,
    thermalConductivityWPerMK: 115,
    defaultHexColor: '#eab308',
  },
  titanium_ti6al4v: {
    id: 'titanium_ti6al4v',
    name: 'Titanium Ti-6Al-4V',
    category: 'metal',
    densityGPerCm3: 4.43,
    youngsModulusGpa: 113.8,
    yieldStrengthMpa: 880,
    thermalConductivityWPerMK: 6.7,
    defaultHexColor: '#64748b',
  },
  carbon_fiber_composite: {
    id: 'carbon_fiber_composite',
    name: 'Carbon Fiber Composite',
    category: 'composite',
    densityGPerCm3: 1.55,
    youngsModulusGpa: 70,
    yieldStrengthMpa: 600,
    thermalConductivityWPerMK: 5.0,
    defaultHexColor: '#334155',
  },
}

/**
 * Estimates the mass in grams for a given solid volume in cubic millimeters (`mm^3`)
 * and an engineering material identifier or name.
 * Note: `1 cm^3 = 1000 mm^3`, so `mass (g) = (volumeMm3 / 1000) * densityGPerCm3`.
 */
export function estimateSolidMassGrams(volumeMm3: number, materialId: string): number {
  if (!Number.isFinite(volumeMm3) || volumeMm3 < 0) {
    throw new RangeError('Solid volume must be a non-negative finite number in mm^3.')
  }
  const normalized = materialId.trim().toLowerCase().replace(/[\s-]+/g, '_')
  const direct = SOLID_ENGINEERING_MATERIALS[materialId] ?? SOLID_ENGINEERING_MATERIALS[normalized]
  const spec = direct ?? Object.values(SOLID_ENGINEERING_MATERIALS).find(
    m => m.name.toLowerCase() === materialId.trim().toLowerCase(),
  )
  if (!spec) {
    throw new Error(`Unknown engineering material: ${materialId}`)
  }
  return (volumeMm3 / 1000) * spec.densityGPerCm3
}

export const solidMaterialRgb=(color:string):[number,number,number]=>[1,3,5].map(i=>parseInt(color.slice(i,i+2),16)/255) as [number,number,number]
export function assignSolidMaterial(document:DirectDocument,ids:readonly string[],material?:DirectMaterial):DirectDocument {
 if(material&&(!material.name.trim()||material.name.length>100||!/^#[0-9a-f]{6}$/i.test(material.color)))throw Error('Invalid body material.')
 if(material&&[material.metallic,material.roughness,material.opacity].some(v=>v!==undefined&&(!Number.isFinite(v)||v<0||v>1)))throw Error('Invalid body material.')
 const next=structuredClone(document)
 for(const body of next.bodies)if(ids.includes(body.id)){
  if(material)body.material=structuredClone(material)
  else delete body.material
 }
 return next
}
