import { afterEach, describe, expect, it, vi } from 'vitest'
import { runGpuCompute } from '../src/services/webgpuCompute'

describe('WebGPU compute feature negotiation', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('requests subgroup-size-control when WGSL uses @subgroup_size', async () => {
    vi.stubGlobal('GPUShaderStage', { COMPUTE: 4 })
    vi.stubGlobal('GPUBufferUsage', { STORAGE: 1, COPY_SRC: 2, MAP_READ: 4, COPY_DST: 8 })
    vi.stubGlobal('GPUMapMode', { READ: 1 })

    const requestDevice = vi.fn(async () => ({
      queue: { writeBuffer: vi.fn(), submit: vi.fn() },
      createShaderModule: vi.fn(() => ({})),
      createBindGroupLayout: vi.fn(() => ({})),
      createPipelineLayout: vi.fn(() => ({})),
      createComputePipeline: vi.fn(() => ({})),
      createCommandEncoder: vi.fn(() => ({
        beginComputePass: () => ({
          setPipeline: vi.fn(),
          setBindGroup: vi.fn(),
          dispatchWorkgroups: vi.fn(),
          end: vi.fn(),
        }),
        copyBufferToBuffer: vi.fn(),
        finish: vi.fn(() => ({})),
      })),
      createBuffer: vi.fn(() => ({
        destroy: vi.fn(),
        mapAsync: vi.fn(async () => undefined),
        getMappedRange: vi.fn(() => new ArrayBuffer(4)),
        unmap: vi.fn(),
      })),
      createBindGroup: vi.fn(() => ({})),
      destroy: vi.fn(),
    }))
    vi.stubGlobal('navigator', {
      gpu: {
        wgslLanguageFeatures: new Set(),
        requestAdapter: async () => ({
          features: new Set(['subgroups', 'subgroup-size-control']),
          requestDevice,
        }),
      },
    })

    await runGpuCompute({
      wgsl: 'enable subgroups;\nenable subgroup_size_control;\n@subgroup_size(32) @compute @workgroup_size(64) fn main() {}',
      entryPoint: 'main',
      dispatches: [{
        buffers: [{ binding: 0, data: new Float32Array(0), output: true }],
        outputBytes: 4,
        workgroups: [1, 1, 1],
      }],
    })

    expect(requestDevice).toHaveBeenCalledWith({ requiredFeatures: ['subgroups', 'subgroup-size-control'] })
  })

  it('rejects buffer_view shaders when WGSL language support is unavailable', async () => {
    const requestDevice = vi.fn()
    vi.stubGlobal('navigator', {
      gpu: {
        wgslLanguageFeatures: new Set(),
        requestAdapter: async () => ({
          features: new Set(),
          requestDevice,
        }),
      },
    })

    await expect(runGpuCompute({
      wgsl: 'requires buffer_view;\n@compute @workgroup_size(1) fn main() {}',
      entryPoint: 'main',
      dispatches: [{
        buffers: [{ binding: 0, data: new Float32Array(0), output: true }],
        outputBytes: 4,
        workgroups: [1, 1, 1],
      }],
    })).rejects.toThrow('WebGPU WGSL lacks required feature(s): buffer_view')
    expect(requestDevice).not.toHaveBeenCalled()
  })

  it('uses the first supported WGSL variant before creating the shader module', async () => {
    vi.stubGlobal('GPUShaderStage', { COMPUTE: 4 })
    vi.stubGlobal('GPUBufferUsage', { STORAGE: 1, COPY_SRC: 2, MAP_READ: 4, COPY_DST: 8 })
    vi.stubGlobal('GPUMapMode', { READ: 1 })

    const createShaderModule = vi.fn(() => ({}))
    const dispatchWorkgroups = vi.fn()
    vi.stubGlobal('navigator', {
      gpu: {
        wgslLanguageFeatures: new Set(['linear_indexing']),
        requestAdapter: async () => ({
          features: new Set(),
          requestDevice: async () => ({
            queue: { writeBuffer: vi.fn(), submit: vi.fn() },
            createShaderModule,
            createBindGroupLayout: vi.fn(() => ({})),
            createPipelineLayout: vi.fn(() => ({})),
            createComputePipeline: vi.fn(() => ({})),
            createCommandEncoder: vi.fn(() => ({
              beginComputePass: () => ({
                setPipeline: vi.fn(),
                setBindGroup: vi.fn(),
                dispatchWorkgroups,
                end: vi.fn(),
              }),
              copyBufferToBuffer: vi.fn(),
              finish: vi.fn(() => ({})),
            })),
            createBuffer: vi.fn(() => ({
              destroy: vi.fn(),
              mapAsync: vi.fn(async () => undefined),
              getMappedRange: vi.fn(() => new ArrayBuffer(4)),
              unmap: vi.fn(),
            })),
            createBindGroup: vi.fn(() => ({})),
            destroy: vi.fn(),
          }),
        }),
      },
    })

    const linear = 'requires linear_indexing;\n@compute @workgroup_size(1) fn main(@builtin(global_invocation_index) i: u32) {}'
    await runGpuCompute({
      wgsl: '@compute @workgroup_size(1) fn main() {}',
      wgslVariants: [
        { label: 'buffer-view', wgsl: 'requires buffer_view;\n@compute @workgroup_size(1) fn main() {}' },
        { label: 'linear', wgsl: linear },
      ],
      entryPoint: 'main',
      dispatches: [{
        buffers: [{ binding: 0, data: new Float32Array(0), output: true }],
        outputBytes: 4,
        workgroups: [1, 1, 1],
        variantWorkgroups: {
          linear: [4, 1, 1],
        },
      }],
    })

    expect(createShaderModule).toHaveBeenCalledWith({ code: linear })
    expect(dispatchWorkgroups).toHaveBeenCalledWith(4, 1, 1)
  })
})
