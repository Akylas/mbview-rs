import { pointToTile } from '@mapbox/tilebelt';
import type { Map as MapLibreMap } from 'maplibre-gl';
import { randomColor } from 'randomcolor';
import { readJSON, writeJSON } from './persisted';
import type { GeometryFilter } from './settings';

export type MapKey = 'main' | 'secondary';
export type GeomKind = 'polygons' | 'lines' | 'points' | 'rasters';
/** How a terrain tileset packs elevation into RGB; the maplibre `raster-dem` `encoding`. */
export type DemEncoding = 'mapbox' | 'terrarium';

/** Every map layer this app creates carries this prefix, so the basemap's own
 * layers can be told apart from ours by name alone. */
export const OWN_PREFIX = '___';

export interface LayerEntry {
  /** the `source-layer` name inside the tiles */
  id: string;
  color: string;
  visible: boolean;
  minzoom?: number;
  maxzoom?: number;
  description?: string;
  /** ids of the maplibre layers drawn for this source-layer */
  mapLayers: Record<GeomKind, string[]>;
}

export interface SourceEntry {
  /** md5 of the path; also the maplibre source id */
  id: string;
  path: string;
  file: string;
  name: string;
  vector: boolean;
  encoding: 'mvt' | 'mlt';
  format?: string;
  tiles: string[];
  bounds?: number[];
  center?: number[];
  minzoom?: number;
  maxzoom?: number;
  attribution?: string;
  description?: string;
  visible: boolean;
  opacity: number;
  layers: LayerEntry[];
  /** what `setup_mbtiles` was asked for, so a reload rebuilds the same thing */
  sourceType?: string;
  layerType?: string;
  /** `raster-dem` sources only: how elevation is encoded in the pixels */
  demEncoding: DemEncoding;
  /** pixel size of the tiles, measured from the file; maplibre assumes 512 when not told */
  tileSize?: number;
  /** `raster-dem` sources only: whether the map is extruded into 3D from this source */
  terrain: boolean;
  terrainExaggeration: number;
  /** the raw tiles.json, kept for the info sheet */
  data: any;
}

/* -------------------------------------------------------------------------- */
/* naming                                                                     */
/* -------------------------------------------------------------------------- */

export function basename(path: string) {
  return path.split(/[\\/]/).pop() || path;
}

export function layerIdPrefix(sourceId: string, layerId: string) {
  return `${OWN_PREFIX}${sourceId}${OWN_PREFIX}${layerId}`;
}

export function isOwnLayer(id: string) {
  return id.startsWith(OWN_PREFIX);
}

/** The raster/raster-dem case draws a single layer for the whole source. */
export function rasterLayerId(sourceId: string) {
  // carries the own-layer prefix so the "show basemap" switch never mistakes it for a basemap layer
  return `${OWN_PREFIX}${sourceId}-layer`;
}

/* -------------------------------------------------------------------------- */
/* colours                                                                    */
/* -------------------------------------------------------------------------- */

/**
 * A stable colour per source-layer name, biased by what the name suggests so
 * water reads blue and roads read orange across every file you open.
 */
export function brightColor(layerId: string, dark: boolean, alpha?: number) {
  let luminosity = 'bright';
  let hue: string | null = null;

  if (/water|ocean|lake|sea|river/.test(layerId)) hue = 'blue';
  if (/state|country|place/.test(layerId)) hue = 'pink';
  if (/road|highway|transport/.test(layerId)) hue = 'orange';
  if (/contour|landuse/.test(layerId)) hue = 'yellow';
  if (/wood|forest|park|landcover/.test(layerId)) hue = 'green';
  if (/contour|building/.test(layerId)) hue = 'monochrome';
  if (/building/.test(layerId)) {
    luminosity = dark ? 'light' : 'dark';
    hue = 'monochrome';
  }

  const rgb = randomColor({ luminosity, hue, seed: layerId, format: 'rgbArray' }) as any;
  return `rgba(${(rgb as number[]).concat([alpha ?? 1]).join(', ')})`;
}

