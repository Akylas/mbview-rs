<script lang="ts">
  import { pointToTile } from '@mapbox/tilebelt';
  import type { Map, MapMouseEvent } from 'maplibre-gl';
  import { onDestroy } from 'svelte';
  import { _ } from 'svelte-i18n';

  /**
   * The readout a tile debugger actually needs: where the camera is, and which
   * tile the pointer is over. Every field copies on click.
   */
  export let map: Map = null;
  export let onCopy: (text: string) => void = () => undefined;

  let zoom = 0;
  let lat = 0;
  let lng = 0;
  let tile: number[] | null = null;

  let attached: Map | null = null;

  function readCamera() {
    if (!attached) return;
    zoom = attached.getZoom();
    const center = attached.getCenter();
    lat = center.lat;
    lng = center.lng;
    tile = pointToTile(lng, lat, Math.floor(zoom));
  }

  function onPointer(event: MapMouseEvent) {
    if (!attached) return;
    lat = event.lngLat.lat;
    lng = event.lngLat.lng;
    tile = pointToTile(lng, lat, Math.floor(attached.getZoom()));
  }

  function attach(next: Map | null) {
    if (attached === next) return;
    detach();
    attached = next;
    if (!next) return;
    next.on('move', readCamera);
    next.on('mousemove', onPointer);
    next.on('mouseout', readCamera);
    readCamera();
  }

  function detach() {
    if (!attached) return;
    attached.off('move', readCamera);
    attached.off('mousemove', onPointer);
    attached.off('mouseout', readCamera);
    attached = null;
  }

  $: attach(map);
  onDestroy(detach);
</script>

{#if map}
  <div class="status">
    <button
      type="button"
      title={$_('copy')}
      on:click={() => onCopy(`${lat.toFixed(6)},${lng.toFixed(6)}`)}
    >
      {lat.toFixed(5)}, {lng.toFixed(5)}
    </button>
    <span class="sep" />
    <button type="button" title={$_('copy')} on:click={() => onCopy(zoom.toFixed(2))}>
      z{zoom.toFixed(2)}
    </button>
    {#if tile}
      <span class="sep" />
      <button
        type="button"
        title={$_('copy')}
        on:click={() => onCopy(`${tile[2]}/${tile[0]}/${tile[1]}`)}
      >
        {tile[2]}/{tile[0]}/{tile[1]}
      </button>
    {/if}
  </div>
{/if}

<style>
  .status {
    position: absolute;
    bottom: calc(6px + var(--safe-bottom));
    left: calc(6px + var(--safe-left));
    z-index: 10;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--surface-float);
    box-shadow: var(--shadow);
    color: var(--text-muted);
    font-family: var(--mono);
    font-size: 10px;
    pointer-events: auto;
  }
  .status button {
    padding: 2px 4px;
    border: none;
    border-radius: var(--radius-pill);
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .status button:hover {
    background: var(--surface-raised);
    color: var(--text);
  }
  .sep {
    width: 1px;
    height: 10px;
    background: var(--border);
  }
</style>
