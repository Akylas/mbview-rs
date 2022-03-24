<script lang="ts">
  import Split from '@geoffcox/svelte-splitter/src/Split.svelte';
  import Compare from '@maplibre/maplibre-gl-compare';
  import '@maplibre/maplibre-gl-compare/dist/maplibre-gl-compare.css';
  import { invoke } from '@tauri-apps/api';
  import { open } from '@tauri-apps/api/dialog';
  import { listen, UnlistenFn } from '@tauri-apps/api/event';
  import { readTextFile } from '@tauri-apps/api/fs';
  import { resolve, resourceDir } from '@tauri-apps/api/path';
  import { open as openURl } from '@tauri-apps/api/shell';
  import {
    Content,
    DataTable,
    Header,
    HeaderAction,
    HeaderGlobalAction,
    HeaderUtilities,
    Select,
    SelectItem,
    SkipToContent,
    Theme,
  } from 'carbon-components-svelte';
  import type { CarbonTheme } from 'carbon-components-svelte/types/Theme/Theme.svelte';
  import SplitScreen16 from 'carbon-icons-svelte/lib/SplitScreen16';
  import OpenPanelBottom16 from 'carbon-icons-svelte/lib/OpenPanelBottom16';
  import { CompassControl, ZoomControl } from 'mapbox-gl-controls';
  import { Map } from 'maplibre-gl';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import { randomColor } from 'randomcolor';
  import { onDestroy, onMount } from 'svelte';
  import { _ } from 'svelte-i18n';
  import FileDrop from 'svelte-tauri-filedrop';
  import type { Feature } from './Map';
  import MapPopup from './MapPopup.svelte';
  import Menu from './Menu.svelte';
  import { ScaleControl } from './ScaleControl';
  import Highlight from 'svelte-highlight';
  import json from 'svelte-highlight/src/languages/json';
  import dark from 'svelte-highlight/src/styles/nnfx-dark';
  import light from 'svelte-highlight/src/styles/nnfx-light';

  let map: Map = null;
  let secondaryMap: Map = null;
  let compareMap: Compare;
  let wantTileBounds = false;
  let mainPopupOnClick = true;
  let secondaryPopupOnClick = true;
  let mainShowBackgroundLayer = false;
  let secondaryShowBackgroundLayer = false;
  let showBottomPanel = false;
  let basemap = 'basic';
  let bottomSplit: Split;
  let secondarySplit: Split;
  // let mapLayers: string[] = [];
  // let mapSources: string[] = [];
  let unlistener: UnlistenFn;
  let unlistenerReload: UnlistenFn;

  let mainFeatures: Feature[];
  let secondaryFeatures: Feature[];
  let mainSources = [];
  let secondarySources = [];
  let wantPopup = true;

  // $: console.log('mainFeatures', mainFeatures);
  onMount(async () => {
    // const styleSrc = await resolve(await resourceDir(), '../resources/styles/streets.json');
    // console.log('test', styleSrc)
    unlistener = await listen<{ path: string; json_url: string; key: string; source_id: string }>(
      'mbtiles',
      (event) => {
        onMBTilesSet(event.payload);
      }
    );

    unlistenerReload = await listen<{ message: string }>('reload-mbtiles', (event) => {
      let mapToRefresh = event.payload.message === 'secondary' ? secondaryMap : map;
      Object.keys(mapToRefresh.style.sourceCaches).forEach((s) => {
        // Remove the tiles for a particular source
        mapToRefresh.style.sourceCaches[s].clearTiles();
        // Load the new tiles for the current viewport (map.transform -> viewport)
        mapToRefresh.style.sourceCaches[s].update(mapToRefresh.transform);
      });
      // Force a repaint, so that the map will be repainted without you having to touch the map
      mapToRefresh.triggerRepaint();
    });

    const currentFile = localStorage.getItem('currentMBtiles');
    if (currentFile && currentFile !== 'undefined') {
      setupMBtiles(currentFile);
    }
  });

  let hasSources = false;
  function setupMBtiles(filePath, key = 'main') {
    try {
      invoke('setup_mbtiles', {
        key,
        path: filePath,
      });
    } catch (error) {
      console.error(error);
    }
  }

  onDestroy(() => {
    unlistener();
    unlistenerReload();
  });

  function brightColor(layerId, alpha?) {
    let luminosity = 'bright';
    let hue = null;

    if (/water|ocean|lake|sea|river/.test(layerId)) {
      hue = 'blue';
    }

    if (/state|country|place/.test(layerId)) {
      hue = 'pink';
    }

    if (/road|highway|transport/.test(layerId)) {
      hue = 'orange';
    }

    if (/contour|building/.test(layerId)) {
      hue = 'monochrome';
    }

    if (/building/.test(layerId)) {
      luminosity = basemap.indexOf('dark') !== -1 ? 'light' : 'dark';
      hue = 'monochrome';
    }

    if (/contour|landuse/.test(layerId)) {
      hue = 'yellow';
    }

    if (/wood|forest|park|landcover/.test(layerId)) {
      hue = 'green';
    }

    const rgb = randomColor({
      luminosity: luminosity,
      hue: hue,
      seed: layerId,
      format: 'rgbArray',
    });
    const rgba = rgb.concat([alpha || 1]);
    return 'rgba(' + rgba.join(', ') + ')';
  }
  function layerIdPrefix(sId, id: string) {
    return `___${sId}___${id}`;
  }
  function addPolygonLayer(map: Map, sId, id: string, layerColor, layers) {
    let layerId = `${layerIdPrefix(sId, id)}-polygons`;

    // mapLayers.push(layerId);
    layers.polygons.push(layerId);
    map.addLayer({
      id: layerId,
      type: 'fill',
      source: sId,
      'source-layer': id,
      filter: ['==', '$type', 'Polygon'],
      layout: {},
      paint: {
        'fill-opacity': 0.1,
        'fill-color': layerColor,
      },
    });
    layerId = layerId + `-outline`;
    layers.polygons.push(layerId);
    // mapLayers.push(layerId);
    map.addLayer({
      id: layerId,
      type: 'line',
      source: sId,
      'source-layer': id,
      filter: ['==', '$type', 'Polygon'],
      layout: {
        'line-join': 'round',
        'line-cap': 'round',
      },
      paint: {
        'line-color': layerColor,
        'line-width': 1,
        'line-opacity': 0.75,
      },
    });
  }
  function addPointLayer(map: Map, sId, id: string, layerColor, layers) {
    let layerId = `${layerIdPrefix(sId, id)}-points`;
    // mapLayers.push(layerId);
    layers.points.push(layerId);
    map.addLayer({
      id: layerId,
      type: 'circle',
      source: sId,
      'source-layer': id,
      filter: ['==', '$type', 'Point'],
      paint: {
        'circle-color': layerColor,
        'circle-radius': 2.5,
        'circle-opacity': 0.75,
      },
    });
  }
  function addLineLayer(map: Map, sId, id: string, layerColor, layers) {
    let layerId = `${layerIdPrefix(sId, id)}-lines`;
    // mapLayers.push(layerId);
    layers.lines.push(layerId);
    map.addLayer({
      id: layerId,
      type: 'line',
      source: sId,
      'source-layer': id,
      filter: ['==', '$type', 'LineString'],
      layout: {
        'line-join': 'round',
        'line-cap': 'round',
      },
      paint: {
        'line-color': layerColor,
        'line-width': 1,
        'line-opacity': 0.75,
      },
    });
  }

  async function removeDataSource(key, source) {
    const resultMap = key === 'main' ? map : secondaryMap;
    const layers = source.layers;
    const layerIds = Object.keys(resultMap.style._layers).filter(
      (s) => s.startsWith(`___${source.id}`) || s === `${source.id}-layer`
    );
    layerIds.forEach((s) => {
      resultMap.removeLayer(s);
      delete layers[s];
    });
    resultMap.removeSource(source.id);
    if (key === 'main') {
      const index = mainSources.findIndex((s) => s.id === source.id);
      if (index !== -1) {
        mainSources.splice(index, 1);
        mainSources = mainSources;
      }
      updateMainSourcesCount();
    } else {
      const index = secondarySources.findIndex((s) => s.id === source.id);
      if (index !== -1) {
        secondarySources.splice(index, 1);
        secondarySources = secondarySources;
      }
    }
  }
  function updateMainSourcesCount() {
    hasSources = mainSources.length > 0;
    if (hasSources) {
      localStorage.setItem('currentMBtiles', mainSources[0].path);
    }
  }

  async function addRasterMBtiles(
    resultMap: Map,
    { key, path, json_url, source_id },
    sourceData,
    createSourceLayer = true
  ) {
    if (!sourceData) {
      sourceData = await (await fetch(json_url)).json();
    }
    sourceData.path = path;

    function onMapLoaded() {
      if (createSourceLayer) {
        resultMap.addSource(sourceData.id, {
          type: 'raster',
          tiles: sourceData.tiles,

          minzoom: sourceData.minzoom,
          maxzoom: sourceData.maxzoom,
          attribution: sourceData.attribution || '',
        });
        resultMap.addLayer({
          id: sourceData.id + '-layer',
          type: 'raster',
          source: sourceData.id,
          minzoom: 0,
          maxzoom: 24,
        });
      }
      sourceData.layers = {
        rasters: [sourceData.id + '-layer'],
      };
      if (key === 'main') {
        mainSources.push(sourceData);
        mainSources = mainSources;
      } else {
        secondarySources.push(sourceData);
        secondarySources = secondarySources;
      }
      updateMainSourcesCount();
    }
    if (resultMap.loaded()) {
      onMapLoaded();
    } else {
      resultMap.once('load', onMapLoaded);
    }
  }
  async function addVectorMBtiles(resultMap: Map, { key, path, json_url, source_id }, vectorData) {
    if (!vectorData) {
      vectorData = await (await fetch(json_url)).json();
    }
    vectorData.path = path;

    function onMapLoaded() {
      const sId = vectorData.id;
      resultMap.addSource(sId, {
        type: 'vector',
        url: json_url,
      });
      const layers = (vectorData.layers = {
        points: [],
        rasters: [],
        lines: [],
        polygons: [],
        colors: {},
      });
      (vectorData.vector_layers || vectorData.Layer).forEach((l) => {
        const layerColor = brightColor(l.id);
        layers.colors[l.id] = layerColor;
        addPolygonLayer(resultMap, sId, l.id, layerColor, layers);
        addLineLayer(resultMap, sId, l.id, layerColor, layers);
        addPointLayer(resultMap, sId, l.id, layerColor, layers);
      });

      if (key === 'main') {
        mainSources.push(vectorData);
        mainSources = mainSources;
      } else {
        secondarySources.push(vectorData);
        secondarySources = secondarySources;
      }
      updateMainSourcesCount();
    }
    if (resultMap.loaded()) {
      onMapLoaded();
    } else {
      resultMap.once('load', onMapLoaded);
    }
  }

  async function createMap({ key, path, json_url, source_id }) {
    let containerKey = key;
    let resultMap: Map;
    let sourceData = await (await fetch(json_url)).json();
    let center;
    let zoom;
    if (key === 'main') {
      zoom = sourceData.minzoom + (sourceData.maxzoom - sourceData.minzoom) / 2;
      center =
        sourceData.center ?? sourceData.bounds
          ? [
              sourceData.bounds[0] + (sourceData.bounds[2] - sourceData.bounds[0]) / 2,
              sourceData.bounds[3] + (sourceData.bounds[1] - sourceData.bounds[3]) / 2,
            ]
          : undefined;
    } else {
      zoom = map.getZoom();
      center = map.getCenter();
    }
    if (sourceData.vector_layers || sourceData.Layer) {
      // let newLayers: Layers = {
      //   points: [],
      //   lines: [],
      //   rasters: [],
      //   polygons: [],
      //   colors: {},
      // };
      // if (key === 'main') {
      //   mainLayers = newLayers;
      // } else {
      //   secondaryLayers = newLayers;
      // }
      const styleSrc = await resolve(await resourceDir(), `_up_/resources/styles/${basemap}.json`);
      const styleStr: string = await readTextFile(styleSrc);
      const style = JSON.parse(styleStr.replace('{{json_url}}', json_url));
      const showBackground =
        key === 'main' ? mainShowBackgroundLayer : secondaryShowBackgroundLayer;
      style.layers.forEach((l) => {
        l.layout = l.layout || {};
        l.layout.visibility = showBackground ? 'visible' : 'none';
      });
      resultMap = new Map({
        container: containerKey,
        style,
        center,
        zoom,
        interactive: true,
      });
      resultMap.showTileBoundaries = wantTileBounds;
      addVectorMBtiles(resultMap, { key, path, json_url, source_id }, sourceData);
    } else {
      // addRasterMBtiles(resultMap, {key, path, json_url}, sourceData);
      resultMap = new Map({
        container: containerKey,
        style: {
          version: 8,
          sources: {
            [sourceData.id]: {
              type: 'raster',
              tiles: sourceData.tiles,

              minzoom: sourceData.minzoom,
              maxzoom: sourceData.maxzoom,
              attribution: sourceData.attribution || '',
            },
          },
          layers: [
            {
              id: sourceData.id + '-layer',
              type: 'raster',
              source: sourceData.id,
              minzoom: 0,
              maxzoom: 24,
            } as any,
          ],
        },
        center,
        zoom,
        interactive: true,
      });
      addRasterMBtiles(resultMap, { key, path, json_url, source_id }, sourceData, false);
    }

    // if (key === 'main') {
    resultMap.addControl(new ScaleControl({}), 'top-right');
    // resultMap.addControl(new RulerControl(), 'top-right');
    resultMap.addControl(new CompassControl(), 'top-right');
    resultMap.addControl(new ZoomControl(), 'top-right');
    // }
    return resultMap;
  }

  function clearMainMap() {
    if (map) {
      try {
        map.remove();
        hasSources = false;
      } catch (err) {
        console.error(err);
      }
      mainSources = [];
    }
  }
  function clearSecondaryMap() {
    if (secondaryMap) {
      try {
        secondaryMap.remove();
      } catch (err) {
        console.error(err);
      }
      secondaryMap = null;
      secondaryFeatures = [];
      secondarySources = [];
      secondarySplit.setPercent(100);
    } else {
      addMBTiles('secondary');
    }
  }

  async function onMBTilesSet({
    path,
    json_url,
    source_id,
    key,
  }: {
    path: string;
    json_url: string;
    key: string;
    source_id: string;
  }) {
    // console.log('onMBTilesSet', path, json_url, source_id, key);
    // if (key === 'main') {
    //   clearMainMap();
    // }
    // clearSecondaryMap();
    // if (compareMap) {
    //   compareMap.remove();
    //   compareMap = null;
    // }
    if (path) {
      if (key === 'main') {
        if (!hasSources) {
          map = await createMap({ key, path, json_url, source_id });
        } else {
          let sourceData = await (await fetch(json_url)).json();

          if (sourceData.vector_layers || sourceData.Layer) {
            addVectorMBtiles(map, { key, path, json_url, source_id }, sourceData);
          } else {
            addRasterMBtiles(map, { key, path, json_url, source_id }, sourceData);
          }
        }
      } else if (key === 'secondary') {
        if (secondaryMap) {
          let sourceData = await (await fetch(json_url)).json();
          if (sourceData.vector_layers || sourceData.Layer) {
            addVectorMBtiles(secondaryMap, { key, path, json_url, source_id }, sourceData);
          } else {
            addRasterMBtiles(secondaryMap, { key, path, json_url, source_id }, sourceData);
          }
        } else {
          secondaryMap = await createMap({ key, path, json_url, source_id });
          mainPopupOnClick = true;
          secondaryPopupOnClick = true;
          compareMap = new Compare(map, secondaryMap, '#comparison-container', {
            // mousemove: true, // Optional. Set to true to enable swiping during cursor movement.
            // orientation: 'horizontal', // Optional. Sets the orientation of swiper to horizontal or vertical, defaults to vertical
          });
          compareMap.setSlider(500);
        }
      }
    }
  }

  async function addMBTiles(key) {
    try {
      const resPath = await open({
        filters: [],
        multiple: false,
        directory: false,
      });
      setupMBtiles(resPath, key);
    } catch (error) {
      console.error(error);
    }
  }

  function handleDroppedFile(paths: string[]) {
    // ...
    setupMBtiles(paths[0]);
  }

  listen<string>('tauri://menu', ({ payload }) => {
    switch (payload) {
      case 'learn_more':
        openURl(REPO_URL);
        break;
    }
  });
  let theme: CarbonTheme = 'g90';

  let bottomPanelPercent = 75;
  function onBottomSplitChanged(e) {
    if (showBottomPanel) {
      bottomPanelPercent = e.detail.percent;
    }
    map?.resize();
    secondaryMap?.resize();
  }
  function switchBottomPanel() {
    showBottomPanel = !showBottomPanel;
    if (showBottomPanel) {
      bottomSplit.setPercent(bottomPanelPercent);
    } else {
      bottomSplit.setPercent(100);
    }
  }
  function switchCompareView() {
    if (compareMap) {
      clearSecondaryMap();
      if (compareMap) {
        compareMap.remove();
        compareMap = null;
      }
    } else {
      addMBTiles('secondary');
    }
  }

  let selectedFeaturesHeaders;
  let selectedFeaturesData;

  function handleSelectedFeatures(features) {
    if (features?.length > 0) {
      selectedFeaturesData = [];
      selectedFeaturesHeaders = [
        {
          key: 'layer',
          value: 'layer',
        },
        {
          key: '$type',
          value: '$type',
        },
        {
          key: '$id',
          value: '$id',
        },
      ];
      let seenKeys = selectedFeaturesHeaders.map((h) => h.key);
      let seenFeatures = [];
      features.forEach((f) => {
        Object.keys(f.properties).forEach((k) => {
          if (seenKeys.indexOf(k) === -1) {
            seenKeys.push(k);
            selectedFeaturesHeaders.push({ key: k, value: k });
          }
        });
        if (seenFeatures.indexOf(f.id) === -1) {
          seenFeatures.push(f.id);

          selectedFeaturesData.push({
            id: f.id,
            layer: f.sourceLayer,
            $type: f.type,
            $id: f.id,
            ...f.properties,
            data: f,
          });
        }
      });
    }
  }
  $: handleSelectedFeatures(mainFeatures)
  $: handleSelectedFeatures(secondaryFeatures)