/* -------------------------------------------------------------------------- */
/* tilejson shape                                                             */
/* -------------------------------------------------------------------------- */

/**
 * tiles.json is written by whatever made the file, and numbers in it are often strings ("13",
 * "3.6,44.1,7.2,46.5"). maplibre does not coerce them: a string maxzoom quietly breaks zoom logic.
 */
export function number(value: unknown): number | undefined {
  const parsed = typeof value === 'string' && value.trim() !== '' ? Number(value) : value;
  return typeof parsed === 'number' && Number.isFinite(parsed) ? parsed : undefined;
}

export function numbers(value: unknown): number[] | undefined {
  const list = typeof value === 'string' ? value.split(',') : Array.isArray(value) ? value : null;
  if (!list) return undefined;
  const parsed = list.map((entry) => number(entry));
  return parsed.every((entry) => entry !== undefined) ? (parsed as number[]) : undefined;
}

/** MLT tiles.json spells the layer list `Layer`; MVT spells it `vector_layers`. */
export function vectorLayersOf(data: any): any[] {
  return data?.vector_layers || data?.Layer || [];
}

export function isVectorSource(data: any) {
  return vectorLayersOf(data).length > 0;
}

/**
 * `mvt` or `mlt`, as understood by the maplibre-gl `encoding` source property.
 * The backend reports it in the tiles.json it serves.
 */
export function tileEncoding(data: any): 'mvt' | 'mlt' {
  return data?.encoding === 'mlt' || data?.format === 'mlt' ? 'mlt' : 'mvt';
}

/* -------------------------------------------------------------------------- */
/* terrain                                                                    */
/* -------------------------------------------------------------------------- */

export function isDemSource(source: Pick<SourceEntry, 'sourceType'>) {
  return source.sourceType === 'raster-dem';
}

/**
 * The source that actually drives the 3D mesh. maplibre advises against sharing
 * one source between a hillshade layer and terrain (the two want different
 * tile resolutions and fight over the cache), so the same tiles are added twice.
 * The id carries the max zoom the twin was built with, see `fitTerrainZoom`.
 */
export function terrainSourceId(sourceId: string, maxzoom: number) {
  return `${terrainPrefix(sourceId)}${maxzoom}`;
}

function terrainPrefix(sourceId: string) {
  return `${sourceId}-terrain-`;
}

/**
 * What a terrain switch is given: which twin to extrude and by how much. One live object per
 * source and map, because maplibre's own 3D button keeps the options it was built with and
 * would otherwise keep pointing at a twin that has since been replaced.
 */
export interface TerrainOptions {
  source: string;
  exaggeration: number;
}

const terrainOptionsByMap = new WeakMap<object, Record<string, TerrainOptions>>();

export function terrainOptions(map: MapLibreMap, source: SourceEntry): TerrainOptions | undefined {
  return terrainOptionsByMap.get(map)?.[source.id];
}

/**
 * Whether a raster tileset says it is elevation data, and how it is packed.
 *
 * Only what the file states outright counts: guessing Terrarium from Mapbox
 * by looking at a name would turn a wrong guess into a screen full of spikes.
 */
export function detectDemEncoding(data: any): DemEncoding | null {
  if (isVectorSource(data)) return null;
  const declared = String(data?.encoding ?? '').toLowerCase();
  if (declared === 'terrarium' || declared === 'mapbox') return declared;
  const text = `${data?.name ?? ''} ${data?.description ?? ''}`;
  if (/terrarium/i.test(text)) return 'terrarium';
  if (/terrain[-_ ]?rgb|mapbox[-_ ]?dem/i.test(text)) return 'mapbox';
  return null;
}

export const MIN_EXAGGERATION = 0.5;
export const MAX_EXAGGERATION = 5;
export const MAX_PITCH = 85;
/** How far the camera tilts when terrain is first switched on, so the relief is visible at once. */
export const TERRAIN_PITCH = 60;

/**
 * Pixel width of a source's tiles. maplibre assumes 512 for raster sources, so
 * a 256px tileset drawn without this is requested one zoom too coarse and
 * looks blurry. Falls back to 256, the size nearly every MBTiles raster uses.
 */
