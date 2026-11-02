// The stylized range: name, kind, colours, piece sizes, and the texture of
// ArchiSim's current range it would replace (for the comparison).
// dims: [a, b, joint, bevel] in metres (a, b: piece size per material).

export const RANGE = {
  // Walls
  brickRed:   { name: 'Ladrillo rojo',          group: 'Fachadas', kind: 1, base: '#c06a48', base2: '#8e4a36', joint: '#e6dccb', dims: [0.24, 0.075, 0.012, 0.012], old: 1, oldScale: 0.25 },
  brickOchre: { name: 'Ladrillo amarillo',      group: 'Fachadas', kind: 1, base: '#e0a066', base2: '#b07648', joint: '#f2ead8', dims: [0.24, 0.075, 0.012, 0.012], old: 1, oldScale: 0.25 },
  ashlar:     { name: 'Sillería',               group: 'Fachadas', kind: 2, base: '#cdbfa3', base2: '#b8a888', joint: '#e9e1cf', dims: [0.62, 0.32, 0.012, 0.03], old: 2, oldScale: 0.4 },
  rubble:     { name: 'Mampostería',            group: 'Fachadas', kind: 3, base: '#b4b0a6', base2: '#c7b79a', joint: '#dcd6c8', dims: [0.32, 0, 0.02, 0.05], old: 2, oldScale: 0.4 },
  stuccoYel:  { name: 'Estuco amarillo',        group: 'Fachadas', kind: 4, base: '#efcf6a', base2: '#efcf6a', joint: '#ffffff', dims: [0, 0, 0, 0], old: 7, oldScale: 0.5 },
  stuccoPink: { name: 'Estuco almagra',         group: 'Fachadas', kind: 4, base: '#e2a48c', base2: '#e2a48c', joint: '#ffffff', dims: [0, 0, 0, 0], old: 7, oldScale: 0.5 },
  stuccoWhite:{ name: 'Encalado',               group: 'Fachadas', kind: 4, base: '#f4efe4', base2: '#f4efe4', joint: '#ffffff', dims: [0, 0, 0, 0], old: 7, oldScale: 0.5 },
  clapBlue:   { name: 'Tablas solapadas azul',  group: 'Fachadas', kind: 5, base: '#7d9fd0', base2: '#7d9fd0', joint: '#5a769c', dims: [0.16, 3.2, 0, 0], old: 5, oldScale: 0.15 },
  clapMint:   { name: 'Tablas solapadas verde', group: 'Fachadas', kind: 5, base: '#a9d3b5', base2: '#a9d3b5', joint: '#5a769c', dims: [0.16, 3.2, 0, 0], old: 5, oldScale: 0.15 },
  concrete:   { name: 'Hormigón suave',         group: 'Fachadas', kind: 16, base: '#c9c4bb', base2: '#c9c4bb', joint: '#999', dims: [2.4, 1.2, 0.01, 0], old: 6, oldScale: 1.2 },
  // Roofs
  shingleGrey:{ name: 'Tejuela pizarra',        group: 'Cubiertas', kind: 6, base: '#5e646e', base2: '#7c8494', joint: '#3e434b', dims: [0.22, 0.17, 0.012, 0], old: 3, oldScale: 0.6 },
  shingleGreen:{ name: 'Tejuela verde',         group: 'Cubiertas', kind: 6, base: '#4f7a58', base2: '#6a946a', joint: '#33503a', dims: [0.26, 0.2, 0.012, 0], old: 3, oldScale: 0.6 },
  scales:     { name: 'Escamas',                group: 'Cubiertas', kind: 7, base: '#6d7590', base2: '#6d7590', joint: '#000', dims: [0.24, 0.16, 0, 0], old: 3, oldScale: 0.6 },
  barrel:     { name: 'Teja curva',             group: 'Cubiertas', kind: 8, base: '#bc6c47', base2: '#a87d55', joint: '#000', dims: [0.22, 0.4, 0, 0], old: 3, oldScale: 0.6 },
  copper:     { name: 'Cobre con pátina',       group: 'Cubiertas', kind: 9, base: '#5e9a80', base2: '#86c0a4', joint: '#000', dims: [0.45, 0, 0, 0], old: 11, oldScale: 0.08 },
  zinc:       { name: 'Zinc',                   group: 'Cubiertas', kind: 9, base: '#8d97a3', base2: '#a5afba', joint: '#000', dims: [0.45, 0, 0, 0], old: 11, oldScale: 0.08 },
  // Ground
  pavers:     { name: 'Adoquín',                group: 'Suelos', kind: 10, base: '#b9ae9c', base2: '#9c8f7c', joint: '#857a6a', dims: [0.2, 0.1, 0.01, 0.012], old: 3, oldScale: 0.6 },
  slabs:      { name: 'Losa de acera',          group: 'Suelos', kind: 11, base: '#d6d0c4', base2: '#d6d0c4', joint: '#eee9de', dims: [0.5, 0.5, 0.008, 0.006], old: 3, oldScale: 0.6 },
  asphalt:    { name: 'Asfalto',                group: 'Suelos', kind: 15, base: '#5f6470', base2: '#6f7480', joint: '#000', dims: [0, 0, 0, 0], old: 7, oldScale: 0.5 },
  planks:     { name: 'Tarima',                 group: 'Suelos', kind: 14, base: '#c08a58', base2: '#c08a58', joint: '#6b4a30', dims: [0.14, 1.8, 0.004, 0], old: 5, oldScale: 0.15 },
  grass:      { name: 'Césped',                 group: 'Suelos', kind: 12, base: '#7fb24c', base2: '#b8cf62', joint: '#000', dims: [0, 0, 0, 0], old: 8, oldScale: 0.3 },
  strata:     { name: 'Roca en estratos',       group: 'Suelos', kind: 13, base: '#e3cc98', base2: '#cfb27c', joint: '#000', dims: [0, 0.75, 0, 0], old: 19, oldScale: 0.5 },
  water:      { name: 'Agua',                   group: 'Suelos', kind: 17, base: '#4aa6c6', base2: '#8fd3e0', joint: '#000', dims: [0, 0, 0, 0], old: 10, oldScale: 1.0 },
  // Details
  trimCream:  { name: 'Pintura crema',          group: 'Detalles', kind: 19, base: '#f3ecd8', base2: '#f3ecd8', joint: '#000', dims: [0, 0, 0, 0], old: 0, oldScale: 1 },
  trimGreen:  { name: 'Pintura verde carruaje', group: 'Detalles', kind: 19, base: '#3f7a52', base2: '#3f7a52', joint: '#000', dims: [0, 0, 0, 0], old: 0, oldScale: 1 },
  trimBrown:  { name: 'Pintura chocolate',      group: 'Detalles', kind: 19, base: '#5a3a2c', base2: '#5a3a2c', joint: '#000', dims: [0, 0, 0, 0], old: 0, oldScale: 1 },
  trimRed:    { name: 'Pintura granate',        group: 'Detalles', kind: 19, base: '#a8433c', base2: '#a8433c', joint: '#000', dims: [0, 0, 0, 0], old: 0, oldScale: 1 },
  glass:      { name: 'Vidrio pintado',         group: 'Detalles', kind: 18, base: '#3d5a6e', base2: '#b8dcea', joint: '#000', dims: [0, 0, 0, 0], old: 0, oldScale: 1 },
  foliage:    { name: 'Follaje',                group: 'Detalles', kind: 20, base: '#4f8f45', base2: '#9cc65a', joint: '#000', dims: [0, 0, 0, 0], old: 0, oldScale: 1 },
  pole:       { name: 'Madera de poste',        group: 'Detalles', kind: 19, base: '#8a6244', base2: '#8a6244', joint: '#000', dims: [0, 0, 0, 0], old: 0, oldScale: 1 },
};
