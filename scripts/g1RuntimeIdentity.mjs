import {resolve} from 'node:path'

export function matchesNodeHost(environment, platform, architecture) {
  const os = {linux:'ubuntu', darwin:'macos', win32:'windows'}[platform]
  const arch = {x64:'x86_64', arm64:'aarch64'}[architecture]
  return os !== undefined && arch !== undefined
    && environment.os.startsWith(`${os}-`) && environment.architecture === arch
}

/** Invoke the frozen test runner through Node on every host, without npm lifecycle hooks or cmd wrappers. */
export function qualificationInvocation(command, nodeExecutable, repositoryRoot) {
  if (command.startsWith('npm test -- ')) {
    return {executable:nodeExecutable, args:[resolve(repositoryRoot,'node_modules/vitest/vitest.mjs'),
      ...command.slice('npm test -- '.length).split(/\s+/u).filter(Boolean)]}
  }
  if (command.startsWith('node ')) {
    return {executable:nodeExecutable, args:command.split(/\s+/u).filter(Boolean).slice(1)}
  }
  throw new Error(`Unsupported frozen command: ${command}`)
}
