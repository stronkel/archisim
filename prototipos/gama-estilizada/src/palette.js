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
  grass:      { name: 'Césped',                 group: 'Suelos', kind: 12, base: '#76ad4a', base2: '#b9d36a', joint: '#000', dims: [0, 0, 0, 0], old: 8, oldScale: 0.3 },
  strata:     { name: 'Roca en estratos',       group: 'Suelos', kind: 13, base: '#e8d2a0', base2: '#c7a16f', joint: '#000', dims: [0, 0.6, 0, 0], old: 19, oldScale: 0.5 },
  water:      { name: 'Agua de mar',            group: 'Suelos', kind: 17, base: '#1d6f9e', base2: '#8fd6e8', joint: '#000', dims: [0, 0, 0, 0], old: 10, oldScale: 1.0, water: [0.74, 0.97] },
  poolWater:  { name: 'Agua de piscina',        group: 'Suelos', kind: 21, base: '#47c4d8', base2: '#d9fbff', joint: '#000', dims: [0, 0, 0, 0], old: 10, oldScale: 1.0, water: [0.28, 0.85] },
  seabed:     { name: 'Fondo de arena',         group: 'Suelos', kind: 4, base: '#cdb98a', base2: '#cdb98a', joint: '#fff', dims: [0, 0, 0, 0], old: 7, oldScale: 0.5, hidden: true },
  gresite:    { name: 'Gresite',                group: 'Suelos', kind: 11, base: '#6cc2d6', base2: '#6cc2d6', joint: '#e8f6f8', dims: [0.1, 0.1, 0.004, 0.002], old: 3, oldScale: 0.6 },
  // Pavings
  pavDiag:    { name: 'Adoquín en diagonal',    group: 'Pavimentos', kind: 22, base: '#b3a796', base2: '#8f8475', joint: '#7a7062', dims: [0.2, 0.1, 0.01, 0.012], old: 3, oldScale: 0.6 },
  pavFan:     { name: 'Adoquín en abanico',     group: 'Pavimentos', kind: 23, base: '#a9a49c', base2: '#8a8178', joint: '#6e675e', dims: [2.0, 0.12, 0.012, 0.012], old: 3, oldScale: 0.6 },
  cobble:     { name: 'Empedrado',              group: 'Pavimentos', kind: 24, base: '#a49a8c', base2: '#c2b49a', joint: '#4f4a43', dims: [0.16, 0, 0.018, 0], old: 2, oldScale: 0.4 },
  oldCobble:  { name: 'Empedrado antiguo',      group: 'Pavimentos', kind: 25, base: '#9c9081', base2: '#b8a78c', joint: '#5e5444', dims: [0.24, 0, 0.03, 0.04], old: 2, oldScale: 0.4 },
  opus:       { name: 'Opus romano',            group: 'Pavimentos', kind: 26, base: '#d8c6a4', base2: '#c4a986', joint: '#efe6d3', dims: [0.15, 0, 0.008, 0.008], old: 3, oldScale: 0.6 },
  flag:       { name: 'Laja',                   group: 'Pavimentos', kind: 27, base: '#a9a196', base2: '#b88e6a', joint: '#d9d1c2', dims: [0.5, 0, 0.025, 0.02], old: 2, oldScale: 0.4 },
  flagGrass:  { name: 'Laja en césped',         group: 'Pavimentos', kind: 28, base: '#b5ab9c', base2: '#b5ab9c', joint: '#76ad4a', dims: [0.7, 0, 0, 0], old: 2, oldScale: 0.4 },
  marble:     { name: 'Mármol',                 group: 'Pavimentos', kind: 29, base: '#f1eee8', base2: '#9a9aa2', joint: '#000', dims: [0.6, 0.6, 0.003, 0], old: 18, oldScale: 0.8 },
  pebbles:    { name: 'Guijarros',              group: 'Pavimentos', kind: 30, base: '#9b968e', base2: '#ece6da', joint: '#3e3a35', dims: [0.09, 0, 0, 0], old: 9, oldScale: 0.08 },
  gravel:     { name: 'Grava',                  group: 'Pavimentos', kind: 31, base: '#c9c1b2', base2: '#9c8f7a', joint: '#6f675b', dims: [0.025, 0, 0, 0], old: 9, oldScale: 0.08 },
  // Wall tiles
  metro:      { name: 'Azulejo metro blanco',   group: 'Alicatados', kind: 32, base: '#f2f1ec', base2: '#f2f1ec', joint: '#d9d6cf', dims: [0.2, 0.1, 0.003, 0.006], old: 3, oldScale: 0.6 },
  metroGreen: { name: 'Azulejo metro verde',    group: 'Alicatados', kind: 32, base: '#5f9c86', base2: '#5f9c86', joint: '#e8e4da', dims: [0.15, 0.075, 0.003, 0.006], old: 3, oldScale: 0.6 },
  glazed:     { name: 'Azulejo liso 15 × 15',   group: 'Alicatados', kind: 11, base: '#e9e5dc', base2: '#e9e5dc', joint: '#f8f6f1', dims: [0.15, 0.15, 0.003, 0.003], old: 3, oldScale: 0.6 },
  sevillano:  { name: 'Azulejo andaluz',        group: 'Alicatados', kind: 33, base: '#2f6aa3', base2: '#d9a03b', joint: '#ece6d8', dims: [0.15, 0, 0.003, 0], old: 3, oldScale: 0.6 },
  hidraulico: { name: 'Baldosa hidráulica',     group: 'Alicatados', kind: 33, base: '#9a4a3c', base2: '#2f4f4a', joint: '#d8d0c0', dims: [0.2, 0, 0.002, 0], old: 3, oldScale: 0.6 },
  // Earth
  earthPath:  { name: 'Tierra de camino',       group: 'Tierras', kind: 34, base: '#b08a62', base2: '#c9a67c', joint: '#000', dims: [0, 0, 0, 0], old: 19, oldScale: 0.5 },
  albero:     { name: 'Albero',                 group: 'Tierras', kind: 35, base: '#e1b25a', base2: '#e1b25a', joint: '#000', dims: [0, 0, 0, 0], old: 19, oldScale: 0.5 },
  furrows:    { name: 'Tierra en surcos',       group: 'Tierras', kind: 36, base: '#8c6444', base2: '#7fae4a', joint: '#000', dims: [0.7, 0, 0, 0], old: 19, oldScale: 0.5 },
  garden:     { name: 'Huerta',                 group: 'Tierras', kind: 37, base: '#7d5a3e', base2: '#5e9a3e', joint: '#000', dims: [0.8, 0, 0, 0], old: 19, oldScale: 0.5 },
  cracked:    { name: 'Tierra cuarteada',       group: 'Tierras', kind: 38, base: '#c49a6c', base2: '#c49a6c', joint: '#000', dims: [0.35, 0, 0.012, 0], old: 19, oldScale: 0.5 },
  // Details
  trimCream:  { name: 'Pintura crema',          group: 'Detalles', kind: 19, base: '#f3ecd8', base2: '#f3ecd8', joint: '#000', dims: [0, 0, 0, 0], old: 0, oldScale: 1 },
  trimGreen:  { name: 'Pintura verde carruaje', group: 'Detalles', kind: 19, base: '#3f7a52', base2: '#3f7a52', joint: '#000', dims: [0, 0, 0, 0], old: 0, oldScale: 1 },
  trimBrown:  { name: 'Pintura chocolate',      group: 'Detalles', kind: 19, base: '#5a3a2c', base2: '#5a3a2c', joint: '#000', dims: [0, 0, 0, 0], old: 0, oldScale: 1 },
  trimRed:    { name: 'Pintura granate',        group: 'Detalles', kind: 19, base: '#a8433c', base2: '#a8433c', joint: '#000', dims: [0, 0, 0, 0], old: 0, oldScale: 1 },
  glass:      { name: 'Vidrio pintado',         group: 'Detalles', kind: 18, base: '#3d5a6e', base2: '#b8dcea', joint: '#000', dims: [0, 0, 0, 0], old: 0, oldScale: 1 },
  foliage:    { name: 'Follaje',                group: 'Detalles', kind: 20, base: '#4f8f45', base2: '#9cc65a', joint: '#000', dims: [0, 0, 0, 0], old: 0, oldScale: 1 },
  pole:       { name: 'Madera de poste',        group: 'Detalles', kind: 19, base: '#8a6244', base2: '#8a6244', joint: '#000', dims: [0, 0, 0, 0], old: 0, oldScale: 1 },
};
