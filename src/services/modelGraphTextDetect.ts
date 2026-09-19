/** Dependency-light probe; the compiler itself stays behind a lazy import. */
export const isModelGraphText = (source: string) => /^\s*\/\/\s*@modelgraph-text\/1\b/.test(source)

/** ModelGraph Text documents are .mg files; everything else the editor holds is OpenSCAD. */
export const sourceFileExtension = (source: string) => (isModelGraphText(source) ? '.mg' : '.scad')
export const SOURCE_FILE_EXTENSION = /\.(scad|mg)$/i
export const SOURCE_FILE_ACCEPT = '.scad,.mg,text/plain'
/** Keeps a .scad or .mg name as given; anything else gets the extension the source calls for. */
export const withSourceExtension = (name: string, source: string) =>
  SOURCE_FILE_EXTENSION.test(name) ? name : `${name}${sourceFileExtension(source)}`
