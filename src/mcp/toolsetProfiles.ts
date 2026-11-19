export const MCP_TOOLSET_PROFILES = ['openscad', 'rush', 'direct', 'exchange', 'drawing', 'cam', 'cae'] as const

export type McpToolsetProfile = (typeof MCP_TOOLSET_PROFILES)[number]

export const ALL_MCP_TOOLSET_PROFILES: ReadonlySet<McpToolsetProfile> = new Set(MCP_TOOLSET_PROFILES)

const PROFILE_SET = new Set<string>(MCP_TOOLSET_PROFILES)

export function parseMcpToolsetProfiles(raw: string | undefined): ReadonlySet<McpToolsetProfile> {
  if (raw === undefined) return ALL_MCP_TOOLSET_PROFILES
  const trimmed = raw.trim()
  if (trimmed === '' || trimmed === 'all') return ALL_MCP_TOOLSET_PROFILES
  const tokens = trimmed.split(',').map(token => token.trim())
  if (tokens.some(token => token.length === 0)) {
    throw new TypeError(`--profiles must be "all" or a comma-separated list of: ${MCP_TOOLSET_PROFILES.join(', ')}`)
  }
  const parsed = new Set<McpToolsetProfile>()
  for (const token of tokens) {
    if (!PROFILE_SET.has(token)) {
      throw new TypeError(`Unknown MCP profile: ${token}. Valid profiles: ${MCP_TOOLSET_PROFILES.join(', ')}, all`)
    }
    parsed.add(token as McpToolsetProfile)
  }
  return parsed
}

export function hasProfile(profiles: ReadonlySet<McpToolsetProfile> | undefined, profile: McpToolsetProfile): boolean {
  return profiles === undefined || profiles.has(profile)
}
