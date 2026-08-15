import type { Map as MapLibreMap } from 'maplibre-gl';
import { randomColor } from 'randomcolor';
import { readJSON, writeJSON } from './persisted';
import type { GeometryFilter } from './settings';

export type MapKey = 'main' | 'secondary';
export type GeomKind = 'polygons' | 'lines' | 'points' | 'rasters';

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
  return `${sourceId}-layer`;
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
/* per-file preferences                                                       */
/* -------------------------------------------------------------------------- */

interface StoredPrefs {
  visible?: boolean;
  opacity?: number;
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
  { path, sourceType, layerType }: { path: string; sourceType?: string; layerType?: string },
  data: any,
  dark: boolean,
): SourceEntry {
  const prefs = loadPrefs(path);
  const vector = isVectorSource(data);
  const id = data.id;

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
    bounds: data.bounds,
    center: data.center,
    minzoom: data.minzoom,
    maxzoom: data.maxzoom,
    attribution: data.attribution,
    description: data.description,
    visible: prefs.visible ?? true,
    opacity: prefs.opacity ?? 1,
    layers,
    sourceType,
    layerType,
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
    map.addSource(source.id, {
      type: (source.sourceType as any) || 'raster',
      tiles: source.tiles,
      minzoom: source.minzoom,
      maxzoom: source.maxzoom,
      attribution: source.attribution || '',
    } as any);
    const layerId = rasterLayerId(source.id);
    map.addLayer({
      id: layerId,
      type: (source.layerType as any) || 'raster',
      source: source.id,
      minzoom: 0,
      maxzoom: 24,
    } as any);
    source.layers[0].mapLayers = { ...emptyMapLayers(), rasters: [layerId] };
  }
}

export function removeSourceFromMap(map: MapLibreMap, source: SourceEntry) {
  if (!map || !map.style) return;
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
      else if (type === 'hillshade') map.setPaintProperty(id, 'hillshade-exaggeration', factor);
    });
  });
}

/**
 * Restacks the map so the panel reads top-to-bottom like every other layer
 * list: `sources[0]` draws above `sources[1]`.
 *
 * Walking bottom-up and moving each layer to the very top leaves the basemap
 * underneath without having to know a single one of its layer names.
 */
export function applyOrder(map: MapLibreMap, sources: SourceEntry[]) {
  if (!map || !map.style) return;
  for (let index = sources.length - 1; index >= 0; index--) {
    mapLayerIds(sources[index]).forEach((id) => {
      if (map.getLayer(id)) map.moveLayer(id);
    });
  }
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
