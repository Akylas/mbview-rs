import {
  addSourceToMap,
  applyOpacity,
  applyTerrain,
  terrainMaxzoom,
  detectDemEncoding,
  applyOrder,
  applyVisibility,
  isOwnLayer,
  makeSource,
  mapLayerIds,
  removeSourceFromMap,
  savePrefs,
} from './sources';

/* a localStorage stand-in, since these functions persist per-file prefs */
const store: Record<string, string> = {};
(globalThis as any).localStorage = {
  getItem: (key: string) => (key in store ? store[key] : null),
  setItem: (key: string, value: string) => (store[key] = value),
  removeItem: (key: string) => delete store[key],
};

class FakeMap {
  style = { _layers: {} as Record<string, any> };
  sources: Record<string, any> = {};
  order: string[] = [];
  layers: Record<string, any> = {};

  addSource(id: string, spec: any) {
    this.sources[id] = spec;
  }
  getSource(id: string) {
    return this.sources[id];
  }
  removeSource(id: string) {
    delete this.sources[id];
  }
  addLayer(spec: any) {
    this.layers[spec.id] = {
      ...spec,
      layout: { ...(spec.layout ?? {}) },
      paint: { ...(spec.paint ?? {}) },
    };
    this.style._layers[spec.id] = this.layers[spec.id];
    this.order.push(spec.id);
  }
  getLayer(id: string) {
    return this.layers[id];
  }
  removeLayer(id: string) {
    delete this.layers[id];
    delete this.style._layers[id];
    this.order = this.order.filter((entry) => entry !== id);
  }
  moveLayer(id: string, before?: string) {
    this.order = this.order.filter((entry) => entry !== id);
    if (before) this.order.splice(this.order.indexOf(before), 0, id);
    else this.order.push(id);
  }
  setLayoutProperty(id: string, key: string, value: any) {
    this.layers[id].layout[key] = value;
  }
  setPaintProperty(id: string, key: string, value: any) {
    this.layers[id].paint[key] = value;
  }
  terrain: any = null;
  sky: any = null;
  pitch = 0;
  setTerrain(spec: any) {
    this.terrain = spec;
  }
  getTerrain() {
    return this.terrain;
  }
  setSky(spec: any) {
    this.sky = spec;
  }
  getSky() {
    return this.sky;
  }
  getPitch() {
    return this.pitch;
  }
  easeTo(options: any) {
    if (options.pitch != null) this.pitch = options.pitch;
  }
}

function tilejson(id: string, layers: string[]) {
  return {
    id,
    name: id,
    format: 'pbf',
    encoding: 'mvt',
    tiles: [`http://localhost:9872/${id}/tiles/{z}/{x}/{y}.pbf`],
    minzoom: 0,
    maxzoom: 14,
    vector_layers: layers.map((layer) => ({ id: layer })),
  };
}

let failures = 0;
function check(name: string, condition: boolean) {
  if (!condition) {
    failures++;
    console.error(`FAIL ${name}`);
  } else {
    console.log(`ok   ${name}`);
  }
}

function visibility(map: FakeMap, id: string) {
  return map.getLayer(id)?.layout?.visibility;
}

/* ------------------------------------------------------------------ */

const map = new FakeMap() as any;
const alpha = makeSource(
  { path: '/data/alpha.mbtiles' },
  tilejson('alpha', ['roads', 'water']),
  true,
);
const beta = makeSource({ path: '/data/beta.mbtiles' }, tilejson('beta', ['buildings']), true);

addSourceToMap(map, alpha, 'http://localhost:9872/alpha/tiles.json');
addSourceToMap(map, beta, 'http://localhost:9872/beta/tiles.json');

check('every layer gets four map layers', mapLayerIds(alpha).length === 2 * 4);
check('sources are registered', !!map.getSource('alpha') && !!map.getSource('beta'));

/* visibility is derived, so re-applying never resurrects a hidden layer */
alpha.layers.find((l) => l.id === 'water').visible = false;
applyVisibility(map, alpha, 'all');
check('hidden layer is hidden', visibility(map, '___alpha___water-lines') === 'none');
check('sibling stays visible', visibility(map, '___alpha___roads-lines') === 'visible');

// what used to break: adding another file re-ran the panel's reset
addSourceToMap(
  map,
  makeSource({ path: '/data/gamma.mbtiles' }, tilejson('gamma', ['poi']), true),
  'x',
);
applyVisibility(map, alpha, 'all');
applyVisibility(map, beta, 'all');
check(
  'hidden layer survives another file being added',
  visibility(map, '___alpha___water-lines') === 'none',
);

/* geometry filter */
applyVisibility(map, alpha, 'points');
check('filter hides lines', visibility(map, '___alpha___roads-lines') === 'none');
check('filter keeps points', visibility(map, '___alpha___roads-points') === 'visible');
check(
  'filter does not unhide a hidden layer',
  visibility(map, '___alpha___water-points') === 'none',
);
applyVisibility(map, alpha, 'all');
check(
  'clearing the filter restores lines',
  visibility(map, '___alpha___roads-lines') === 'visible',
);

