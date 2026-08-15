<script lang="ts">
  import { Popup, type Map, type MapMouseEvent } from 'maplibre-gl';
  import { onDestroy } from 'svelte';
  import { _ } from 'svelte-i18n';
  import type { SourceEntry } from '../lib/sources';
  import type { InspectMode } from '../lib/settings';
  import type { Feature } from './Map';
  import PopupProperty from './PopupProperty.svelte';

  export let features: Feature[] = [];
  export let map: Map = null;
  export let sources: SourceEntry[] = [];
  export let mode: InspectMode = 'click';

  /** how far off the pointer a feature can be and still be picked, in px */
  const PICK_RADIUS = 3;

  let main: HTMLDivElement;
  let colors: Record<string, string> = {};
  let hasMultipleSources = false;

  const popup = new Popup({
    closeButton: false,
    closeOnClick: false,
    className: 'map_popup',
  });

  $: {
    hasMultipleSources = sources.length > 1;
    colors = sources.reduce((acc, source) => {
      source.layers.forEach((layer) => {
        if (!acc[layer.id]) acc[layer.id] = layer.color;
      });
      return acc;
    }, {} as Record<string, string>);
  }

  /**
   * Only what is actually drawn right now, so a hidden layer cannot answer a
   * query and put a feature in the popup that is nowhere on screen.
   */
  function queryableLayers() {
    const ids: string[] = [];
    sources.forEach((source) => {
      if (!source.visible) return;
      source.layers.forEach((layer) => {
        if (!layer.visible) return;
        ids.push(
          // the polygon outline would double every polygon hit
          ...layer.mapLayers.polygons.filter((id) => !id.endsWith('-outline')),
          ...layer.mapLayers.lines,
          ...layer.mapLayers.points
        );
      });
    });
    return ids.filter((id) => map?.getLayer(id));
  }

  function inspect(event: MapMouseEvent) {
    if (!map || !sources.length) return;
    const box: [[number, number], [number, number]] = [
      [event.point.x - PICK_RADIUS, event.point.y + PICK_RADIUS],
      [event.point.x + PICK_RADIUS, event.point.y - PICK_RADIUS],
    ];

    const layers = queryableLayers();
    const found = layers.length ? map.queryRenderedFeatures(box, { layers }) : [];

    features = (found ?? [])
      .sortBy({ prop: 'sourceLayer' })
      .map((feature) => {
        const source = sources.find((entry) => entry.id === feature['source']);
        feature['sourceName'] = source?.name ?? feature['source'];
        return feature as unknown as Feature;
      });

    if (!features.length) {
      popup.remove();
    } else {
      popup.setLngLat(event.lngLat).setDOMContent(main).addTo(map);
    }
  }

  /* ---------------------------------------------------------------------- */
  /* listener lifecycle                                                     */
  /* ---------------------------------------------------------------------- */

  // The handlers are bound once per map and read `mode` when they fire.
  // Re-registering them from a reactive block — which is what this component
  // used to do — stacked a fresh pair on every render and leaked the old ones.
  let attached: Map | null = null;
  let shiftWasDown = false;

  function onMove(event: MapMouseEvent) {
    if (mode === 'off') return;
    const shift = (event.originalEvent as MouseEvent)?.shiftKey;
    if (mode === 'hover' || shift) {
      shiftWasDown = shift;
      inspect(event);
    } else if (shiftWasDown) {
      // shift released: drop the popup it was holding open
      shiftWasDown = false;
      popup.remove();
    }
  }

  function onClick(event: MapMouseEvent) {
    if (mode === 'click') inspect(event);
  }

  function attach(next: Map | null) {
    if (attached === next) return;
    detach();
    attached = next;
    if (!next) return;
    next.on('mousemove', onMove);
    next.on('click', onClick);
  }

  function detach() {
    if (!attached) return;
    attached.off('mousemove', onMove);
    attached.off('click', onClick);
    try {
      popup.remove();
    } catch (error) {
      /* the map may already be gone */
    }
    attached = null;
  }

  $: attach(map);
  $: if (map?.getCanvas()) {
    map.getCanvas().style.cursor = mode === 'off' ? '' : 'crosshair';
  }
  $: if (mode === 'off') popup.remove();

  onDestroy(detach);
</script>

<!-- The content lives here until maplibre re-parents it into the popup, so the
     host stays hidden and never shows up as stray text over the map. -->
<div class="popup-host">
<div bind:this={main} class="mbview_popup selectable">
  {#each features as feature}
    {@const layerId = feature['sourceLayer']}
    <div class="feature">
      <div class="layer">
        <span class="dot" style:background={colors[layerId] ?? 'var(--text-faint)'} />
        <span class="layer-name">{layerId}</span>
        {#if hasMultipleSources}
          <span class="source-name">{feature['sourceName']}</span>
        {/if}
      </div>
      <div class="props">
        {#if feature.geometry.type === 'Point'}
          <PopupProperty
            name="lat,lon"
            value={`${feature.geometry.coordinates[1]},${feature.geometry.coordinates[0]}`}
          />
        {/if}
        {#if feature.id !== undefined}
          <PopupProperty name="$id" value={feature.id} />
        {/if}
        <PopupProperty name="$type" value={feature.geometry.type} />
        {#if feature.properties}
          {#each Object.entries(feature.properties) as [name, value]}
            <PopupProperty {name} {value} />
          {/each}
        {/if}
      </div>
    </div>
  {/each}
  {#if features.length === 0}
    <div class="feature"><div class="props">{$_('no_features')}</div></div>
  {/if}
</div>
</div>

<style>
  .popup-host {
    display: none;
  }
  .mbview_popup {
    max-height: 44vh;
    overflow-y: auto;
    color: var(--text);
    font-size: 11px;
  }
  .feature {
    padding: 6px 10px;
  }
  .feature:not(:last-child) {
    border-bottom: 1px solid var(--border);
  }
  .layer {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-bottom: 3px;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
  }
  .layer-name {
    font-weight: 500;
  }
  .source-name {
    color: var(--text-faint);
    font-size: 10px;
  }
  .props {
    display: table;
    border-collapse: collapse;
  }
</style>