export async function detectTileSize(source: SourceEntry): Promise<number> {
  const template = source.tiles?.[0];
  if (!template) return 256;
  try {
    const bounds = boundsOf(source);
    const [lng, lat] = bounds
      ? [(bounds[0] + bounds[2]) / 2, (bounds[1] + bounds[3]) / 2]
      : source.center?.length >= 2
        ? [source.center[0], source.center[1]]
        : [0, 0];
    // A regional file has no tile at its lowest zoom over most of the world, so ask where the
    // data is: the centre of its bounds, from the deepest level down, until a tile answers.
    const max = source.maxzoom ?? 14;
    const min = source.minzoom ?? 0;
    for (let zoom = max; zoom >= Math.max(min, max - 5); zoom--) {
      const [x, y, z] = pointToTile(lng, lat, zoom);
      const url = template
        .replace('{z}', String(z))
        .replace('{x}', String(x))
        .replace('{y}', String(y));
      const response = await fetch(url);
      if (!response.ok) continue;
      const bitmap = await createImageBitmap(await response.blob());
      const size = bitmap.width;
      bitmap.close?.();
      return size >= 128 && size <= 2048 ? size : 256;
    }
    return 256;
  } catch (error) {
    return 256;
  }
}

const SKY = {
  'sky-color': '#9ab9d8',
  'horizon-color': '#dfe8f1',
  'fog-color': '#f0f3f7',
  'sky-horizon-blend': 0.6,
  'horizon-fog-blend': 0.6,
  'fog-ground-blend': 0.7,
  'atmosphere-blend': ['interpolate', ['linear'], ['zoom'], 0, 1, 10, 1, 12, 0],
};

/**
 * Keeps the 3D terrain of one map in step with a `raster-dem` source: on when
 * the file is shown and its terrain switch is on, off when this file is the one
 * currently extruding the map and either stops being true. Idempotent, so it is
 * safe to call after every change that might matter.
 */
export function applyTerrain(map: MapLibreMap, source: SourceEntry, all?: SourceEntry[]) {
  if (!map || !map.style || !isDemSource(source)) return;
  const options = terrainOptions(map, source);
  if (!options || !map.getSource(options.source)) return;
  // the terrain is being swapped onto a new twin; it comes back by itself in a moment
  if (all && fitTerrainZoom(map, source, all)) return;
  options.exaggeration = source.terrainExaggeration;
  const on = source.visible && source.terrain;
  if (on) {
    map.setTerrain({ ...options });
  } else if (map.getTerrain()?.source === options.source) {
    map.setTerrain(null);
    if (map.getPitch() > 0) map.easeTo({ pitch: 0, duration: 300 });
  }
  applySky(map);
}

/**
 * The deepest zoom the elevation may be read at. maplibre looks elevation up per drawn tile, and
 * cannot do it for a tile that is overscaled (a vector source past its own maxzoom) while the
 * elevation is not: the lookup is off by the difference, which draws holes and cliffs. Capping the
 * elevation's maxzoom at the lowest of every source on the map keeps the two in step; the DEM is
 * then simply overscaled instead.
 */
export function terrainMaxzoom(source: SourceEntry, all: SourceEntry[]) {
  const limits = all
    .filter((other) => other.id !== source.id)
    .map((other) => other.maxzoom)
    .filter((zoom): zoom is number => typeof zoom === 'number');
  return Math.max(Math.min(source.maxzoom ?? 22, ...limits), source.minzoom ?? 0);
}

let terrainSuspended = false;

/**
 * True while this module is switching terrain off on purpose and is about to switch it back on.
 * The "terrain off" event of such a pause is not the user's doing and must not be read as such.
 */
export function isTerrainSuspended() {
  return terrainSuspended;
}

/** How long terrain stays off while the map lets go of the old elevation tiles. */
const TERRAIN_SETTLE_MS = 80;

