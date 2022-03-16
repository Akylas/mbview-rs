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

  let map: Map;
  let mapContainer;
  let wantTileBounds = false;
  let drawerOpened = true;
  let basemap = 'basic';
  let mapLayers: string[] = [];
  let mapSources: string[] = [];
  let unlistener: UnlistenFn;

  let popup = new Popup({
    closeButton: false,
    closeOnClick: false,
  });

  let layers = {
    points: [],
    lines: [],
    polygons: [],
    colors: {},
  };
  let sources = [];
  let wantPopup = true;

  const tilesJSON = 'http://localhost:9782/test/tiles.json';
  // const tilesJSON = 'http://localhost:8082/data/data.json';

  onMount(async () => {
    // const styleSrc = await resolve(await resourceDir(), '../resources/styles/streets.json');
    // console.log('test', styleSrc)
    unlistener = await listen<{ path: string; json_url: string }>('mbtiles', (event) => {
      onMBTilesSet(event.payload);
    });

    const currentFile = localStorage.getItem('currentMBtiles');
    if (currentFile) {
      setupMBtiles(currentFile);
    }
  });

  let currentMbTiles = null;
  function setupMBtiles(filePath) {
    try {
      invoke('setup_mbtiles', {
        path: filePath,
      });
      currentMbTiles = filePath;
      localStorage.setItem('currentMBtiles', filePath);
    } catch (error) {
      console.error(error);
    }
  }

  onDestroy(() => {
    unlistener();
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

  function addPolygonLayer(id: string, layerColor, layers) {
    let layerId = `${id}-polygons`;

    mapLayers.push(layerId);
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
    mapLayers.push(layerId);
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
  function addPointLayer(id: string, layerColor, layers) {
    let layerId = `${id}-points`;
    mapLayers.push(layerId);
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
  function addLineLayer(id: string, layerColor, layers) {
    let layerId = `${id}-lines`;
    mapLayers.push(layerId);
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
    return `<div class="mbview_layer" style="color:${layers.colors[layerId]};">${layerId}</div>`;
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

  async function onMBTilesSet({ path, json_url }: { path: string; json_url: string }) {
    // console.log('onMBTilesSet', path, json_url);
    if (map) {
      // mapLayers.forEach((i) => map.removeLayer(i));
      // mapSources.forEach((i) => map.removeSource(i));
      try {
        map.remove();
      } catch (err) {
        console.error(err);
      }
    }
    mapSources = [];
    mapLayers = [];
    sources = [];

    if (path) {
      let result = await (await fetch(json_url)).json();
      console.log('result', Object.keys(result), result);
      if (result.vector_layers || result.Layer) {
        const styleSrc = await resolve(
          await resourceDir(),
          `_up_/resources/styles/${basemap}.json`
        );
        const style: string = await readTextFile(styleSrc);
        map = new Map({
          container: mapContainer,
          style: JSON.parse(style.replace('{{json_url}}', json_url)),
          center:
            result.center ?? result.bounds
              ? [
                  result.bounds[0] + (result.bounds[2] - result.bounds[0]) / 2,
                  result.bounds[3] + (result.bounds[1] - result.bounds[3]) / 2,
                ]
              : undefined,
          zoom: result.minzoom + (result.maxzoom - result.minzoom) / 2,
          interactive: true,
        });
        map.showTileBoundaries = wantTileBounds;
        sources.push(result);

        map.on('mousemove', function (e) {
          if (!layers) {
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
              layers: layers.polygons.concat(layers.lines.concat(layers.points)),
            }) || [];
          map.getCanvas().style.cursor = features.length ? 'pointer' : '';

          if (!features.length || !wantPopup) {
            popup.remove();
          } else {
            popup.setLngLat(e.lngLat).setHTML(renderPopup(features)).addTo(map);
          }
        });

        map.on('load', () => {
          mapSources.push('mbtiles');
          map.addSource('mbtiles', {
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
            addPolygonLayer(l.id, layerColor, newLayers);
            addLineLayer(l.id, layerColor, newLayers);
            addPointLayer(l.id, layerColor, newLayers);
          });
          layers = newLayers;
        });
      } else {
        map = new Map({
          container: mapContainer,
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

      map.addControl(new maplibregl.ScaleControl({}), 'bottom-right');
    }
  }

  async function selectMBtiles() {
    try {
      const resPath = await open({
        filters: [],
        multiple: false,
        directory: false,
      });
      setupMBtiles(resPath);
    } catch (error) {
      console.error(error);
    }
  }

  function handleDroppedFile(paths: string[]) {
    // ...
    setupMBtiles(paths[0]);
  }

  listen<string>('tauri://menu', ({ payload }) => {
    // console.log('on menu', payload);
    switch (payload) {
      case 'learn_more':
        openURl(REPO_URL);
        break;
    }
  });
</script>

<div class="drawer-container">
  <Menu {layers} {sources} {map} bind:wantPopup bind:wantTileBounds bind:drawerOpened />

  <!-- <Scrim fixed={false} /> -->
  <AppContent id="app-content">
    <!-- <IconButton
      class="material-icons"
      on:click={() => (drawerOpened = !drawerOpened)}
      style="position:absolute;z-index:100;">menu</IconButton
    > -->
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
      <Fab color="primary" on:click={selectMBtiles} style="align-self:flex-end;margin: 20px;pointer-events:auto;">
        <Icon class="material-icons">download</Icon>
      </Fab>
    </div>
    {#if !currentMbTiles}
      <label id="no_mbtiles">drop or open a MBtiles</label>
    {/if}
    <div class="map" id="map" bind:this={mapContainer} />
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