/* source-level toggle */
alpha.visible = false;
applyVisibility(map, alpha, 'all');
check('hiding the file hides its layers', visibility(map, '___alpha___roads-lines') === 'none');
alpha.visible = true;
applyVisibility(map, alpha, 'all');
check(
  'showing the file leaves per-layer state alone',
  visibility(map, '___alpha___water-lines') === 'none',
);

/* opacity */
alpha.opacity = 0.5;
applyOpacity(map, alpha);
check(
  'opacity scales lines',
  map.getLayer('___alpha___roads-lines').paint['line-opacity'] === 0.375,
);
check(
  'opacity scales fills',
  map.getLayer('___alpha___roads-polygons').paint['fill-opacity'] === 0.05,
);

/* ordering: sources[0] must end up drawn last */
applyOrder(map, [beta, alpha]);
check(
  'the top source draws above the one below it',
  map.order.findIndex((id: string) => id.startsWith('___beta')) >
    map.order.map((id: string) => id.startsWith('___alpha')).lastIndexOf(true),
);

applyOrder(map, [alpha, beta]);
check(
  'reordering restacks them',
  map.order.findIndex((id: string) => id.startsWith('___alpha')) >
    map.order.map((id: string) => id.startsWith('___beta')).lastIndexOf(true),
);

/* prefs follow the file */
savePrefs(alpha);
const reopened = makeSource(
  { path: '/data/alpha.mbtiles' },
  tilejson('alpha', ['roads', 'water']),
  true,
);
check(
  'reopening restores hidden layers',
  reopened.layers.find((l) => l.id === 'water').visible === false,
);
check('reopening restores opacity', reopened.opacity === 0.5);

/* MLT tilejson spells the layer list differently */
const mlt = makeSource(
  { path: '/data/mlt.mbtiles' },
  {
    id: 'mlt',
    name: 'mlt',
    format: 'mlt',
    encoding: 'mlt',
    tiles: [],
    Layer: [{ id: 'transportation' }],
  },
  true,
);
check('MLT layers are read', mlt.layers.length === 1 && mlt.vector);
check('MLT encoding is detected', mlt.encoding === 'mlt');

/* removal cleans up */
removeSourceFromMap(map, beta);
check('removed source is gone', !map.getSource('beta'));
check('removed layers are gone', !map.order.some((id: string) => id.startsWith('___beta')));

/* numbers written as strings by the tile maker */
const stringly = makeSource(
  { path: '/data/s.pmtiles' },
  { ...tilejson('s', ['water']), minzoom: '5', maxzoom: '13', bounds: '3.6,44.1,7.2,46.5', center: [1, 2, 3] },
  false,
);
check('string maxzoom becomes a number', stringly.maxzoom === 13 && stringly.minzoom === 5);
check('string bounds become an array', Array.isArray(stringly.bounds) && stringly.bounds.length === 4 && stringly.bounds[0] === 3.6);

/* terrain */
const demJson = (extra: any = {}) => ({
  id: 'dem',
  name: 'dem',
  format: 'png',
  tiles: ['http://localhost:9872/dem/tiles/{z}/{x}/{y}.png'],
  minzoom: 0,
  maxzoom: 10,
  bounds: [5, 44, 8, 47],
  ...extra,
});
check('plain raster is not terrain', detectDemEncoding(demJson()) === null);
check('declared encoding is read', detectDemEncoding(demJson({ encoding: 'terrarium' })) === 'terrarium');
check('terrain-rgb in the name is mapbox', detectDemEncoding(demJson({ name: 'Alps Terrain-RGB' })) === 'mapbox');
check('vector files are never terrain', detectDemEncoding(tilejson('v', ['water'])) === null);

const demMap = new FakeMap() as any;
const dem = makeSource(
  { path: '/data/dem.pmtiles', sourceType: 'raster-dem', layerType: 'hillshade', demEncoding: 'terrarium' },
  demJson({ tile_size: 512 }),
  false,
);
check('dem source keeps its encoding', dem.demEncoding === 'terrarium');
check('dem source reads tile size', dem.tileSize === 512);
check('terrain defaults on', dem.terrain === true);
addSourceToMap(demMap, dem, 'http://localhost:9872/dem/tiles.json');
check('dem source is raster-dem', demMap.getSource('dem').type === 'raster-dem');
check('dem source passes encoding', demMap.getSource('dem').encoding === 'terrarium');
check('dem source passes tile size', demMap.getSource('dem').tileSize === 512);
check('dem source stays inside its bounds', demMap.getSource('dem').bounds?.[0] === 5);
check('dem hillshade layer is added', demMap.getLayer('___dem-layer')?.type === 'hillshade');
applyTerrain(demMap, dem);
check('terrain uses its own twin source', demMap.getTerrain()?.source === 'dem-terrain-10');
check('the hillshade source is left alone', !!demMap.getSource('dem'));
check('terrain gets a sky', !!demMap.getSky());
dem.terrainExaggeration = 2.5;
applyTerrain(demMap, dem);
check('exaggeration follows the slider', demMap.getTerrain()?.exaggeration === 2.5);
dem.visible = false;
applyTerrain(demMap, dem);
check('hiding the file drops the terrain', demMap.getTerrain() === null);
dem.visible = true;
dem.terrain = false;
applyTerrain(demMap, dem);
check('switching 3D off drops the terrain', demMap.getTerrain() === null);
dem.terrain = true;
applyTerrain(demMap, dem);
removeSourceFromMap(demMap, dem);
check('removing the source releases the terrain', demMap.getTerrain() === null && !demMap.getSource('dem'));
check('removing the source removes the twin', !demMap.getSource('dem-terrain-10'));