/**
 * Replaces the terrain twin when the cap above has changed, e.g. after another file was added.
 *
 * While terrain is live this is done as off, swap, on: maplibre keeps per-tile elevation lookups
 * for the source it started with, and handing it a different one mid-flight draws tiles with the
 * old lookup (steps and walls). Returns whether terrain was paused and will be restored.
 */
function fitTerrainZoom(map: MapLibreMap, source: SourceEntry, all: SourceEntry[]) {
  const options = terrainOptions(map, source);
  if (!options) return false;
  const wanted = terrainMaxzoom(source, all);
  const wantedId = terrainSourceId(source.id, wanted);
  if (options.source === wantedId) return false;
  const previous = options.source;
  const active = map.getTerrain()?.source === previous;
  if (active) {
    terrainSuspended = true;
    try {
      map.setTerrain(null);
    } finally {
      terrainSuspended = false;
    }
  }
  if (map.getSource(previous)) map.removeSource(previous);
  if (!map.getSource(wantedId)) map.addSource(wantedId, demSpec(source, wanted));
  options.source = wantedId;
  if (!active) return false;
  setTimeout(() => {
    // whatever happened in the meantime wins: removed, hidden, or switched off by the user
    if (!map.style || terrainOptions(map, source) !== options || options.source !== wantedId) return;
    if (!(source.visible && source.terrain) || map.getTerrain()) return;
    map.setTerrain({ ...options });
    applySky(map);
  }, TERRAIN_SETTLE_MS);
  return true;
}

/**
 * A sky while there is terrain, none otherwise. Separate from `applyTerrain` because the
 * terrain can also be switched by maplibre's own control, which knows nothing about this.
 */
export function applySky(map: MapLibreMap) {
  if (!map || !map.style) return;
  // a tilted map without a sky is a plane ending in a hard edge
  if (map.getTerrain() && !map.getSky()) map.setSky(SKY as any);
  else if (!map.getTerrain() && map.getSky()) map.setSky(undefined);
}

/** Which `raster-dem` source, if any, is extruding `map` right now. */
export function terrainOwner(map: MapLibreMap, sources: SourceEntry[]) {
  const current = map?.style ? map.getTerrain()?.source : undefined;
  return current ? sources.find((source) => current.startsWith(terrainPrefix(source.id))) : undefined;
}

/** Whether `map` currently shows 3D relief. */
export function hasTerrain(map: MapLibreMap) {
  return !!map?.style && !!map.getTerrain();
}

/* -------------------------------------------------------------------------- */
/* per-file preferences                                                       */
/* -------------------------------------------------------------------------- */

interface StoredPrefs {
  visible?: boolean;
  opacity?: number;
  terrain?: boolean;
  terrainExaggeration?: number;
  layers?: Record<string, boolean>;
}

const PREFS_KEY = 'mbview.prefs';

function allPrefs(): Record<string, StoredPrefs> {
  return readJSON<Record<string, StoredPrefs>>(PREFS_KEY, {});
}

/**
 * Visibility and opacity follow the *file*, not the slot it happens to occupy,
 * so hiding a layer survives a reload, a re-open, and being moved between the
 * two compare maps.
 */
export function savePrefs(source: SourceEntry) {
  const prefs = allPrefs();
  prefs[source.path] = {
    visible: source.visible,
    opacity: source.opacity,
    terrain: source.terrain,
    terrainExaggeration: source.terrainExaggeration,
    layers: source.layers.reduce(
      (acc, layer) => {
        acc[layer.id] = layer.visible;
        return acc;
      },
      {} as Record<string, boolean>,
    ),
  };
  writeJSON(PREFS_KEY, prefs);
}

function loadPrefs(path: string): StoredPrefs {
  return allPrefs()[path] ?? {};
}

/* -------------------------------------------------------------------------- */
/* building a SourceEntry                                                     */
/* -------------------------------------------------------------------------- */

