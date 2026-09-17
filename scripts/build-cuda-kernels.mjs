// Regenerates the checked-in PTX for the CUDA kernel ports (`*.cu` next to the
// Rust module that embeds them). The PTX is committed so the `cuda` cargo
// feature builds without the CUDA toolkit; only this script needs `nvcc`.
//
//   node scripts/build-cuda-kernels.mjs           # rebuild all
//   node scripts/build-cuda-kernels.mjs --check   # fail if committed PTX is stale
//
// `--arch` (default compute_75) picks the virtual architecture; the driver
// JIT-compiles PTX for the installed GPU. The PTX ISA version follows the nvcc
// release, so the run-time driver must be at least as new as the toolkit used
// here (CUDA 13.x toolkit → R580+ driver).
import {spawnSync} from 'node:child_process'
import {existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync} from 'node:fs'
import {tmpdir} from 'node:os'
import {resolve, join} from 'node:path'
import {fileURLToPath} from 'node:url'

const root = fileURLToPath(new URL('../', import.meta.url))
const kernels = [
  {cu: 'crates/sdf-core/src/sdf_grid.cu', ptx: 'crates/sdf-core/src/sdf_grid.ptx'},
  {cu: 'crates/math-core/src/nearest_neighbor.cu', ptx: 'crates/math-core/src/nearest_neighbor.ptx'},
  {cu: 'crates/math-core/src/distance_pairs.cu', ptx: 'crates/math-core/src/distance_pairs.ptx'},
  {cu: 'crates/math-core/src/distance_pair_sum.cu', ptx: 'crates/math-core/src/distance_pair_sum.ptx'},
  {cu: 'crates/math-core/src/nearest_two.cu', ptx: 'crates/math-core/src/nearest_two.ptx'},
  {cu: 'crates/math-core/src/chamfer.cu', ptx: 'crates/math-core/src/chamfer.ptx'},
  {cu: 'crates/math-core/src/point_bounds.cu', ptx: 'crates/math-core/src/point_bounds.ptx'},
  {cu: 'crates/math-core/src/point_moments.cu', ptx: 'crates/math-core/src/point_moments.ptx'},
  {cu: 'crates/math-core/src/point_cloud_stats.cu', ptx: 'crates/math-core/src/point_cloud_stats.ptx'},
  {cu: 'crates/photogrammetry-core/src/gpu/matching.cu', ptx: 'crates/photogrammetry-core/src/gpu/matching.ptx'},
]
const args = process.argv.slice(2)
const check = args.includes('--check')
const archIndex = args.indexOf('--arch')
const arch = archIndex >= 0 ? args[archIndex + 1] : 'compute_75'

function nvcc() {
  if (process.env.NVCC) return process.env.NVCC
  const cudaPath = process.env.CUDA_PATH
  if (cudaPath) {
    const candidate = join(cudaPath, 'bin', process.platform === 'win32' ? 'nvcc.exe' : 'nvcc')
    if (existsSync(candidate)) return candidate
  }
  return 'nvcc'
}

// nvcc needs a host C++ compiler; on Windows locate MSVC through vswhere when
// `cl.exe` is not already on PATH.
function hostCompilerArgs() {
  if (process.platform !== 'win32' || process.env.NVCC_CCBIN === '') return []
  if (process.env.NVCC_CCBIN) return ['-ccbin', process.env.NVCC_CCBIN]
  if (spawnSync('where.exe', ['cl.exe'], {stdio: 'ignore'}).status === 0) return []
  const vswhere = join(process.env['ProgramFiles(x86)'] ?? 'C:\\Program Files (x86)', 'Microsoft Visual Studio', 'Installer', 'vswhere.exe')
  if (!existsSync(vswhere)) return []
  const probe = spawnSync(vswhere, ['-latest', '-products', '*', '-requires', 'Microsoft.VisualStudio.Component.VC.Tools.x86.x64', '-property', 'installationPath'], {encoding: 'utf8'})
  const install = probe.stdout?.trim()
  if (!install) return []
  const msvc = join(install, 'VC', 'Tools', 'MSVC')
  if (!existsSync(msvc)) return []
  const versions = readdirSync(msvc).sort()
  const bin = join(msvc, versions[versions.length - 1], 'bin', 'Hostx64', 'x64')
  return existsSync(join(bin, 'cl.exe')) ? ['-ccbin', bin] : []
}

// Strip the toolkit/build banner so the committed text only changes when the
// kernel or nvcc version changes, not per machine.
function normalize(ptx) {
  return ptx.replace(/\r\n/g, '\n').replace(/^\/\/ Based on .*\n/gm, '')
}

const out = mkdtempSync(join(tmpdir(), 'osv-cuda-'))
const ccbin = hostCompilerArgs()
let stale = 0
try {
  for (const {cu, ptx} of kernels) {
    const target = join(out, 'kernel.ptx')
    const result = spawnSync(
      nvcc(),
      ['--ptx', '-O3', `-arch=${arch}`, ...ccbin, '-o', target, resolve(root, cu)],
      {cwd: root, stdio: 'inherit'},
    )
    if (result.error) throw result.error
    if (result.status !== 0) process.exit(result.status ?? 1)
    const fresh = normalize(readFileSync(target, 'utf8'))
    const committed = existsSync(resolve(root, ptx)) ? normalize(readFileSync(resolve(root, ptx), 'utf8')) : ''
    if (check) {
      if (fresh !== committed) {
        console.error(`stale PTX: ${ptx} (regenerate with node scripts/build-cuda-kernels.mjs)`)
        stale++
      }
    } else if (fresh !== committed) {
      writeFileSync(resolve(root, ptx), fresh)
      console.log(`wrote ${ptx}`)
    } else {
      console.log(`up to date ${ptx}`)
    }
  }
} finally {
  rmSync(out, {recursive: true, force: true})
}
if (stale) process.exit(1)
