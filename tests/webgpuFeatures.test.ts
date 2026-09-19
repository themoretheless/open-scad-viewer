import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  inspectWebGpuCapabilities,
  requiredWebGpuFeaturesForWgsl,
  requiredWgslLanguageFeatures,
  selectSupportedWgslVariant,
} from '../src/services/webgpuFeatures'

describe('WebGPU feature detection from WGSL', () => {
  afterEach(() => vi.unstubAllGlobals())
  it('does not request optional features for ordinary compute shaders', () => {
    expect(requiredWebGpuFeaturesForWgsl('@compute @workgroup_size(1) fn main() {}')).toEqual([])
  })

  it('requests subgroups when WGSL enables subgroup builtins', () => {
    expect(requiredWebGpuFeaturesForWgsl('enable subgroups;\n@compute @workgroup_size(64) fn main() {}'))
      .toEqual(['subgroups'])
  })

  it('requests subgroup-size-control and its dependency for controlled subgroup shaders', () => {
    expect(requiredWebGpuFeaturesForWgsl('enable subgroup_size_control;\n@subgroup_size(32) @compute @workgroup_size(64) fn main() {}'))
      .toEqual(['subgroups', 'subgroup-size-control'])
  })

  it('detects WGSL linear_indexing from requires or linear builtins', () => {
    expect(requiredWgslLanguageFeatures('requires linear_indexing;\n@compute @workgroup_size(64) fn main(@builtin(global_invocation_index) i: u32) {}'))
      .toEqual(['linear_indexing'])
    expect(requiredWgslLanguageFeatures('@compute @workgroup_size(64) fn main(@builtin(workgroup_index) i: u32) {}'))
      .toEqual(['linear_indexing'])
  })

  it('detects WGSL buffer_view from requires or buffer view builtins', () => {
    expect(requiredWgslLanguageFeatures('requires buffer_view;\n@compute @workgroup_size(1) fn main() {}'))
      .toEqual(['buffer_view'])
    expect(requiredWgslLanguageFeatures('@compute @workgroup_size(1) fn main() { _ = bufferLength(data); }'))
      .toEqual(['buffer_view'])
  })

  it('detects WGSL swizzle_assignment from requires directive', () => {
    expect(requiredWgslLanguageFeatures('requires swizzle_assignment;\nfn update() { var v = vec4f(); v.xy = vec2f(1.0); }'))
      .toEqual(['swizzle_assignment'])
  })

  it('detects newer WGSL language features from comma-separated requires directives', () => {
    expect(requiredWgslLanguageFeatures([
      'requires primitive_index, subgroup_id;',
      'requires subgroup_uniformity;',
      'requires texture_and_sampler_let, uniform_buffer_standard_layout;',
      'requires texture_formats_tier1, texture_formats_tier2;',
      '@compute @workgroup_size(1) fn main() {}',
    ].join('\n'))).toEqual([
      'primitive_index',
      'subgroup_id',
      'subgroup_uniformity',
      'texture_and_sampler_let',
      'uniform_buffer_standard_layout',
      'texture_formats_tier1',
      'texture_formats_tier2',
    ])
  })

  it('detects primitive and subgroup indexing builtins that require WGSL language support', () => {
    expect(requiredWgslLanguageFeatures('@fragment fn main(@builtin(primitive_index) primitive: u32) -> @location(0) vec4f { return vec4f(f32(primitive)); }'))
      .toEqual(['primitive_index'])
    expect(requiredWgslLanguageFeatures('@compute @workgroup_size(64) fn main(@builtin(num_subgroups) groups: u32) {}'))
      .toEqual(['subgroup_id'])
  })

  it('selects the first supported WGSL variant', () => {
    const adapter = { features: new Set() } as unknown as GPUAdapter
    vi.stubGlobal('navigator', { gpu: { wgslLanguageFeatures: new Set(['linear_indexing']) } })
    expect(selectSupportedWgslVariant(adapter, [
      { label: 'swizzle', wgsl: 'requires swizzle_assignment;\n@compute @workgroup_size(1) fn main() {}' },
      { label: 'buffer-view', wgsl: 'requires buffer_view;\n@compute @workgroup_size(1) fn main() {}' },
      { label: 'linear', wgsl: 'requires linear_indexing;\n@compute @workgroup_size(1) fn main(@builtin(global_invocation_index) i: u32) {}' },
      { label: 'baseline', wgsl: '@compute @workgroup_size(1) fn main() {}' },
    ]).label).toBe('linear')
  })

  it('reports a compact WebGPU capability snapshot', async () => {
    vi.stubGlobal('navigator', {
      gpu: {
        wgslLanguageFeatures: new Set(['buffer_view', 'linear_indexing']),
        requestAdapter: async () => ({
          features: new Set(['subgroups']),
          limits: { subgroupMinSize: 8, subgroupMaxSize: 32 },
        }),
      },
    })
    await expect(inspectWebGpuCapabilities()).resolves.toEqual({
      available: true,
      webgpuFeatures: ['subgroups'],
      wgslLanguageFeatures: ['buffer_view', 'linear_indexing'],
      subgroupMinSize: 8,
      subgroupMaxSize: 32,
    })
  })
})