/* elevation may not out-zoom what is drawn over it */
const vec = makeSource({ path: '/data/v.pmtiles' }, { ...tilejson('v', ['water']), maxzoom: 11 }, false);
check('terrain zoom is capped by the lowest source', terrainMaxzoom(dem, [dem, vec]) === 10);
const deep = makeSource({ path: '/data/d.pmtiles' }, { ...tilejson('d', ['water']), maxzoom: 14 }, false);
check('a deeper source does not lower it', terrainMaxzoom(dem, [dem, deep]) === 10);
const shallow = makeSource({ path: '/data/s.pmtiles' }, { ...tilejson('s', ['water']), maxzoom: 7 }, false);
check('a shallower source does', terrainMaxzoom(dem, [dem, shallow]) === 7);
const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const pending: Promise<void>[] = [];
pending.push(
  (async () => {
    const capMap = new FakeMap() as any;
    const terrainCalls: any[] = [];
    const rawSetTerrain = capMap.setTerrain.bind(capMap);
    capMap.setTerrain = (spec: any) => (terrainCalls.push(spec?.source ?? null), rawSetTerrain(spec));
    addSourceToMap(capMap, dem, 'u');
    applyTerrain(capMap, dem);
    check('terrain starts on the twin', capMap.getTerrain()?.source === 'dem-terrain-10');
    applyTerrain(capMap, dem, [dem, shallow]);
    check('terrain pauses while the twin is swapped', capMap.getTerrain() === null);
    check('the twin is rebuilt at the cap', capMap.getSource('dem-terrain-7')?.maxzoom === 7 && !capMap.getSource('dem-terrain-10'));
    check('the hillshade source keeps its own', capMap.getSource('dem').maxzoom === 10);
    await sleep(200);
    check('terrain comes back on the new twin', capMap.getTerrain()?.source === 'dem-terrain-7');
    check('only one pause happened', terrainCalls.filter((call) => call === null).length === 1);
    applyTerrain(capMap, dem, [dem]);
    await sleep(200);
    check('the cap lifts when the shallow source goes', capMap.getSource('dem-terrain-10')?.maxzoom === 10 && !capMap.getSource('dem-terrain-7'));
    check('and terrain follows', capMap.getTerrain()?.source === 'dem-terrain-10');
    // switched off by the user during the pause: it must stay off
    applyTerrain(capMap, dem, [dem, shallow]);
    dem.terrain = false;
    await sleep(200);
    check('a switch-off during the pause is respected', capMap.getTerrain() === null);
    dem.terrain = true;
  })()
);

/* elevation shading never covers the data above it */
const stackMap = new FakeMap() as any;
const stackDem = makeSource({ path: '/data/sd.pmtiles', sourceType: 'raster-dem', layerType: 'hillshade' }, demJson({ id: 'sd', name: 'sd' }), false);
const stackVec = makeSource({ path: '/data/sv.pmtiles' }, tilejson('sv', ['water']), false);
addSourceToMap(stackMap, stackVec, 'u');
addSourceToMap(stackMap, stackDem, 'u');
applyOrder(stackMap, [stackDem, stackVec]); // the panel lists the DEM on top
check('the hillshade layer is ours, not basemap', isOwnLayer('___sd-layer'));
check('hillshade stays below vector layers', stackMap.order.indexOf('___sd-layer') < stackMap.order.findIndex((id: string) => id.startsWith('___sv')));
check('hillshade uses maplibre defaults', stackMap.getLayer('___sd-layer').paint['hillshade-method'] === undefined);

const detected = makeSource({ path: '/data/auto.mbtiles' }, demJson({ encoding: 'mapbox' }), false);
check('a self-declared DEM becomes raster-dem', detected.sourceType === 'raster-dem');
const plain = makeSource({ path: '/data/plain.mbtiles' }, demJson(), false);
check('an ordinary raster stays raster', plain.sourceType === undefined);

Promise.all(pending).then(() => {
  console.log(failures === 0 ? '\nall good' : `\n${failures} failing`);
  process.exit(failures === 0 ? 0 : 1);
});
