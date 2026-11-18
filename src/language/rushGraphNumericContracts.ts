/** Canonical units and numeric types reported by the native RushGraph compiler. */
export type Unit = 'mm' | 'cm' | 'm' | 'in' | 'deg' | 'rad'
export type Dimension = readonly [length: number, angle: number]
export type RushGraphNumericType = 'int' | 'f32' | 'f64' | 'length' | 'angle'