export function makeSource(
  {
    path,
    sourceType,
    layerType,
    demEncoding,
  }: { path: string; sourceType?: string; layerType?: string; demEncoding?: DemEncoding },
  data: any,
  dark: boolean,
): SourceEntry {
  const prefs = loadPrefs(path);
  const vector = isVectorSource(data);
  const id = data.id;

  // a raster that declares itself elevation data does not have to be added "as terrain" by hand
  const detected = detectDemEncoding(data);
  if (!vector && !sourceType && detected) {
    sourceType = 'raster-dem';
    layerType = layerType ?? 'hillshade';
  }
  if (vector) sourceType = layerType = undefined;

  const layers: LayerEntry[] = vector
    ? vectorLayersOf(data)
        .map((layer: any) => {
          const layerId = layer.id ?? layer.name;
          return {
            id: layerId,
            color: brightColor(layerId, dark),
            visible: prefs.layers?.[layerId] ?? true,
            minzoom: layer.minzoom,
            maxzoom: layer.maxzoom,
            description: layer.description,
            mapLayers: emptyMapLayers(),
          };
        })
        .sort((a, b) => a.id.localeCompare(b.id))
    : [
        {
          id: data.name || basename(path),
          color: 'transparent',
          visible: true,
          mapLayers: emptyMapLayers(),
        },
      ];

  return {
    id,
    path,
    file: basename(path),
    name: data.name || basename(path),
    vector,
    encoding: tileEncoding(data),
    format: data.format,
    tiles: data.tiles ?? [],
    bounds: numbers(data.bounds),
    center: numbers(data.center),
    minzoom: number(data.minzoom),
    maxzoom: number(data.maxzoom),
    attribution: data.attribution,
    description: data.description,
    visible: prefs.visible ?? true,
    opacity: prefs.opacity ?? 1,
    layers,
    sourceType,
    layerType,
    demEncoding: demEncoding ?? detected ?? 'mapbox',
    tileSize: Number(data.tileSize ?? data.tile_size) || undefined,
    terrain: prefs.terrain ?? true,
    terrainExaggeration: prefs.terrainExaggeration ?? 1,
    data,
  };
}

function emptyMapLayers(): Record<GeomKind, string[]> {
  return { polygons: [], lines: [], points: [], rasters: [] };
}

/* -------------------------------------------------------------------------- */
/* adding to / removing from a map                                            */
/* -------------------------------------------------------------------------- */

/** Base opacity per layer role, before the source's own opacity scales it. */
const BASE_OPACITY = {
  fill: 0.1,
  outline: 0.75,
  line: 0.75,
  circle: 0.75,
  raster: 1,
};

/** Tiles are read at their own resolution, so smoothing them only makes data blurrier than it is. */
const RASTER_PAINT = {
  'raster-resampling': 'linear',
  'raster-fade-duration': 0,
};

/** The maplibre source spec of a raster or raster-dem file. */
function demSpec(source: SourceEntry, maxzoom: number | undefined) {
  const bounds = boundsOf(source);
  return {
    type: (source.sourceType as any) || 'raster',
    tiles: source.tiles,
    minzoom: source.minzoom,
    maxzoom,
    // never ask for tiles the file cannot have: for elevation a missing tile
    // reads as a hole in the ground, not as nothing
    ...(bounds ? { bounds } : {}),
    ...(source.tileSize ? { tileSize: source.tileSize } : {}),
    ...(isDemSource(source) ? { encoding: source.demEncoding } : {}),
  } as any;
}

/** maplibre's default `hillshade-exaggeration`. */
const HILLSHADE_DEFAULT = 0.5;

