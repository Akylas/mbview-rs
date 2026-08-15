import {
  addSourceToMap,
  applyOpacity,
  applyOrder,
  applyVisibility,
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

console.log(failures === 0 ? '\nall good' : `\n${failures} failing`);
process.exit(failures === 0 ? 0 : 1);
