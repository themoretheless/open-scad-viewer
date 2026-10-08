/** Dependency-light probe; the compiler itself stays behind a lazy import. */
export const isRush = (source: string) => /^\s*\/\/\s*@rush(?:\/1)?(?=\s|$)/.test(source)
export const isRushFrontend = (source: string) => isRush(source) || /^\s*\/\/\s*@modelgraph-text\/1\b/.test(source)

/** Rush uses .r; legacy RushGraph Text keeps .mg. */
export const sourceFileExtension = (source: string) => (isRush(source) ? '.r' : isRushFrontend(source) ? '.mg' : '.scad')
export const SOURCE_FILE_EXTENSION = /\.(scad|mg|r)$/i
export const SOURCE_FILE_ACCEPT = '.scad,.r,.mg,text/plain'
/** Preserve recognized source filenames; infer the extension for unnamed documents. */
export const withSourceExtension = (name: string, source: string) =>
  SOURCE_FILE_EXTENSION.test(name) ? name : `${name}${sourceFileExtension(source)}`
