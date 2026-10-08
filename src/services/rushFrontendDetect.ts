/** Dependency-light Rush source detection; the compiler stays behind a lazy import. */
export const isRush = (source: string) => /^\s*\/\/\s*@rush(?:\/1)?(?=\s|$)/.test(source)
export const isRushFrontend = isRush
export const sourceFileExtension = (source: string) => isRush(source) ? '.r' : '.scad'
export const SOURCE_FILE_EXTENSION = /\.(scad|r)$/i
export const SOURCE_FILE_ACCEPT = '.scad,.r,text/plain'
export const withSourceExtension = (name: string, source: string) =>
  SOURCE_FILE_EXTENSION.test(name) ? name : `${name}${sourceFileExtension(source)}`
