<script lang="ts">
  import { open } from '@tauri-apps/api/dialog';
  import { onDestroy, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api';
  import { Map, Popup } from 'maplibre-gl';
  import 'maplibre-gl/dist/maplibre-gl.css';
  import { resolve, resourceDir } from '@tauri-apps/api/path';
  import { emit, listen, UnlistenFn } from '@tauri-apps/api/event';
  import { randomColor } from 'randomcolor';

  let map: Map;
  let mapContainer;
  let basemap = 'basic';

  const tilesJSON = 'mbtiles://test/tiles.json';
  // const tilesJSON = 'http://localhost:8082/data/data.json';

  let unlistener: UnlistenFn;
  onMount(async () => {
    // const styleSrc = await resolve(await resourceDir(), '../resources/styles/streets.json');
    // console.log('test', styleSrc)
    unlistener = await listen<{ message: string }>('mbtiles', (event) => {
      onMBTilesSet(event.payload.message);
    });
    try {
      invoke('setup_mbtiles', {
        path: '/Volumes/dev/openmaptiles/openmaptiles/data/tiles.mbtiles',
        // path: 'asset://../resources/world_cities.mbtiles',
      });
    } catch (error) {
      console.error(error);
    }
  });

  onDestroy(() => {
    unlistener();
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
    let layerId = `${id}-pts`;
    mapLayers.push(layerId);
    layers.pts.push(layerId);
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

  var popup = new Popup({
    closeButton: false,
    closeOnClick: false,
  });

  let layers = {
    pts: [],
    lines: [],
    polygons: [],
    colors: {},
  };
  let wantPopup = true;

  async function onMBTilesSet(mbtilesPath) {
    console.log('onMBTilesSet', mbtilesPath);
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

    if (mbtilesPath) {
      let result = await (await fetch(tilesJSON)).json();
      console.log('got result2 ', result);
      map = new Map({
        container: mapContainer,
        style: `asset://../resources/styles/${basemap}.json`,
        center: result.center,
        zoom: Math.max(result.minzoom, 10),
        interactive: true,
      });

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
            layers: layers.polygons.concat(layers.lines.concat(layers.pts)),
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
          url: tilesJSON,
        });
        let newLayers = {
          pts: [],
          lines: [],
          polygons: [],
          colors: {},
        };
        result.vector_layers.forEach((l) => {
          const layerColor = brightColor(l.id);
          newLayers.colors[l.id] = layerColor;
          addPolygonLayer(l.id, layerColor, newLayers);
          addLineLayer(l.id, layerColor, newLayers);
          addPointLayer(l.id, layerColor, newLayers);
        });
        layers = newLayers;
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
</script>

<div class="container">
  <button type="button" id="open-dialog" on:click={selectMBtiles} style="z-index:100;"
    >Selectionner le fichier</button
  >
  <div class="map" id="map" bind:this={mapContainer} />
</div>

<style>
</style>
