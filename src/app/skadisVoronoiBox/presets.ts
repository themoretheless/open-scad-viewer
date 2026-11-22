import { SKADIS_VORONOI_BOX_DEFAULTS, type SkadisVoronoiBoxParams } from './schema'

export interface SkadisVoronoiPreset {
  id: string
  name: { ru: string; en: string }
  description: { ru: string; en: string }
  params: SkadisVoronoiBoxParams
}

export const SKADIS_VORONOI_PRESETS: readonly SkadisVoronoiPreset[] = [
  {
    id: 'skadis_dovetail_voronoi',
    name: {
      ru: 'SKÅDIS · паз + Вороной (120×75×80)',
      en: 'SKÅDIS · Dovetail + Voronoi (120×75×80)',
    },
    description: {
      ru: 'Классическая коробка с асимметричным пазом SKÅDIS и скруглённой органической сеткой Вороного, как в генераторе коробок.',
      en: 'Standard IKEA SKÅDIS box with bottom-insert dovetail channels and organic rounded box-style Voronoi lattice.',
    },
    params: { ...SKADIS_VORONOI_BOX_DEFAULTS },
  },
  {
    id: 'skadis_mixed_lattice',
    name: {
      ru: 'SKÅDIS · Вороной стены + соты дно',
      en: 'SKÅDIS · Voronoi walls + Honeycomb floor',
    },
    description: {
      ru: 'Комбинированное заполнение: органическая решётка Вороного на стенках и жёсткие шестиугольные соты на дне.',
      en: 'Mixed infill: organic Voronoi wall lattice paired with regular honeycomb hexagons on the floor.',
    },
    params: {
      ...SKADIS_VORONOI_BOX_DEFAULTS,
      lattice_mode: 'trabecular',
      floor_lattice_mode: 'honeycomb',
      floor_cell_size: 14,
      floor_rib: 2.2,
    },
  },
  {
    id: 'skadis_compact',
    name: {
      ru: 'SKÅDIS · узкая 72×50×55 (шаг 40)',
      en: 'SKÅDIS · Compact 72×50×55 (40 mm pitch)',
    },
    description: {
      ru: 'Компактный органайзер на два соседних отверстия SKÅDIS через 40 мм.',
      en: 'Compact bin spanning two adjacent 40 mm SKÅDIS slots.',
    },
    params: {
      ...SKADIS_VORONOI_BOX_DEFAULTS,
      width: 72,
      depth: 50,
      height: 55,
      corner_radius: 4,
      skadis_spacing_steps: 1,
      skadis_top_offset: 12,
      cell_size: 13,
      floor_cell_size: 13,
    },
  },
  {
    id: 'skadis_wide',
    name: {
      ru: 'SKÅDIS · широкая 220×100×110 (3 крючка)',
      en: 'SKÅDIS · Wide 220×100×110 (3 hooks)',
    },
    description: {
      ru: 'Широкий лоток с тремя крючками через 80 мм, двумя перегородками и наклонным передом.',
      en: 'Wide tray with three 80 mm spaced hooks, two open Voronoi dividers and 10° tilted front.',
    },
    params: {
      ...SKADIS_VORONOI_BOX_DEFAULTS,
      width: 220,
      depth: 100,
      height: 110,
      front_angle: 10,
      dividers: 2,
      hook_count: 3,
      skadis_spacing_steps: 2,
      cell_size: 18,
      floor_cell_size: 18,
    },
  },
  {
    id: 'skadis_print_plate',
    name: {
      ru: 'SKÅDIS · стол для печати (корпус + крючки)',
      en: 'SKÅDIS · Full print plate (box + hooks)',
    },
    description: {
      ru: 'Раскладка на стол принтера: корпус дном вниз и крючки на плоском боку (z = 0).',
      en: 'Ready-to-slice bed layout: upright box plus flat-lying dovetail hooks on z = 0.',
    },
    params: {
      ...SKADIS_VORONOI_BOX_DEFAULTS,
      skadis_part: 3,
      print_optimized: true,
    },
  },
  {
    id: 'rail_voronoi',
    name: {
      ru: 'Универсальная рейка · Вороной + съёмные крючки',
      en: 'Universal Rail · Voronoi + dovetail hooks',
    },
    description: {
      ru: 'Коробка под произвольную рейку с точками Бесселя и раздельными крючками.',
      en: 'Generic rail bin with Bessel-spaced dovetail hooks and Voronoi lattice walls.',
    },
    params: {
      ...SKADIS_VORONOI_BOX_DEFAULTS,
      mount_system: 'rail_dovetail',
      corner_radius: 6,
      front_angle: 12,
    },
  },
]
