<script lang="ts">
  import { Icon } from '@smui/common';
  import { AppContent, Title } from '@smui/drawer';
  import Fab from '@smui/fab';
  import { invoke } from '@tauri-apps/api';
  import { open } from '@tauri-apps/api/dialog';
  import { listen, UnlistenFn } from '@tauri-apps/api/event';
  import { readTextFile } from '@tauri-apps/api/fs';
  import { resolve, resourceDir } from '@tauri-apps/api/path';
  import { open as openURl } from '@tauri-apps/api/shell';
  import * as maplibregl from 'maplibre-gl';
  import { Map, Popup } from 'maplibre-gl';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import { randomColor } from 'randomcolor';
  import { onDestroy, onMount } from 'svelte';
  import 'svelte-material-ui/bare.css';
  import FileDrop from 'svelte-tauri-filedrop';
  import Menu from './Menu.svelte';
  import { _, isLoading } from 'svelte-i18n';
  import { ScaleControl } from './ScaleControl';
  import { CompassControl, RulerControl, ZoomControl } from 'mapbox-gl-controls';
  import { mapObject } from 'maplibre-gl/src/util/util';
  import Compare from '@maplibre/maplibre-gl-compare';
  import '@maplibre/maplibre-gl-compare/dist/maplibre-gl-compare.css';

  let map: Map;
  let secondaryMap: Map;
  let compareMap: Compare;
  let wantTileBounds = false;
  let popupOnClick = false;
  let showBackgroundLayer = true;
  let basemap = 'basic';
  // let mapLayers: string[] = [];
  // let mapSources: string[] = [];
  let unlistener: UnlistenFn;
  let unlistenerReload: UnlistenFn;

  // $: {
  //   if (!popupOnClick) {
  //     popup._onClose();
  //   }
  // }

  let popup = new Popup({
    closeButton: false,
    closeOnClick: false,
    className: 'map_popup',
    // closeOnMove: false,
  });

  let mainLayers = {
    points: [],
    lines: [],
    polygons: [],
    colors: {},
  };
  let mainSources = [];
  let secondaryLayers = {
    points: [],
    lines: [],
    polygons: [],
    colors: {},
  };
  let secondarySources = [];
  let wantPopup = true;

  const tilesJSON = 'http://localhost:9782/test/tiles.json';
  // const tilesJSON = 'http://localhost:8082/data/data.json';

  onMount(async () => {
    // const styleSrc = await resolve(await resourceDir(), '../resources/styles/streets.json');
    // console.log('test', styleSrc)
    unlistener = await listen<{ path: string; json_url: string; key: string }>(
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
    if (currentFile) {
      setupMBtiles(currentFile);
    }
  });

  let currentMbTiles = null;
  function setupMBtiles(filePath, key = 'main') {
    try {
      invoke('setup_mbtiles', {
        key,
        path: filePath,
      });
      currentMbTiles = filePath;
      if (key === 'main') {
        localStorage.setItem('currentMBtiles', filePath);
      }
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

  function addPolygonLayer(map: Map, id: string, layerColor, layers) {
    let layerId = `${id}-polygons`;

    // mapLayers.push(layerId);
    layers.polygons.push(layerId);
    map.addLayer({
      id: layerId,
      type: 'fill',
      source: 'mbtiles',
      'source-layer': id,
      filter: ['==', '$type', 'Polygon'],
      layout: {},
      paint: {
        'fill-opacity': 0.1,
        'fill-color': layerColor,
      },
    });
    layerId = `${id}-polygons-outline`;
    layers.polygons.push(layerId);
    // mapLayers.push(layerId);
    map.addLayer({
      id: layerId,
      type: 'line',
      source: 'mbtiles',
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
  function addPointLayer(map: Map, id: string, layerColor, layers) {
    let layerId = `${id}-points`;
    // mapLayers.push(layerId);
    layers.points.push(layerId);
    map.addLayer({
      id: layerId,
      type: 'circle',
      source: 'mbtiles',
      'source-layer': id,
      filter: ['==', '$type', 'Point'],
      paint: {
        'circle-color': layerColor,
        'circle-radius': 2.5,
        'circle-opacity': 0.75,
      },
    });
  }
  function addLineLayer(map: Map, id: string, layerColor, layers) {
    let layerId = `${id}-lines`;
    // mapLayers.push(layerId);
    layers.lines.push(layerId);
    map.addLayer({
      id: layerId,
      type: 'line',
      source: 'mbtiles',
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

  function displayValue(value, propName) {
    if (propName === '@timestamp') {
      return value.toString() + '<br>[ ' + new Date(value * 1000).toISOString() + ' ]';
    }
    if (typeof value === 'undefined' || value === null) return value;
    if (typeof value === 'object' || typeof value === 'number' || typeof value === 'string')
      return value.toString();
    return value;
  }

  function renderProperty(propertyName, property) {
    return (
      '<div class="mbview_property">' +
      '<div class="mbview_property-name">' +
      propertyName +
      '</div>' +
      '<div class="mbview_property-value">' +
      displayValue(property, propertyName) +
      '</div>' +
      '</div>'
    );
  }

  function renderLayer(layerId) {
    return `<div class="mbview_layer" style="color:${mainLayers.colors[layerId]};">${layerId}</div>`;
  }

  function renderProperties(feature) {
    var sourceProperty = renderLayer(feature.layer['source-layer'] || feature.layer.source);
    var idProperty = renderProperty('$id', feature.id);
    var typeProperty = renderProperty('$type', feature.geometry.type);
    var properties = Object.keys(feature.properties).map(function (propertyName) {
      return renderProperty(propertyName, feature.properties[propertyName]);
    });
    return (
      feature.id ? [sourceProperty, idProperty, typeProperty] : [sourceProperty, typeProperty]
    )
      .concat(properties)
      .join('');
  }

  function renderFeatures(features) {
    return features
      .map(function (ft) {
        return '<div class="mbview_feature">' + renderProperties(ft) + '</div>';
      })
      .join('');
  }

  function renderPopup(features) {
    return '<div class="mbview_popup">' + renderFeatures(features) + '</div>';
  }

  function handlePopup(e) {
    if (!mainLayers) {
      return;
    }
    // set a bbox around the pointer
    var selectThreshold = 3;
    var queryBox = [
      [e.point.x - selectThreshold, e.point.y + selectThreshold], // bottom left (SW)
      [e.point.x + selectThreshold, e.point.y - selectThreshold], // top right (NE)
    ];

    var features =
      map.queryRenderedFeatures(queryBox, {
        layers: mainLayers.polygons.concat(mainLayers.lines.concat(mainLayers.points)),
      }) || [];
    map.getCanvas().style.cursor = features.length ? 'pointer' : '';

    if (!features.length || !wantPopup) {
      popup.remove();
    } else {
      popup.setLngLat(e.lngLat).setHTML(renderPopup(features)).addTo(map);
    }
  }

  async function createMap({ key, path, json_url }) {
    let containerKey = key;
    let resultMap: Map;
    let result = await (await fetch(json_url)).json();

    let center;
    let zoom;
    if (key === 'main') {
      zoom = result.minzoom + (result.maxzoom - result.minzoom) / 2;
      center =
        result.center ?? result.bounds
          ? [
              result.bounds[0] + (result.bounds[2] - result.bounds[0]) / 2,
              result.bounds[3] + (result.bounds[1] - result.bounds[3]) / 2,
            ]
          : undefined;
    } else {
      zoom = map.getZoom();
      center = map.getCenter();
    }
    if (result.vector_layers || result.Layer) {
      const styleSrc = await resolve(await resourceDir(), `_up_/resources/styles/${basemap}.json`);
      const style: string = await readTextFile(styleSrc);
      resultMap = new Map({
        container: containerKey,
        style: JSON.parse(style.replace('{{json_url}}', json_url)),
        center,
        zoom,
        interactive: true,
      });
      resultMap.showTileBoundaries = wantTileBounds;

      if (key === 'main') {
        mainSources.push(result);
        resultMap.on('mousemove', function (e) {
          if (popupOnClick) {
            return;
          }
          handlePopup(e);
        });
        resultMap.on('click', function (e) {
          if (popupOnClick) {
            handlePopup(e);
          }
        });
      }

      resultMap.on('load', () => {
        // mapSources.push('mbtiles');
        resultMap.addSource('mbtiles', {
          type: 'vector',
          url: json_url,
        });
        let newLayers = {
          points: [],
          lines: [],
          polygons: [],
          colors: {},
        };
        (result.vector_layers || result.Layer).forEach((l) => {
          const layerColor = brightColor(l.id);
          newLayers.colors[l.id] = layerColor;
          addPolygonLayer(resultMap, l.id, layerColor, newLayers);
          addLineLayer(resultMap, l.id, layerColor, newLayers);
          addPointLayer(resultMap, l.id, layerColor, newLayers);
        });
        if (key === 'main') {
          mainLayers = newLayers;
        }
      });
    } else {
      resultMap = new Map({
        container: containerKey,
        style: {
          version: 8,
          sources: {
            'raster-tiles': {
              type: 'raster',
              tiles: result.tiles,

              minzoom: result.minzoom,
              maxzoom: result.maxzoom,
              attribution: result.attribution || '',
            },
          },
          layers: [
            {
              id: 'simple-tiles',
              type: 'raster',
              source: 'raster-tiles',
              minzoom: 0,
              maxzoom: 24,
            } as any,
          ],
        },
        center:
          result.center ?? result.bounds
            ? [
                result.bounds[0] + (result.bounds[2] - result.bounds[0]) / 2,
                result.bounds[3] + (result.bounds[1] - result.bounds[3]) / 2,
              ]
            : undefined,
        zoom: result.minzoom + (result.maxzoom - result.minzoom),
        interactive: true,
      });
    }

    // if (key === 'main') {
      resultMap.addControl(new ScaleControl({}), 'bottom-right');
      resultMap.addControl(new RulerControl(), 'bottom-right');
      resultMap.addControl(new CompassControl(), 'bottom-right');
      resultMap.addControl(new ZoomControl(), 'bottom-right');
    // }
    return resultMap;
  }

  function clearMainMap() {
    if (map) {
      try {
        map.remove();
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
    }
  }

  async function onMBTilesSet({
    path,
    json_url,
    key,
  }: {
    path: string;
    json_url: string;
    key: string;
  }) {
    if (key === 'main') {
      clearMainMap();
    }
    clearSecondaryMap();
    if (compareMap) {
      compareMap.remove();
      compareMap = null;
    }
    if (path) {
      if (key === 'main') {
        map = await createMap({ key, path, json_url });
      } else if (key === 'secondary') {
        secondaryMap = await createMap({ key, path, json_url });
        popupOnClick = true;
        compareMap = new Compare(map, secondaryMap, '#comparison-container', {
          // mousemove: true, // Optional. Set to true to enable swiping during cursor movement.
          // orientation: 'horizontal', // Optional. Sets the orientation of swiper to horizontal or vertical, defaults to vertical
        });
        compareMap.setSlider(500);
      }
    }
  }

  async function selectSecondaryMBtiles() {
    try {
      const resPath = await open({
        filters: [],
        multiple: false,
        directory: false,
      });
      setupMBtiles(resPath, 'secondary');
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
</script>

<div class="drawer-container">
  <Menu
    layers={mainLayers}
    sources={mainSources}
    {map}
    bind:wantPopup
    bind:wantTileBounds
    bind:popupOnClick
    bind:showBackgroundLayer
  />

  <AppContent id="app-content">
    <FileDrop
      style="position:absolute;z-index:100;"
      extensions={['mbtiles', 'etiles']}
      handleFiles={handleDroppedFile}
      let:files
    >
      <div class="dropzone" class:droppable={files.length > 0}>
        {#if files.length > 0}
          <Title>Import Mbtiles : {files[0]}</Title>
        {/if}
      </div>
    </FileDrop>
    <div
      style="position:absolute; width:100%;height:100%;display:flex;z-index:100;pointer-events:none;"
    >
      <Fab
        color="primary"
        on:click={selectSecondaryMBtiles}
        style="align-self:flex-end;margin: 20px;pointer-events:auto;"
      >
        <Icon class="material-icons">horizontal_distribute</Icon>
      </Fab>
    </div>
    {#if !currentMbTiles}
      <Title id="no_mbtiles">{$_('drop_open_mbtiles')}</Title>
    {/if}
    <div id="comparison-container">
      <div id="secondary" class="map" />
      <div id="main" class="map" />
    </div>
  </AppContent>
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
