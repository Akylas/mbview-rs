import type { Feature as GeoFeature } from 'geojson';
export interface Layers {
  points: string[];
  lines: string[];
  rasters: string[];
  polygons: string[];
  colors?: Record<string, unknown>;
}

export interface Source {
  layers: Layers;
  [_: string]: any;
}

export interface Feature extends GeoFeature {
  sourceName: string;
  source: string;
  sourceLayer: string;
}
