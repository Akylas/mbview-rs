<script lang="ts">
  import { pointToTile } from '@mapbox/tilebelt';
  import { VectorTile } from '@mapbox/vector-tile';
  import Compare from '@maplibre/maplibre-gl-compare';
  import '@maplibre/maplibre-gl-compare/dist/maplibre-gl-compare.css';
  import { decodeTile as decodeMLTTile } from '@maplibre/mlt';
  import { invoke } from '@tauri-apps/api/core';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { dirname, resolve, resourceDir } from '@tauri-apps/api/path';
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';
  import { open } from '@tauri-apps/plugin-dialog';
  import { readTextFile } from '@tauri-apps/plugin-fs';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import Copy from 'carbon-icons-svelte/lib/Copy.svelte';
  import CopyFile from 'carbon-icons-svelte/lib/CopyFile.svelte';
  import EarthFilled from 'carbon-icons-svelte/lib/EarthFilled.svelte';
  import Folder from 'carbon-icons-svelte/lib/Folder.svelte';
  import Layers from 'carbon-icons-svelte/lib/Layers.svelte';
  import Renew from 'carbon-icons-svelte/lib/Renew.svelte';
  import Settings from 'carbon-icons-svelte/lib/Settings.svelte';
  import SplitScreen from 'carbon-icons-svelte/lib/SplitScreen.svelte';
  import Table from 'carbon-icons-svelte/lib/Table.svelte';
  import View from 'carbon-icons-svelte/lib/View.svelte';
  import { Map, NavigationControl, ScaleControl } from 'maplibre-gl';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import Pbf from 'pbf';
  import { onDestroy, onMount, tick } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { compact } from '../lib/layout';
  import { readJSON, writeJSON } from '../lib/persisted';
  import { rememberRecent, resolvedTheme, settings } from '../lib/settings';
  import {
    addSourceToMap,
    applyBackground,
    applyOpacity,
    applyOrder,
    applyVisibility,
    boundsOf,
    makeSource,
    removeSourceFromMap,
    type MapKey,
    type SourceEntry,
  } from '../lib/sources';
  import ContextMenu from './ContextMenu.svelte';
  import FeatureTable from './FeatureTable.svelte';
  import FileDrop from './FileDrop.svelte';
  import type { Feature } from './Map';
  import MapPopup from './MapPopup.svelte';
  import SettingsSheet from './SettingsSheet.svelte';
  import SourceInfo from './SourceInfo.svelte';
  import SourcePanel from './SourcePanel.svelte';
  import StatusBar from './StatusBar.svelte';
  import IconButton from './ui/IconButton.svelte';
  import MenuItem from './ui/MenuItem.svelte';
  import Resizer from './ui/Resizer.svelte';
  import SegmentedControl from './ui/SegmentedControl.svelte';
  import Sheet from './ui/Sheet.svelte';

  /* ---------------------------------------------------------------------- */
  /* state                                                                  */
  /* ---------------------------------------------------------------------- */

  let mainMap: Map = null;
  let secondaryMap: Map = null;
  let compareMap: Compare = null;

  let mainSources: SourceEntry[] = [];
  let secondarySources: SourceEntry[] = [];

  let mainFeatures: Feature[] = [];
  let secondaryFeatures: Feature[] = [];

  let mainMapDiv: HTMLElement;
  let secondaryMapDiv: HTMLElement;

  /** which map the panel and dropped files are pointed at */
  let activeKey: MapKey = 'main';
  let panelSheetOpen = false;
  let settingsOpen = false;
  let infoSource: SourceEntry | null = null;
  let restoring = false;

  let savedZoom = readJSON<number | null>('mbview.zoom', null);
  let savedCenter = readJSON<{ lat: number; lng: number } | null>('mbview.center', null);
  let savedSlider = readJSON<number | null>('mbview.slider', null);

  let unlisteners: UnlistenFn[] = [];

  $: hasSources = mainSources.length > 0 || secondarySources.length > 0;
  $: activeSources = activeKey === 'secondary' ? secondarySources : mainSources;
  $: activeMap = activeKey === 'secondary' ? secondaryMap : mainMap;
  $: tableFeatures = [...(mainFeatures ?? []), ...(secondaryFeatures ?? [])];

  /* ---------------------------------------------------------------------- */
  /* lifecycle                                                              */
  /* ---------------------------------------------------------------------- */

  onMount(async () => {
    // wrapped so `vite dev` in a plain browser still renders the shell, which
    // is the only practical way to work on the chrome without a full rebuild
    try {
      await listenToShell();
    } catch (error) {
      console.warn('running outside the app shell', error);
      return;
    }
    await restoreSession();
  });

  async function listenToShell() {
    unlisteners.push(
      await listen<MBTilesPayload>('mbtiles', (event) => onMBTilesSet(event.payload)),
      await listen<{ message: string }>('menu', (event) => {
        switch (event.payload.message) {
          case 'open':
            addMBTiles({ key: activeKey });
            break;
          case 'learn_more':
            openUrl(REPO_URL);
            break;
        }
      }),
      // the backend watches the files and asks for a repaint when one changes
      await listen<{ message: string }>('reload-mbtiles', () => {
        [mainMap, secondaryMap].forEach(refreshTiles);
      })
    );
  }

  onDestroy(() => {
    unlisteners.forEach((stop) => stop?.());
    clearMaps();
  });

  interface MBTilesPayload {
    path: string;
    json_url: string;
    key: MapKey;
    source_id: string;
    source_type?: string;
    layer_type?: string;
  }

  /** Everything currently open, so the next launch comes back the same. */
  function persistSession() {
    if (restoring) return;
    writeJSON('mbview.open', {
      main: mainSources.map((source) => source.path),
      secondary: secondarySources.map((source) => source.path),
    });
  }

  async function restoreSession() {
    const stored = readJSON<{ main: string[]; secondary: string[] }>('mbview.open', {
      main: [],
      secondary: [],
    });
    // migration from the single-file keys this app used to write
    const legacyMain = localStorage.getItem('currentMBtiles');
    const main = stored.main?.length ? stored.main : legacyMain ? [legacyMain] : [];
    const secondary = stored.secondary ?? [];
    if (!main.length && !secondary.length) return;

    restoring = true;
    try {
      // Sequential, and bottom-up: each source is added on top of the stack, so
      // replaying the saved top-first list in reverse rebuilds the same order.
      // Concurrent setups would race, since the backend answers on an event.
      for (const path of main.slice().reverse()) await setupMBtiles({ path, key: 'main' });
      for (const path of secondary.slice().reverse())
        await setupMBtiles({ path, key: 'secondary' });
    } finally {
      restoring = false;
      persistSession();
    }
  }

  /* ---------------------------------------------------------------------- */
  /* opening files                                                          */
  /* ---------------------------------------------------------------------- */

  /**
   * Resolves once the source that `setup_mbtiles` announces has been added.
   * A plain record rather than a `Map`, whose name maplibre has taken here.
   */
  const pending: Record<string, () => void> = {};

  async function setupMBtiles({
    path,
    key = 'main',
    sourceType,
    layerType,
  }: {
    path: string;
    key?: MapKey;
    sourceType?: string;
    layerType?: string;
  }) {
    const sources = key === 'secondary' ? secondarySources : mainSources;
    if (sources.some((source) => source.path === path)) return;

    const token = `${key}:${path}`;
    const done = new Promise<void>((resolveDone) => {
      pending[token] = resolveDone;
      // never leave the restore loop hanging on a file the backend rejected
      setTimeout(() => {
        if (pending[token]) {
          delete pending[token];
          resolveDone();
        }
      }, 10000);
    });

    try {
      await invoke('setup_mbtiles', { key, path, sourceType, layerType });
    } catch (error) {
      console.error('setup_mbtiles failed', path, error);
      delete pending[token];
      return;
    }
    await done;
  }

  let lastFolder: string = localStorage.getItem('lastOpenFolder');

  async function addMBTiles({
    key = 'main',
    sourceType,
    layerType,
    path,
  }: {
    key?: MapKey;
    sourceType?: string;
    layerType?: string;
    path?: string;
  }) {
    try {
      const chosen =
        path ??
        (await open({ multiple: false, directory: false, defaultPath: lastFolder }));
      if (!chosen || typeof chosen !== 'string') return;
      await setupMBtiles({ path: chosen, key, sourceType, layerType });
      rememberRecent(chosen);
      lastFolder = await dirname(chosen);
      localStorage.setItem('lastOpenFolder', lastFolder);
    } catch (error) {
      console.error(error);
    }
  }

  async function onMBTilesSet(payload: MBTilesPayload) {
    const { path, json_url, key, source_type, layer_type } = payload;
    const token = `${key}:${path}`;
    const resolveDone = pending[token];
    try {
      if (!path || !json_url) return;
      const sources = key === 'secondary' ? secondarySources : mainSources;
      if (sources.some((source) => source.path === path)) return;

      const data = await (await fetch(json_url)).json();
      const source = makeSource(
        { path, sourceType: source_type, layerType: layer_type },
        data,
        $resolvedTheme === 'dark'
      );

      if (key === 'secondary') {
        if (!mainMap) return; // B only exists next to A
        if (!secondaryMap) {
          secondaryMap = await createMap('secondary', source, json_url);
          await startCompare();
        }
        await attachSource(secondaryMap, source, json_url);
        secondarySources = [source, ...secondarySources];
      } else {
        const first = !mainMap;
        if (first) {
          mainMap = await createMap('main', source, json_url);
          watchCamera(mainMap);
        }
        await attachSource(mainMap, source, json_url);
        mainSources = [source, ...mainSources];
        if (first && !savedCenter) fitTo(source);
      }
      persistSession();
    } catch (error) {
      console.error('onMBTilesSet failed', payload, error);
    } finally {
      if (resolveDone) {
        delete pending[token];
        resolveDone();
      }
    }
  }

  function whenReady(map: Map): Promise<void> {
    return map.isStyleLoaded() ? Promise.resolve() : new Promise((done) => map.once('load', () => done()));
  }

  async function attachSource(map: Map, source: SourceEntry, jsonUrl: string) {
    await whenReady(map);
    if (map.getSource(source.id)) return;
    addSourceToMap(map, source, jsonUrl);
    applyVisibility(map, source, $settings.geometryFilter);
    applyOpacity(map, source);
    // a source added later belongs on top, which is where the panel shows it
    applyOrder(map, [source, ...(map === secondaryMap ? secondarySources : mainSources)]);
  }

  /* ---------------------------------------------------------------------- */
  /* maps                                                                   */
  /* ---------------------------------------------------------------------- */

  function backdropColor(theme: string) {
    return theme === 'dark' ? '#141416' : '#f7f7f9';
  }

  function blankStyle() {
    return {
      version: 8 as const,
      sources: {},
      layers: [
        {
          // named as one of ours so the "show basemap" toggle leaves it alone:
          // with no basemap this is the backdrop, not something to hide
          id: '___backdrop',
          type: 'background' as const,
          paint: { 'background-color': backdropColor($resolvedTheme) },
        },
      ],
    };
  }

  /**
   * The preview basemaps double as the thing being previewed: `basic` and
   * `terrain` point their vector source at whatever file is open. That only
   * works for a vector file, so a raster one falls back to a plain backdrop
   * rather than a style whose every layer would fail to load.
   */
  async function buildStyle(source: SourceEntry, jsonUrl: string) {
    if ($settings.basemap === 'none') return blankStyle();
    try {
      const path = await resolve(await resourceDir(), `_up_/resources/styles/${$settings.basemap}.json`);
      const raw = await readTextFile(path);
      const needsOurTiles = raw.includes('{{json_url}}') || raw.includes('mbtiles://');
      if (needsOurTiles && !source.vector) return blankStyle();

      const style = JSON.parse(raw.split('{{json_url}}').join(jsonUrl));
      Object.values(style.sources ?? {}).forEach((entry: any) => {
        if (entry?.type === 'vector' && (entry.url === jsonUrl || String(entry.url).startsWith('mbtiles://'))) {
          entry.url = jsonUrl;
          entry.encoding = source.encoding;
        }
      });
      (style.layers ?? []).forEach((layer: any) => {
        layer.layout = layer.layout || {};
        layer.layout.visibility = $settings.showBackground ? 'visible' : 'none';
      });
      return style;
    } catch (error) {
      console.error('basemap style unavailable, falling back to a blank one', error);
      return blankStyle();
    }
  }

  async function createMap(key: MapKey, source: SourceEntry, jsonUrl: string) {
    const style = await buildStyle(source, jsonUrl);
    const center =
      key === 'main'
        ? savedCenter ?? centerOf(source)
        : mainMap.getCenter();
    const zoom =
      key === 'main'
        ? savedZoom ?? (source.minzoom ?? 0) + (((source.maxzoom ?? 14) - (source.minzoom ?? 0)) / 2)
        : mainMap.getZoom();

    const map = new Map({
      container: key,
      style: style as any,
      center: center as any,
      zoom,
      interactive: true,
      attributionControl: { compact: true },
    });
    map.showTileBoundaries = $settings.showTileBoundaries;
    map.showCollisionBoxes = $settings.showCollisionBoxes;
    // maplibre's own controls, so they inherit the same stylesheet the rest of
    // the map chrome does instead of arriving unstyled from a mapbox plugin
    map.addControl(
      new NavigationControl({ showCompass: true, showZoom: true, visualizePitch: true }),
      'top-right'
    );
    map.addControl(new ScaleControl({ maxWidth: 100, unit: 'metric' }), 'top-right');
    await whenReady(map);
    return map;
  }

  function centerOf(source: SourceEntry) {
    if (source.center?.length >= 2) return [source.center[0], source.center[1]];
    const bounds = boundsOf(source);
    if (!bounds) return [0, 0];
    return [(bounds[0] + bounds[2]) / 2, (bounds[1] + bounds[3]) / 2];
  }

  function watchCamera(map: Map) {
    map.on('moveend', () => {
      savedZoom = map.getZoom();
      savedCenter = map.getCenter();
      writeJSON('mbview.zoom', savedZoom);
      writeJSON('mbview.center', { lat: savedCenter.lat, lng: savedCenter.lng });
    });
  }

  function refreshTiles(map: Map) {
    if (!map?.style) return;
    // `sourceCaches` was renamed to `tileManagers` in maplibre-gl 5
    const caches = (map.style as any).tileManagers ?? (map.style as any).sourceCaches;
    Object.keys(caches ?? {}).forEach((id) => {
      caches[id].clearTiles();
      caches[id].update((map as any).transform);
    });
    map.triggerRepaint();
  }

  async function startCompare() {
    if (!mainMap || !secondaryMap) return;
    if (compareMap) compareMap.remove();
    await tick();
    compareMap = new Compare(mainMap, secondaryMap, '#comparison-container', {});
    compareMap.setSlider(savedSlider ?? (mainMapDiv?.clientWidth || 800) / 2);
    guardSwiperDrag();
  }

  /**
   * The compare plugin binds its own `mousedown` and never calls
   * preventDefault, so a drag also starts a text selection and smears it
   * across every popup and control the pointer crosses. Ours runs alongside
   * — preventDefault does not stop the plugin's handler from firing — and
   * suppresses selection for the length of the drag only, so popup text is
   * still selectable the rest of the time.
   */
  function guardSwiperDrag() {
    const swiper = document.querySelector(
      '#comparison-container .compare-swiper-vertical, #comparison-container .compare-swiper-horizontal'
    );
    if (!swiper) return;

    const stop = () => {
      document.body.classList.remove('swiping');
      document.removeEventListener('mouseup', stop);
      document.removeEventListener('touchend', stop);
      document.removeEventListener('touchcancel', stop);
    };

    swiper.addEventListener('mousedown', (event) => {
      event.preventDefault();
      document.body.classList.add('swiping');
      document.addEventListener('mouseup', stop);
    });
    // touch never starts a selection on its own, but a long drag can still
    // raise the selection handles on mobile webviews
    swiper.addEventListener('touchstart', () => {
      document.body.classList.add('swiping');
      document.addEventListener('touchend', stop);
      document.addEventListener('touchcancel', stop);
    });
  }

  function clearMaps() {
    // the swiper is about to be torn out from under a drag that may be live
    document.body.classList.remove('swiping');
    try {
      compareMap?.remove();
    } catch (error) {
      /* already gone */
    }
    compareMap = null;
    try {
      secondaryMap?.remove();
    } catch (error) {
      /* already gone */
    }
    secondaryMap = null;
    try {
      mainMap?.remove();
    } catch (error) {
      /* already gone */
    }
    mainMap = null;
    mainFeatures = [];
    secondaryFeatures = [];
  }

  function closeCompare() {
    document.body.classList.remove('swiping');
    if (compareMap) {
      savedSlider = (compareMap as any).currentPosition ?? savedSlider;
      writeJSON('mbview.slider', savedSlider);
      compareMap.remove();
      compareMap = null;
    }
    secondaryMap?.remove();
    secondaryMap = null;
    secondarySources = [];
    secondaryFeatures = [];
    if (activeKey === 'secondary') activeKey = 'main';
    persistSession();
  }

  function toggleCompare() {
    if (secondaryMap) {
      closeCompare();
    } else if (mainMap) {
      addMBTiles({ key: 'secondary' });
    }
  }

  /* ---------------------------------------------------------------------- */
  /* source operations                                                      */
  /* ---------------------------------------------------------------------- */

  function mapFor(key: MapKey) {
    return key === 'secondary' ? secondaryMap : mainMap;
  }

  function removeSource(key: MapKey, source: SourceEntry) {
    const map = mapFor(key);
    if (map) removeSourceFromMap(map, source);
    if (key === 'secondary') {
      secondarySources = secondarySources.filter((entry) => entry.id !== source.id);
      if (secondarySources.length === 0) closeCompare();
    } else {
      mainSources = mainSources.filter((entry) => entry.id !== source.id);
      if (mainSources.length === 0) {
        // nothing left to compare against either
        closeCompare();
        mainMap?.remove();
        mainMap = null;
        mainFeatures = [];
      }
    }
    persistSession();
  }

  async function reloadSource(key: MapKey, source: SourceEntry) {
    const map = mapFor(key);
    if (!map) return;
    try {
      await invoke('reload_mbtiles', { path: source.path });
    } catch (error) {
      console.error(error);
    }
    refreshTiles(map);
  }

  /** Re-open every file from scratch, keeping order, camera and split. */
  async function reloadAll() {
    const mainPaths = mainSources.map((source) => source.path);
    const secondaryPaths = secondarySources.map((source) => source.path);
    if (compareMap) savedSlider = (compareMap as any).currentPosition ?? savedSlider;
    if (mainMap) {
      savedZoom = mainMap.getZoom();
      savedCenter = mainMap.getCenter();
    }
    restoring = true;
    clearMaps();
    mainSources = [];
    secondarySources = [];
    await tick();
    try {
      for (const path of mainPaths.slice().reverse()) await setupMBtiles({ path, key: 'main' });
      for (const path of secondaryPaths.slice().reverse())
        await setupMBtiles({ path, key: 'secondary' });
    } finally {
      restoring = false;
      persistSession();
    }
  }

  function fitTo(source: SourceEntry) {
    const bounds = boundsOf(source);
    const map = mapFor(activeKey) ?? mainMap;
    if (!map) return;
    if (bounds) {
      map.fitBounds(bounds as any, { padding: 40, duration: 400 });
    } else if (source.center?.length >= 2) {
      map.flyTo({ center: [source.center[0], source.center[1]], zoom: source.center[2] ?? 10 });
    }
  }

  function onReorder(key: MapKey) {
    const map = mapFor(key);
    if (map) applyOrder(map, key === 'secondary' ? secondarySources : mainSources);
    persistSession();
  }

  function copy(text: string) {
    writeText(text).catch((error) => console.error(error));
  }

  /* ---------------------------------------------------------------------- */
  /* settings that reach into the maps                                      */
  /* ---------------------------------------------------------------------- */

  $: [mainMap, secondaryMap].forEach((map) => {
    if (map) map.showTileBoundaries = $settings.showTileBoundaries;
  });
  $: [mainMap, secondaryMap].forEach((map) => {
    if (map) map.showCollisionBoxes = $settings.showCollisionBoxes;
  });
  $: {
    const on = $settings.showBackground;
    [mainMap, secondaryMap].forEach((map) => map && applyBackground(map, on));
  }
  $: {
    // with no basemap the backdrop is ours to keep in step with the theme,
    // and repainting it beats rebuilding the whole style for a colour
    const color = backdropColor($resolvedTheme);
    [mainMap, secondaryMap].forEach((map) => {
      if (map?.getLayer('___backdrop')) {
        map.setPaintProperty('___backdrop', 'background-color', color);
      }
    });
  }
  $: {
    const filter = $settings.geometryFilter;
    if (mainMap) mainSources.forEach((source) => applyVisibility(mainMap, source, filter));
    if (secondaryMap) secondarySources.forEach((source) => applyVisibility(secondaryMap, source, filter));
  }

  async function changeBasemap(name: string) {
    if (name === $settings.basemap) return;
    settings.update((current) => ({ ...current, basemap: name }));
    await reloadAll();
  }

  /* ---------------------------------------------------------------------- */
  /* tile dump                                                              */
  /* ---------------------------------------------------------------------- */

  function dumpGeometry(rings: any[]) {
    return rings.map((ring: any[]) =>
      ring.reduce((acc: number[], { x, y }: any) => {
        acc.push(x, y);
        return acc;
      }, [])
    );
  }

  function dumpMVT(buffer: ArrayBuffer) {
    const tile = new VectorTile(new Pbf(buffer));
    return {
      layers: Object.values(tile.layers).map((vl: any) => {
        const { version, name, extent, length } = vl;
        const features = [];
        for (let index = 0; index < length; index++) {
          const feature = vl.feature(index);
          features.push({
            type: feature.type,
            extent: feature.extent,
            id: feature.id,
            properties: feature.properties,
            geometry: dumpGeometry(feature.loadGeometry()),
          });
        }
        return { version, name, extent, features };
      }),
    };
  }

  // MLT has no `VectorTile` equivalent: the decoder hands back one FeatureTable
  // per layer, holding columnar vectors.
  function dumpMLT(buffer: ArrayBuffer) {
    return {
      layers: decodeMLTTile(new Uint8Array(buffer)).map((table: any) => ({
        name: table.name,
        extent: table.extent,
        features: table.getFeatures().map((feature: any) => ({
          type: feature.geometry.type,
          extent: table.extent,
          // ids beyond 2^53 decode as BigInt, which JSON.stringify refuses
          id: typeof feature.id === 'bigint' ? feature.id.toString() : feature.id,
          properties: feature.properties,
          geometry: dumpGeometry(feature.geometry.coordinates),
        })),
      })),
    };
  }

  function tileAt(key: MapKey, point: { x: number; y: number }) {
    const map = mapFor(key);
    if (!map) return null;
    const lngLat = map.unproject([point.x, point.y]);
    return { lngLat, tile: pointToTile(lngLat.lng, lngLat.lat, Math.floor(map.getZoom())) };
  }

  async function copyTileAsGeoJSON(key: MapKey, point: { x: number; y: number }) {
    const at = tileAt(key, point);
    if (!at) return;
    const sources = key === 'secondary' ? secondarySources : mainSources;
    const result: Record<string, unknown> = {};
    for (const source of sources) {
      if (!source.tiles?.length) continue;
      try {
        const url = source.tiles[0]
          .replace('{x}', String(at.tile[0]))
          .replace('{y}', String(at.tile[1]))
          .replace('{z}', String(at.tile[2]));
        const buffer = await (await fetch(url)).arrayBuffer();
        result[source.file] = source.encoding === 'mlt' ? dumpMLT(buffer) : dumpMVT(buffer);
      } catch (error) {
        console.error('tile dump failed', source.path, error);
      }
    }
    const keys = Object.keys(result);
    copy(JSON.stringify(keys.length === 1 ? result[keys[0]] : result, null, 2));
  }

  function copyTileUrl(key: MapKey, point: { x: number; y: number }) {
    const at = tileAt(key, point);
    const sources = key === 'secondary' ? secondarySources : mainSources;
    if (!at || !sources.length || !sources[0].tiles?.length) return;
    copy(
      sources[0].tiles[0]
        .replace('{x}', String(at.tile[0]))
        .replace('{y}', String(at.tile[1]))
        .replace('{z}', String(at.tile[2]))
    );
  }

  function openInOSM(key: MapKey = 'main') {
    const map = mapFor(key) ?? mainMap;
    if (!map) return;
    const center = map.getCenter();
    openUrl(`https://www.openstreetmap.org/#map=${Math.round(map.getZoom()) + 1}/${center.lat}/${center.lng}`);
  }

  /* ---------------------------------------------------------------------- */
  /* layout                                                                 */
  /* ---------------------------------------------------------------------- */

  function resizeMaps() {
    mainMap?.resize();
    secondaryMap?.resize();
  }

  function onPanelResize(event: CustomEvent<number>) {
    settings.update((current) => ({
      ...current,
      panelWidth: Math.max(220, Math.min(560, current.panelWidth + event.detail)),
    }));
    resizeMaps();
  }

  function onTableResize(event: CustomEvent<number>) {
    const delta = (event.detail / window.innerHeight) * 100;
    settings.update((current) => ({
      ...current,
      tableHeight: Math.max(12, Math.min(75, current.tableHeight - delta)),
    }));
    resizeMaps();
  }

  $: {
    // any chrome change moves the map's box; maplibre needs telling
    void $settings.panelOpen;
    void $settings.showTable;
    void $settings.panelWidth;
    void $settings.tableHeight;
    void $compact;
    tick().then(resizeMaps);
  }

  /* ---------------------------------------------------------------------- */
  /* keyboard                                                               */
  /* ---------------------------------------------------------------------- */

  function onKeydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement;
    if (target?.matches?.('input, textarea, select, [contenteditable="true"]')) return;
    if (event.metaKey || event.ctrlKey) {
      if (event.key === 'o') {
        event.preventDefault();
        addMBTiles({ key: activeKey });
      }
      return;
    }
    switch (event.key) {
      case 'b':
        settings.update((s) => ({ ...s, showBackground: !s.showBackground }));
        break;
      case 't':
        settings.update((s) => ({ ...s, showTileBoundaries: !s.showTileBoundaries }));
        break;
      case 'i':
        settings.update((s) => ({
          ...s,
          inspect: s.inspect === 'click' ? 'hover' : s.inspect === 'hover' ? 'off' : 'click',
        }));
        break;
      case 'f':
        settings.update((s) => ({ ...s, showTable: !s.showTable }));
        break;
      case 'l':
        if ($compact) panelSheetOpen = !panelSheetOpen;
        else settings.update((s) => ({ ...s, panelOpen: !s.panelOpen }));
        break;
      case 's':
        toggleCompare();
        break;
    }
  }

  /* the context menu points at whichever map was right-clicked */
  let mainPoint = { x: 0, y: 0 };
  let secondaryPoint = { x: 0, y: 0 };
