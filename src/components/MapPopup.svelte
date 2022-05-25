<script lang="ts">
  import { Map, Popup } from 'maplibre-gl';
  import type { Feature, Layers, Source } from './Map';
  import PopupProperty from './PopupProperty.svelte';
  export let features: Feature[] = [];
  let colors: Record<string, string>;
  let layers: Layers;
  export let enabled = true;
  export let onlyOnClick = false;
  export let map: Map;
  export let sources: Source[];
  let hasMultipleSources: boolean = false;
  let main: HTMLDivElement;

  let popup = new Popup({
    closeButton: false,
    closeOnClick: false,
    className: 'map_popup',
    // closeOnMove: false,
  });

  $: {
    if (sources) {

      hasMultipleSources = Object.keys(sources).length > 1;
 
      colors = sources.reduce((prev, source) => {
        source.layers.colors && Object.keys(source.layers.colors).forEach((c) => {
          if (!prev[c]) {
            prev[c] = source.layers.colors[c];
          }
        });
        return prev;
      }, {});

      layers = sources.reduce(
        (prev, curr) => {
          const layer = curr.layers;
          if (layer.points) {
            prev.points.push(...layer.points);
          }
          if (layer.lines) {
            prev.lines.push(...layer.lines);
          }
          if (layer.polygons) {
            prev.polygons.push(...layer.polygons);
          }
          if (layer.rasters) {
            prev.rasters.push(...layer.rasters);
          }
          return prev;
        },
        {
          points: [],
          rasters: [],
          lines: [],
          polygons: [],
        }
      );
    }
  }

  function handlePopup(sources: Source[], popup: Popup, e) {
    if (!sources) {
      return;
    }
    // set a bbox around the pointer
    var selectThreshold = 3;
    var queryBox = [
      [e.point.x - selectThreshold, e.point.y + selectThreshold], // bottom left (SW)
      [e.point.x + selectThreshold, e.point.y - selectThreshold], // top right (NE)
    ];

    features =
      map
        .queryRenderedFeatures(queryBox, {
          layers: layers.polygons.filter(s=>!s.endsWith('-outline')).concat(layers.lines.concat(layers.points)),
        })
        ?.sortBy({ prop: 'sourceLayer' })
        .map((f) => {
          const source = sources.find((s) => s.id === f['source']);
          f['sourceName'] = source.name || source.id;
          return f as Feature;
        }) || [];
    if (!features.length || !enabled) {
      popup.remove();
    } else {
      popup.setLngLat(e.lngLat).setDOMContent(main).addTo(map);
    }
  }
  let shiftKeyOn = false;
  $: {
    if (map) {
      map.getCanvas().style.cursor = 'pointer';
      map.on('mousemove', function (e) {
        if (e.originalEvent.shiftKey) {
          shiftKeyOn = true;
        }
        if (!e.originalEvent.shiftKey) {
          if (onlyOnClick && shiftKeyOn) {
            popup.remove();
            shiftKeyOn = false;
          }
          return;
        }
        handlePopup(sources, popup, e);
      });
      map.on('click', function (e) {
        if (onlyOnClick) {
          handlePopup(sources, popup, e);
        }
      });
    }
  }
</script>

<div bind:this={main} class="mbview_popup">
  {#each features as feature}
    {@const layerId = feature['sourceLayer']}
    <div class="mbview_feature">
      <div class="mbview_layer">
        <span style:color={colors[layerId]} style:font-weight="bold">#{layerId}</span>
        {#if hasMultipleSources}
          <span>({feature['sourceName']})</span>
        {/if}
      </div>
      {#if feature.id}
        <PopupProperty name="$id" value={feature.id} />
      {/if}
      <PopupProperty name="$type" value={feature.type} />
      {#if feature.properties}
        {#each Object.entries(feature.properties) as [name, value]}
          <PopupProperty {name} {value} />
        {/each}
      {/if}
    </div>
  {/each}
</div>

<style>
  .mbview_popup {
    margin: 10px;
    color: #333;
    display: table;
    font-size: 10px;
  }

  .mbview_feature:not(:last-child) {
    border-bottom: 1px solid #ccc;
  }
</style>
