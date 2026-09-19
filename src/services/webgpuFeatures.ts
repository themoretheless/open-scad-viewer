type WebGpuFeatureName = GPUFeatureName | 'subgroups' | 'subgroup-size-control'
type WgslLanguageFeatureName =
  | 'linear_indexing'
  | 'immediate_address_space'
  | 'buffer_view'
  | 'swizzle_assignment'
  | 'primitive_index'
  | 'subgroup_id'
  | 'subgroup_uniformity'
  | 'texture_and_sampler_let'
  | 'texture_formats_tier1'
  | 'texture_formats_tier2'
  | 'uniform_buffer_standard_layout'

const KNOWN_WGSL_LANGUAGE_FEATURES = new Set<WgslLanguageFeatureName>([
  'buffer_view',
  'immediate_address_space',
  'linear_indexing',
  'primitive_index',
  'subgroup_id',
  'subgroup_uniformity',
  'swizzle_assignment',
  'texture_and_sampler_let',
  'texture_formats_tier1',
  'texture_formats_tier2',
  'uniform_buffer_standard_layout',
])

export interface WgslFeatureCheck {
  readonly webgpu: readonly WebGpuFeatureName[]
  readonly wgsl: readonly WgslLanguageFeatureName[]
}

export interface WgslVariant {
  readonly label: string
  readonly wgsl: string
}

export interface WebGpuCapabilitySnapshot {
  readonly available: boolean
  readonly webgpuFeatures: readonly string[]
  readonly wgslLanguageFeatures: readonly string[]
  readonly subgroupMinSize: number | null
  readonly subgroupMaxSize: number | null
}

export function requiredWebGpuFeaturesForWgsl(wgsl: string): WebGpuFeatureName[] {
  const features: WebGpuFeatureName[] = []
  if (/\benable\s+subgroups\b/.test(wgsl)) features.push('subgroups')
  if (/\benable\s+subgroup_size_control\b/.test(wgsl) || /@subgroup_size\s*\(/.test(wgsl)) {
    if (!features.includes('subgroups')) features.push('subgroups')
    features.push('subgroup-size-control')
  }
  return features
}

export function unsupportedWebGpuFeatures(adapter: GPUAdapter, features: readonly WebGpuFeatureName[]): string[] {
  return features.filter(feature => !adapter.features.has(feature as GPUFeatureName))
}

export function requiredWgslLanguageFeatures(wgsl: string): WgslLanguageFeatureName[] {
  const features: WgslLanguageFeatureName[] = []
  for (const match of wgsl.matchAll(/\brequires\s+([A-Za-z0-9_,\s]+?)\s*;/g)) {
    for (const feature of match[1]!.split(',').map(value => value.trim()).filter(Boolean)) {
      if (KNOWN_WGSL_LANGUAGE_FEATURES.has(feature as WgslLanguageFeatureName)
        && !features.includes(feature as WgslLanguageFeatureName)) {
        features.push(feature as WgslLanguageFeatureName)
      }
    }
  }
  if (/\brequires\s+linear_indexing\b/.test(wgsl)
    || /@builtin\s*\(\s*(global_invocation_index|workgroup_index)\s*\)/.test(wgsl)) {
    if (!features.includes('linear_indexing')) features.push('linear_indexing')
  }
  if (/\brequires\s+immediate_address_space\b/.test(wgsl) || /\bvar\s*<\s*immediate\s*>/.test(wgsl)) {
    if (!features.includes('immediate_address_space')) features.push('immediate_address_space')
  }
  if (/\brequires\s+buffer_view\b/.test(wgsl)
    || /\b(bufferView|bufferArrayView|bufferLength)\s*\(/.test(wgsl)) {
    if (!features.includes('buffer_view')) features.push('buffer_view')
  }
  if (/@builtin\s*\(\s*primitive_index\s*\)/.test(wgsl)) {
    if (!features.includes('primitive_index')) features.push('primitive_index')
  }
  if (/@builtin\s*\(\s*(subgroup_id|num_subgroups)\s*\)/.test(wgsl)) {
    if (!features.includes('subgroup_id')) features.push('subgroup_id')
  }
  return features
}

export function unsupportedWgslLanguageFeatures(features: readonly WgslLanguageFeatureName[]): string[] {
  const supported = typeof navigator !== 'undefined' ? navigator.gpu?.wgslLanguageFeatures : undefined
  return features.filter(feature => supported?.has(feature) !== true)
}

export function requiredFeaturesForWgsl(wgsl: string): WgslFeatureCheck {
  return {
    webgpu: requiredWebGpuFeaturesForWgsl(wgsl),
    wgsl: requiredWgslLanguageFeatures(wgsl),
  }
}

export function unsupportedFeaturesForWgsl(adapter: GPUAdapter, wgsl: string): string[] {
  const required = requiredFeaturesForWgsl(wgsl)
  return [
    ...unsupportedWgslLanguageFeatures(required.wgsl),
    ...unsupportedWebGpuFeatures(adapter, required.webgpu),
  ]
}

export function selectSupportedWgslVariant(adapter: GPUAdapter, variants: readonly WgslVariant[]): WgslVariant {
  for (const variant of variants) {
    if (unsupportedFeaturesForWgsl(adapter, variant.wgsl).length === 0) return variant
  }
  const first = variants[0]
  if (!first) throw new Error('WebGPU compute job has no WGSL variant')
  const unsupported = unsupportedFeaturesForWgsl(adapter, first.wgsl)
  throw new Error(`WebGPU lacks required feature(s): ${unsupported.join(', ')}`)
}

export async function inspectWebGpuCapabilities(): Promise<WebGpuCapabilitySnapshot> {
  const adapter = await navigator.gpu?.requestAdapter({ powerPreference: 'high-performance' })
  if (!adapter) {
    return { available: false, webgpuFeatures: [], wgslLanguageFeatures: [], subgroupMinSize: null, subgroupMaxSize: null }
  }
  const limits = adapter.limits as GPUSupportedLimits & { subgroupMinSize?: number; subgroupMaxSize?: number }
  return {
    available: true,
    webgpuFeatures: [...adapter.features].sort(),
    wgslLanguageFeatures: [...(navigator.gpu?.wgslLanguageFeatures ?? [])].sort(),
    subgroupMinSize: limits.subgroupMinSize ?? null,
    subgroupMaxSize: limits.subgroupMaxSize ?? null,
  }
}