</script>

<svelte:window on:keydown={onKeydown} on:resize={resizeMaps} />

<div class="app">
  <header class="toolbar">
    <div class="brand">
      <Layers size={16} />
      {#if !$compact}<span>MBTiles</span>{/if}
    </div>

    <IconButton
      icon={Folder}
      label={$_('open_mbtiles')}
      on:click={() => addMBTiles({ key: activeKey })}
    />
    <IconButton
      icon={SplitScreen}
      label={$_('opens_split')}
      active={!!secondaryMap}
      disabled={!mainMap}
      on:click={toggleCompare}
    />
    <IconButton
      icon={Table}
      label={$_('open_bottom_panel')}
      active={$settings.showTable}
      on:click={() => settings.update((s) => ({ ...s, showTable: !s.showTable }))}
    />
    {#if !$compact}
      <IconButton
        icon={View}
        label={$_('show_background_layer')}
        active={$settings.showBackground}
        on:click={() => settings.update((s) => ({ ...s, showBackground: !s.showBackground }))}
      />
    {/if}

    <div class="spacer" />

    <IconButton icon={Renew} label={$_('reload')} disabled={!hasSources} on:click={reloadAll} />
    {#if !$compact}
      <IconButton
        icon={EarthFilled}
        label={$_('open_osm')}
        disabled={!mainMap}
        on:click={() => openInOSM(activeKey)}
      />
    {/if}
    <IconButton icon={Settings} label={$_('settings')} on:click={() => (settingsOpen = true)} />
    {#if $compact}
      <IconButton
        icon={Layers}
        label={$_('layers')}
        active={panelSheetOpen}
        on:click={() => (panelSheetOpen = !panelSheetOpen)}
      />
    {/if}
  </header>

  <div class="body">
    {#if !$compact && $settings.panelOpen}
      <aside class="panel" style:width="{$settings.panelWidth}px">
        {#if secondaryMap}
          <div class="tabs">
            <SegmentedControl
              bind:value={activeKey}
              options={[
                { value: 'main', label: $_('map_a') },
                { value: 'secondary', label: $_('map_b') },
              ]}
            />
          </div>
        {/if}
        <SourcePanel
          sources={activeSources}
          map={activeMap}
          on:add={(event) => addMBTiles({ key: activeKey, ...event.detail })}
          on:remove={(event) => removeSource(activeKey, event.detail)}
          on:zoom={(event) => fitTo(event.detail)}
          on:info={(event) => (infoSource = event.detail)}
          on:copyPath={(event) => copy(event.detail.path)}
          on:reloadSource={(event) => reloadSource(activeKey, event.detail)}
          on:reorder={() => onReorder(activeKey)}
          on:reordered={() => onReorder(activeKey)}
        />
      </aside>
      <Resizer orientation="vertical" on:move={onPanelResize} />
    {/if}

    <div class="stage">
      <div class="maps">
        <FileDrop
          extensions={['mbtiles', 'etiles']}
          handleFiles={(paths) => paths.forEach((path) => addMBTiles({ key: activeKey, path }))}
          let:files
        >
          <div class="dropzone" class:droppable={files.length > 0}>
            {#if files.length > 0}
              <div class="drop-card">
                <p>{$_('import_mbtiles')}</p>
                <strong>{files.map((file) => file.split(/[\\/]/).pop()).join(', ')}</strong>
              </div>
            {/if}
          </div>
        </FileDrop>

        {#if !hasSources}
          <div class="welcome">
            <Layers size={32} />
            <h1>{$_('drop_open_mbtiles')}</h1>
            <button type="button" class="cta" on:click={() => addMBTiles({ key: 'main' })}>
              {$_('open_mbtiles')}
            </button>
            {#if $settings.recent.length}
              <ul class="recent">
                {#each $settings.recent.slice(0, 5) as path (path)}
                  <li>
                    <button type="button" on:click={() => addMBTiles({ key: 'main', path })}>
                      {path.split(/[\\/]/).pop()}
                    </button>
                  </li>
                {/each}
              </ul>
            {/if}
          </div>
        {/if}

        <div id="comparison-container">
          <div id="secondary" class="map" bind:this={secondaryMapDiv}>
            <MapPopup
              map={secondaryMap}
              sources={secondarySources}
              bind:features={secondaryFeatures}
              mode={$settings.inspect}
            />
          </div>
          <div id="main" class="map" bind:this={mainMapDiv}>
            <MapPopup
              map={mainMap}
              sources={mainSources}
              bind:features={mainFeatures}
              mode={$settings.inspect}
            />
          </div>
        </div>

        <StatusBar map={activeMap} onCopy={copy} />
      </div>

      {#if $settings.showTable && !$compact}
        <Resizer orientation="horizontal" on:move={onTableResize} />
        <div class="table" style:height="{$settings.tableHeight}%">
          <FeatureTable
            features={tableFeatures}
            onCopy={copy}
            onClose={() => settings.update((s) => ({ ...s, showTable: false }))}
          />
        </div>
      {/if}
    </div>
  </div>

  {#if !$compact}
    <button
      type="button"
      class="panel-toggle"
      class:closed={!$settings.panelOpen}
      aria-label={$_('layers')}
      title={$_('layers')}
      on:click={() => settings.update((s) => ({ ...s, panelOpen: !s.panelOpen }))}
    >
      <Layers size={16} />
    </button>
  {/if}
</div>

{#if $compact}
  <Sheet
    open={panelSheetOpen}
    title={$_('layers')}
    onClose={() => (panelSheetOpen = false)}
  >
    {#if secondaryMap}
      <div class="tabs">
        <SegmentedControl
          bind:value={activeKey}
          options={[
            { value: 'main', label: $_('map_a') },
            { value: 'secondary', label: $_('map_b') },
          ]}
        />
      </div>
    {/if}
    <div class="sheet-panel">
      <SourcePanel
        sources={activeSources}
        map={activeMap}
        on:add={(event) => addMBTiles({ key: activeKey, ...event.detail })}
        on:remove={(event) => removeSource(activeKey, event.detail)}
        on:zoom={(event) => fitTo(event.detail)}
        on:info={(event) => (infoSource = event.detail)}
        on:copyPath={(event) => copy(event.detail.path)}
        on:reloadSource={(event) => reloadSource(activeKey, event.detail)}
        on:reorder={() => onReorder(activeKey)}
        on:reordered={() => onReorder(activeKey)}
      />
    </div>
  </Sheet>

  {#if $settings.showTable}
    <div class="table-overlay">
      <FeatureTable
        features={tableFeatures}
        onCopy={copy}
        onClose={() => settings.update((s) => ({ ...s, showTable: false }))}
      />
    </div>
  {/if}
{/if}

<ContextMenu target={mainMapDiv} bind:point={mainPoint}>
  <MenuItem
    label={$_('copy_tile_geojson')}
    icon={CopyFile}
    on:click={() => copyTileAsGeoJSON('main', mainPoint)}
  />
  <MenuItem
    label={$_('copy_tile_url')}
    icon={Copy}
    on:click={() => copyTileUrl('main', mainPoint)}
  />
  <MenuItem
    label={$_('copy_coordinates')}
    icon={Copy}
    on:click={() => {
      const at = tileAt('main', mainPoint);
      if (at) copy(`${at.lngLat.lat},${at.lngLat.lng}`);
    }}
  />
  <MenuItem label={$_('open_osm')} icon={EarthFilled} on:click={() => openInOSM('main')} />
</ContextMenu>

<ContextMenu target={secondaryMapDiv} bind:point={secondaryPoint}>
  <MenuItem
    label={$_('copy_tile_geojson')}
    icon={CopyFile}
    on:click={() => copyTileAsGeoJSON('secondary', secondaryPoint)}
  />
  <MenuItem
    label={$_('copy_tile_url')}
    icon={Copy}
    on:click={() => copyTileUrl('secondary', secondaryPoint)}
  />
  <MenuItem label={$_('open_osm')} icon={EarthFilled} on:click={() => openInOSM('secondary')} />
</ContextMenu>

<SettingsSheet
  open={settingsOpen}
  onClose={() => (settingsOpen = false)}
  onBasemap={changeBasemap}
/>

<SourceInfo source={infoSource} onClose={() => (infoSource = null)} />

<style>
  .app {
    display: flex;
    position: absolute;
    inset: 0;
    flex-direction: column;
    overflow: hidden;
  }

  .toolbar {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 2px;
    padding: 3px 6px;
    padding-top: calc(3px + var(--safe-top));
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 8px 0 4px;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
    letter-spacing: 0.02em;
  }
  .spacer {
    flex: 1;
  }

  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .panel {
    display: flex;
    flex: 0 0 auto;
    flex-direction: column;
    min-height: 0;
    background: var(--surface);
  }
  .tabs {
    flex: 0 0 auto;
    padding: 4px 8px 0;
  }

  .stage {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  .maps {
    position: relative;
    flex: 1;
    min-height: 0;
    background: var(--surface-sunken);
  }
  .table {
    flex: 0 0 auto;
    min-height: 0;
    border-top: 1px solid var(--border);
  }
  .table-overlay {
    position: absolute;
    inset: auto 0 0 0;
    z-index: 45;
    height: 60%;
    border-top: 1px solid var(--border);
    background: var(--surface);
    box-shadow: var(--shadow);
  }

  .dropzone {
    position: absolute;
    inset: 0;
    z-index: 30;
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
  }
  .droppable {
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .drop-card {
    padding: 16px 24px;
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--shadow);
    text-align: center;
  }
  .drop-card p {
    margin: 0 0 4px;
    color: var(--text-muted);
    font-size: 12px;
  }

  .welcome {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    color: var(--text-faint);
    text-align: center;
  }
  .welcome h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 500;
  }
  .cta {
    min-height: var(--control-h);
    padding: 0 16px;
    border: none;
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--on-accent);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }
  .recent {
    margin: 4px 0 0;
    padding: 0;
    list-style: none;
  }
  .recent button {
    padding: 3px 8px;
    border: none;
    background: transparent;
    color: var(--accent-text);
    font-size: 12px;
    cursor: pointer;
  }
  .recent button:hover {
    text-decoration: underline;
  }

  .panel-toggle {
    /* top-left: the status bar owns the bottom-left corner */
    position: absolute;
    top: calc(var(--tap) + 16px + var(--safe-top));
    left: 8px;
    z-index: 15;
    display: none;
    align-items: center;
    justify-content: center;
    width: var(--tap);
    height: var(--tap);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-float);
    box-shadow: var(--shadow);
    color: var(--text-muted);
    cursor: pointer;
  }
  .panel-toggle.closed {
    display: flex;
  }

  .sheet-panel {
    height: 100%;
    min-height: 220px;
  }
</style>
