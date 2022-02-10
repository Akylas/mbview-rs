<script lang="ts">
  import { open } from '@tauri-apps/api/dialog';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api';
  import { Map } from 'maplibre-gl';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import { resolve, resourceDir } from '@tauri-apps/api/path';
  import { emit, listen } from '@tauri-apps/api/event';
  import { randomColor } from 'randomcolor';

  let map: Map;
  let mapContainer;
  let basemap = 'basic';

  const tilesJSON = 'mbtiles://test/tiles.json';
  // const tilesJSON = 'http://localhost:8082/data/data.json';

  onMount(async () => {
    // const styleSrc = await resolve(await resourceDir(), '../resources/styles/streets.json');
    // console.log('test', styleSrc)
    try {
      map = new Map({
        container: mapContainer,
        style: `asset://../resources/styles/${basemap}.json`,
        center: { lat: 45, lon: 5 },
        zoom: 10,
        interactive: true,
      });
      invoke('setup_mbtiles', {
        // path: '/Volumes/dev/openmaptiles/openmaptiles/data/tiles.mbtiles',
        path: 'asset://../resources/world_cities.mbtiles',
      });
    } catch (error) {
      console.error(error);
    }
  });
  let mapLayers: string[] = [];
  let mapSources: string[] = [];

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

  function addPolygonLayer(id: string) {
    console.log('addPolygonLayer', id);
    const layerColor = brightColor(id);
    let layerId = `${id}-polygons`;
    mapLayers.push(layerId);
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

  async function onMBTilesSet(mbtilesPath) {
    console.log('onMBTilesSet', mbtilesPath);
    mapLayers.forEach((i) => map.removeLayer(i));
    mapLayers = [];
    mapSources.forEach((i) => map.removeSource(i));
    mapSources = [];

    if (mbtilesPath) {
      let result = await (await fetch(tilesJSON)).json();
      map.setCenter(result.center);
      console.log('got result ', result);
      mapSources.push('mbtiles');
      map.addSource('mbtiles', {
        type: 'vector',
        url: tilesJSON,
      });

      result.vector_layers.forEach((l) => {
        addPolygonLayer(l.id);
      });
    }
  }

  async function selectMBtiles() {
    try {
      const resPath = await open({
        filters: [],
        multiple: false,
        directory: false,
      });
      console.log('resPath', resPath);
      await invoke('setup_mbtiles', {
        path: resPath,
      });
    } catch (error) {
      console.error(error);
    }
  }

  listen<{ message: string }>('mbtiles', (event) => {
    onMBTilesSet(event.payload.message);
  });
</script>

<div class="container">
  <button type="button" id="open-dialog" on:click={selectMBtiles} style="z-index:100;"
    >Selectionner le fichier</button
  >
  <div class="map" id="map" bind:this={mapContainer} />
</div>
