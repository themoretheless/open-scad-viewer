import { describe, expect, it } from 'vitest'
import { sdfWgslVariants } from '../src/services/sdfGpu'

describe('SDF WebGPU WGSL variants', () => {
  it('builds a linear_indexing shader variant for the 1D SDF sweep', () => {
    const wgsl = [
      '@compute @workgroup_size(256)',
      'fn main(@builtin(global_invocation_id) id: vec3<u32>) {',
      '  if (id.x >= 16u) { return; }',
      '  values[id.x] = 1.0;',
      '}',
    ].join('\n')

    expect(sdfWgslVariants(wgsl)).toEqual([
      {
        label: 'sdf-linear-indexing',
        wgsl: [
          'requires linear_indexing;',
          '@compute @workgroup_size(256)',
          'fn main(@builtin(global_invocation_index) index: u32) {',
          '  if (index >= 16u) { return; }',
          '  values[index] = 1.0;',
          '}',
        ].join('\n'),
      },
      { label: 'sdf-baseline', wgsl },
    ])
  })

  it('keeps only the baseline when the shader uses multidimensional invocation ids', () => {
    const wgsl = '@compute @workgroup_size(16, 16)\nfn main(@builtin(global_invocation_id) id: vec3<u32>) { values[id.y] = f32(id.x); }'
    expect(sdfWgslVariants(wgsl)).toEqual([{ label: 'sdf-baseline', wgsl }])
  })
})
