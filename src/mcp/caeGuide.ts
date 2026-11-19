import type { McpServer } from '@modelcontextprotocol/server'
import { z } from 'zod/v4'

export const caeVec3Schema = z.tuple([z.number().finite(), z.number().finite(), z.number().finite()])
export const caeBool3Schema = z.tuple([z.boolean(), z.boolean(), z.boolean()])
export const caeBool6Schema = z.tuple([z.boolean(), z.boolean(), z.boolean(), z.boolean(), z.boolean(), z.boolean()])

export const CAE_GUIDE = {
  schema: 'openscad-viewer/cae-guide-v1',
  units: {
    length: 'mm',
    force: 'N',
    stress: 'MPa (N/mm^2)',
    moment: 'N*mm',
    density: 't/mm^3 (e.g. steel ~ 7.85e-9)',
  },
  disclaimer:
    'Screening/linear finite-element solvers in WASM Rust; outputs do not substitute for stamped engineering certification.',
  budgets: {
    truss: { maxNodes: 512, maxMembers: 2048 },
    frame: { maxNodes: 256, maxMembers: 1024 },
    printedSolid: { maxNodes: 125, maxTets: 400, maxBonds: 200 },
  },
  examples: {
    truss: {
      nodes_mm: [
        [0, 0, 0],
        [1000, 0, 0],
        [500, 866, 0],
      ],
      members: [
        { nodes: [0, 1], young_mpa: 200000, area_mm2: 100 },
        { nodes: [0, 2], young_mpa: 200000, area_mm2: 100 },
        { nodes: [1, 2], young_mpa: 200000, area_mm2: 100 },
      ],
      restrained: [
        [true, true, true],
        [false, true, true],
        [false, false, true],
      ],
      forces_n: [
        [0, 0, 0],
        [0, 0, 0],
        [0, -1000, 0],
      ],
    },
    frame: {
      nodes_mm: [
        [0, 0, 0],
        [1000, 0, 0],
      ],
      members: [
        {
          nodes: [0, 1],
          young_mpa: 200000,
          poisson: 0.3,
          area_mm2: 400,
          iyy_mm4: 13333,
          izz_mm4: 13333,
          j_mm4: 22500,
        },
      ],
      restrained: [
        [true, true, true, true, true, true],
        [false, false, false, false, false, false],
      ],
      forces_n: [
        [0, 0, 0],
        [0, 0, -500],
      ],
      moments_nmm: [
        [0, 0, 0],
        [0, 0, 0],
      ],
    },
    printed_solid: {
      nodes_mm: [
        [0, 0, 0],
        [1, 0, 0],
        [0, 1, 0],
        [0, 0, -1],
        [0, 0, 0],
        [1, 0, 0],
        [0, 1, 0],
        [0, 0, 1],
      ],
      tets: [
        { nodes: [0, 1, 2, 3], region: 'shell', young_mpa: 2000, poisson: 0.3 },
        { nodes: [4, 5, 6, 7], region: 'infill', young_mpa: 2000, poisson: 0.3 },
      ],
      bonds: [
        {
          shell: [0, 1, 2],
          infill: [4, 5, 6],
          normal_stiffness_mpa_per_mm: 100,
          shear_stiffness_mpa_per_mm: 50,
          tension_mpa: 10,
          shear_mpa: 8,
          compression_mpa: 30,
          property_source: 'Synthetic; not measured',
        },
      ],
      restrained: [
        [true, true, true],
        [true, true, true],
        [true, true, true],
        [true, true, true],
        [false, false, false],
        [false, false, false],
        [false, false, false],
        [false, false, false],
      ],
      forces_n: [
        [0, 0, 0],
        [0, 0, 0],
        [0, 0, 0],
        [0, 0, 0],
        [0, 0, 1],
        [0, 0, 1],
        [0, 0, 1],
        [0, 0, 0],
      ],
      safety_factor: 1,
      profile: {
        material: 'custom',
        grade: 'Synthetic example',
        property_source: 'Synthetic; not measured',
        nozzle_mm: 0.4,
        line_width_mm: 0.45,
        layer_height_mm: 0.2,
        nozzle_temp_c: 210,
        bed_temp_c: 60,
      },
      service_temp_c: 20,
    },
  },
}

export function registerCaeGuideResource(server: McpServer): void {
  server.registerResource(
    'cae-guide',
    'cae://guide',
    {
      title: 'Structural CAE units, solver limits, and runnable input examples',
      description:
        'Units, disclaimer, element budgets, and runnable fixtures for cae_truss_analyze, cae_frame_analyze, and cae_printed_solid_analyze.',
      mimeType: 'application/json',
    },
    () => ({
      contents: [{ uri: 'cae://guide', mimeType: 'application/json', text: JSON.stringify(CAE_GUIDE, null, 2) }],
    }),
  )
}