export function addSourceToMap(map: MapLibreMap, source: SourceEntry, jsonUrl: string) {
  if (source.vector) {
    map.addSource(source.id, {
      type: 'vector',
      url: jsonUrl,
      encoding: source.encoding,
    } as any);

    source.layers.forEach((layer) => {
      const prefix = layerIdPrefix(source.id, layer.id);
      layer.mapLayers = emptyMapLayers();

      const polygonId = `${prefix}-polygons`;
      map.addLayer({
        id: polygonId,
        type: 'fill',
        source: source.id,
        'source-layer': layer.id,
        filter: ['==', '$type', 'Polygon'],
        layout: {},
        paint: { 'fill-opacity': BASE_OPACITY.fill, 'fill-color': layer.color },
      });
      const outlineId = `${polygonId}-outline`;
      map.addLayer({
        id: outlineId,
        type: 'line',
        source: source.id,
        'source-layer': layer.id,
        filter: ['==', '$type', 'Polygon'],
        layout: { 'line-join': 'round', 'line-cap': 'round' },
        paint: {
          'line-color': layer.color,
          'line-width': 1,
          'line-opacity': BASE_OPACITY.outline,
        },
      });
      layer.mapLayers.polygons.push(polygonId, outlineId);

      const lineId = `${prefix}-lines`;
      map.addLayer({
        id: lineId,
        type: 'line',
        source: source.id,
        'source-layer': layer.id,
        filter: ['==', '$type', 'LineString'],
        layout: { 'line-join': 'round', 'line-cap': 'round' },
        paint: { 'line-color': layer.color, 'line-width': 1, 'line-opacity': BASE_OPACITY.line },
      });
      layer.mapLayers.lines.push(lineId);

      const pointId = `${prefix}-points`;
      map.addLayer({
        id: pointId,
        type: 'circle',
        source: source.id,
        'source-layer': layer.id,
        filter: ['==', '$type', 'Point'],
        paint: {
          'circle-color': layer.color,
          'circle-radius': 2.5,
          'circle-opacity': BASE_OPACITY.circle,
        },
      });
      layer.mapLayers.points.push(pointId);
    });
  } else {
    const dem = isDemSource(source);
    const spec = demSpec(source, source.maxzoom);
    map.addSource(source.id, { ...spec, attribution: source.attribution || '' });
    // no attribution here: the twin reads the same file and the credit is already shown
    if (dem) {
      const twin = terrainSourceId(source.id, source.maxzoom ?? 22);
      map.addSource(twin, spec);
      const all = terrainOptionsByMap.get(map) ?? {};
      all[source.id] = { source: twin, exaggeration: source.terrainExaggeration };
      terrainOptionsByMap.set(map, all);
    }
    const layerId = rasterLayerId(source.id);
    const type = (source.layerType as any) || 'raster';
    map.addLayer({
      id: layerId,
      type,
      source: source.id,
      minzoom: 0,
      maxzoom: 24,
      ...(type === 'raster' ? { paint: RASTER_PAINT } : {}),
    } as any);
    source.layers[0].mapLayers = { ...emptyMapLayers(), rasters: [layerId] };
  }
}

export function removeSourceFromMap(map: MapLibreMap, source: SourceEntry) {
  if (!map || !map.style) return;
  // terrain holds on to its source: let go of it first
  if (isDemSource(source)) {
    const options = terrainOptions(map, source);
    if (options) {
      if (map.getTerrain()?.source === options.source) {
        map.setTerrain(null);
        applySky(map);
      }
      if (map.getSource(options.source)) map.removeSource(options.source);
      delete terrainOptionsByMap.get(map)![source.id];
    }
  }
  mapLayerIds(source).forEach((id) => {
    if (map.getLayer(id)) map.removeLayer(id);
  });
  if (map.getSource(source.id)) map.removeSource(source.id);
}

/** Every maplibre layer id belonging to a source, in painter order. */
export function mapLayerIds(source: SourceEntry): string[] {
  const ids: string[] = [];
  source.layers.forEach((layer) => {
    ids.push(
      ...layer.mapLayers.rasters,
      ...layer.mapLayers.polygons,
      ...layer.mapLayers.lines,
      ...layer.mapLayers.points,
    );
  });
  return ids;
}

/* -------------------------------------------------------------------------- */
/* visibility, opacity, order                                                 */
/* -------------------------------------------------------------------------- */

function kindAllowed(kind: GeomKind, filter: GeometryFilter) {
  if (kind === 'rasters') return true;
  return filter === 'all' || filter === kind;
}

/**
 * Recomputes visibility for one source from scratch.
 *
 * Deliberately derived rather than toggled: the source flag, the per-layer
 * flag and the geometry filter are three independent inputs, and anything that
 * tries to remember their combination drifts out of step the moment one of
 * them changes behind its back.
 */