</script>

<Theme bind:theme persist persistKey="__carbon-theme" />
<svelte:head>
  {#if theme === 'white'}
    {@html light}
  {:else}
    {@html dark}
  {/if}
</svelte:head>

<div class="drawer-container">
  <Header company="MBTiles" platformName="Viewer">
    <svelte:fragment slot="skip-to-content">
      <SkipToContent />
    </svelte:fragment>
    <HeaderUtilities>
      <HeaderGlobalAction
        aria-label={$_('opens_split')}
        icon={SplitScreen16}
        on:click={switchCompareView}
      />
      <HeaderGlobalAction
        aria-label={$_('open_bottom_panel')}
        icon={OpenPanelBottom16}
        on:click={switchBottomPanel}
      />
      <HeaderAction>
        <div style="padding:10px">
          <h3 style:margin-bottom="30px">{$_('settings')}</h3>
          <Select labelText={$_('theme')} bind:selected={theme}>
            <SelectItem value="white" text="White" />
            <SelectItem value="g10" text="Gray 10" />
            <SelectItem value="g80" text="Gray 80" />
            <SelectItem value="g90" text="Gray 90" />
            <SelectItem value="g100" text="Gray 100" />
          </Select>
        </div>
      </HeaderAction>
    </HeaderUtilities>
  </Header>

  <div style="padding-top:3rem;flex:auto;">
    <Split
      on:changed={onBottomSplitChanged}
      bind:this={bottomSplit}
      horizontal
      initialPrimarySize="100%"
      minPrimarySize="50%"
      splitterSize={showBottomPanel ? '10px' : '0px'}
    >
      <Split
        slot="primary"
        initialPrimarySize="270px"
        minPrimarySize="270px"
        minSecondarySize="50%"
      >
        <Menu
          id="primary"
          slot="primary"
          sources={mainSources}
          {map}
          on:add_source={() => addMBTiles('main')}
          on:remove_source={(event) => removeDataSource('main', event.detail)}
          bind:wantPopup
          bind:wantTileBounds
          bind:showBackgroundLayer={mainShowBackgroundLayer}
        />
        <svelte:fragment slot="secondary">
          <Split
            initialPrimarySize="100%"
            bind:this={secondarySplit}
            minPrimarySize="50%"
            splitterSize={secondaryMap ? '10px' : '0px'}
          >
            <div id="app-content" slot="primary">
              <FileDrop
                extensions={['mbtiles', 'etiles']}
                handleFiles={handleDroppedFile}
                let:files
              >
                <div class="dropzone" class:droppable={files.length > 0}>
                  {#if files.length > 0}
                    <h1 style:text-align="center" style:word-break="break-word">
                      Import Mbtiles: <br />
                      {files[0]}
                    </h1>
                  {/if}
                </div>
              </FileDrop>
              <!-- <div
              style="position:absolute; width:100%;height:100%;display:flex;z-index:100;pointer-events:none;"
            >
            </div> -->
              {#if !hasSources}
                <h1 id="no_mbtiles">{$_('drop_open_mbtiles')}</h1>
              {/if}
              <div id="comparison-container">
                <div id="secondary" class="map">
                  <MapPopup
                    map={secondaryMap}
                    sources={secondarySources}
                    bind:features={secondaryFeatures}
                    enabled={wantPopup}
                    onlyOnClick={secondaryPopupOnClick}
                  />
                </div>
                <div id="main" class="map">
                  <MapPopup
                    {map}
                    sources={mainSources}
                    bind:features={mainFeatures}
                    enabled={wantPopup}
                    onlyOnClick={mainPopupOnClick}
                  />
                </div>
              </div>
            </div>
            <svelte:fragment slot="secondary">
              {#if secondaryMap}
                <Menu
                  id="secondary"
                  on:remove_source={(event) => removeDataSource('secondary', event.detail)}
                  on:add_source={() => addMBTiles('secondary')}
                  sources={secondarySources}
                  map={secondaryMap}
                  bind:wantPopup
                  bind:wantTileBounds
                  bind:showBackgroundLayer={secondaryShowBackgroundLayer}
                />
              {/if}
            </svelte:fragment>
          </Split>
        </svelte:fragment>
      </Split>
      <svelte:fragment slot="secondary">
        <div style:height="100%" style:overflow="auto">
          <DataTable
            class="canSelectText"
            size="short"
            expandable
            headers={selectedFeaturesHeaders}
            rows={selectedFeaturesData}
          >
            <svelte:fragment slot="expanded-row" let:row>
              <Highlight
                class="canSelectText"
                language={json}
                code={JSON.stringify(row.data, null, 2)}
              />
            </svelte:fragment>
          </DataTable>
        </div>
      </svelte:fragment>
    </Split>
  </div>
</div>

<style>
  .dropzone {
    position: absolute;
    width: 100%;
    height: 100%;
    pointer-events: none;
    z-index: 100;
    padding: 20px;
    background: transparent;
    border: 1 solid #eee;
    display: flex;
    justify-content: center;
    align-items: center;
  }
  .droppable {
    background: #d6dff088;
  }
</style>