export function applyVisibility(
  map: MapLibreMap,
  source: SourceEntry,
  filter: GeometryFilter = 'all',
) {
  if (!map || !map.style) return;
  source.layers.forEach((layer) => {
    (Object.keys(layer.mapLayers) as GeomKind[]).forEach((kind) => {
      const on = source.visible && layer.visible && kindAllowed(kind, filter);
      layer.mapLayers[kind].forEach((id) => {
        if (map.getLayer(id)) {
          map.setLayoutProperty(id, 'visibility', on ? 'visible' : 'none');
        }
      });
    });
  });
}

export function applyOpacity(map: MapLibreMap, source: SourceEntry) {
  if (!map || !map.style) return;
  const factor = source.opacity;
  source.layers.forEach((layer) => {
    layer.mapLayers.polygons.forEach((id) => {
      if (!map.getLayer(id)) return;
      if (id.endsWith('-outline')) {
        map.setPaintProperty(id, 'line-opacity', BASE_OPACITY.outline * factor);
      } else {
        map.setPaintProperty(id, 'fill-opacity', BASE_OPACITY.fill * factor);
      }
    });
    layer.mapLayers.lines.forEach((id) => {
      if (map.getLayer(id)) map.setPaintProperty(id, 'line-opacity', BASE_OPACITY.line * factor);
    });
    layer.mapLayers.points.forEach((id) => {
      if (map.getLayer(id))
        map.setPaintProperty(id, 'circle-opacity', BASE_OPACITY.circle * factor);
    });
    layer.mapLayers.rasters.forEach((id) => {
      if (!map.getLayer(id)) return;
      const type = (map.getLayer(id) as any).type;
      // hillshade has no opacity property of that name
      if (type === 'raster') map.setPaintProperty(id, 'raster-opacity', factor);
      // maplibre's own default exaggeration is 0.5, which is what an opacity of 100% means
      else if (type === 'hillshade')
        map.setPaintProperty(id, 'hillshade-exaggeration', HILLSHADE_DEFAULT * factor);
    });
  });
}

/**
 * Restacks the map so the panel reads top-to-bottom like every other layer
 * list: `sources[0]` draws above `sources[1]`.
 *
 * Walking bottom-up and moving each layer to the very top leaves the basemap
 * underneath without having to know a single one of its layer names.
 * The one exception is elevation files, which always sit below everything else.
 */
export function applyOrder(map: MapLibreMap, sources: SourceEntry[]) {
  if (!map || !map.style) return;
  // Elevation layers (hillshade, terrain-RGB) are a surface, not data: they go first so every
  // other file, which is moved above them, draws its lines and points on top instead of under a shade.
  const groups = [sources.filter(isDemSource), sources.filter((source) => !isDemSource(source))];
  groups.forEach((group) => {
    for (let index = group.length - 1; index >= 0; index--) {
      mapLayerIds(group[index]).forEach((id) => {
        if (map.getLayer(id)) map.moveLayer(id);
      });
    }
  });
}

/** Show or hide every layer that is not ours — i.e. the basemap. */
export function applyBackground(map: MapLibreMap, visible: boolean) {
  if (!map || !map.style || !map.style._layers) return;
  Object.keys(map.style._layers)
    .filter((id) => !isOwnLayer(id))
    .forEach((id) => {
      if (map.getLayer(id)) {
        map.setLayoutProperty(id, 'visibility', visible ? 'visible' : 'none');
      }
    });
}

/* -------------------------------------------------------------------------- */
/* misc                                                                       */
/* -------------------------------------------------------------------------- */

/** The bounds a source covers, clamped to something maplibre will accept. */
export function boundsOf(source: SourceEntry): [number, number, number, number] | null {
  const bounds = source.bounds;
  if (!bounds || bounds.length < 4) return null;
  const [west, south, east, north] = bounds;
  if ([west, south, east, north].some((n) => typeof n !== 'number' || Number.isNaN(n))) return null;
  return [Math.max(-180, west), Math.max(-85, south), Math.min(180, east), Math.min(85, north)];
}
