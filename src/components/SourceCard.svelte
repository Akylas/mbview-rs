<script lang="ts">
  import ChevronDown from 'carbon-icons-svelte/lib/ChevronDown.svelte';
  import Draggable from 'carbon-icons-svelte/lib/Draggable.svelte';
  import OverflowMenuVertical from 'carbon-icons-svelte/lib/OverflowMenuVertical.svelte';
  import View from 'carbon-icons-svelte/lib/View.svelte';
  import ViewOff from 'carbon-icons-svelte/lib/ViewOff.svelte';
  import type { Map } from 'maplibre-gl';
  import { createEventDispatcher } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { applyOpacity, applyVisibility, savePrefs, type SourceEntry } from '../lib/sources';
  import type { GeometryFilter } from '../lib/settings';
  import Slider from './ui/Slider.svelte';

  export let source: SourceEntry;
  export let map: Map;
  export let filter: GeometryFilter = 'all';
  export let expanded = false;
  export let dragging = false;
  /** how many layer names it takes before the list gets its own search box */
  const SEARCH_THRESHOLD = 8;

  const dispatch = createEventDispatcher();

  let search = '';

  $: visibleLayers = search
    ? source.layers.filter((layer) => layer.id.toLowerCase().includes(search.toLowerCase()))
    : source.layers;
  $: hiddenCount = source.layers.filter((layer) => !layer.visible).length;

  function commit() {
    applyVisibility(map, source, filter);
    savePrefs(source);
    // the array in the parent holds the same objects, but svelte needs telling
    source = source;
  }

  function toggleSource() {
    source.visible = !source.visible;
    commit();
  }

  function toggleLayer(layerId: string) {
    const layer = source.layers.find((entry) => entry.id === layerId);
    if (!layer) return;
    layer.visible = !layer.visible;
    commit();
  }

  function setAllLayers(visible: boolean) {
    // only what the search is currently showing, so a filtered "none" is not a
    // surprise wipe of the layers you cannot see
    visibleLayers.forEach((layer) => (layer.visible = visible));
    commit();
  }

  function onOpacity() {
    applyOpacity(map, source);
    savePrefs(source);
  }

  // The element is read here rather than from the dispatched event: by the time
  // a parent handles it, `currentTarget` has already been cleared.
  function startDrag(event: PointerEvent) {
    dispatch('dragstart', { event, node: event.currentTarget as HTMLElement });
  }

  function openMenu(event: MouseEvent) {
    dispatch('menu', (event.currentTarget as HTMLElement).getBoundingClientRect());
  }
</script>

<div class="card" class:dragging class:off={!source.visible}>
  <div class="head">
    <span
      class="grip"
      role="button"
      tabindex="-1"
      aria-label={$_('reorder')}
      title={$_('reorder')}
      on:pointerdown={startDrag}
    >
      <Draggable size={16} />
    </span>

    <button
      type="button"
      class="eye"
      class:on={source.visible}
      aria-pressed={source.visible}
      aria-label={source.visible ? $_('hide_source') : $_('show_source')}
      title={source.visible ? $_('hide_source') : $_('show_source')}
      on:click={toggleSource}
    >
      <svelte:component this={source.visible ? View : ViewOff} size={16} />
    </button>

    <button
      type="button"
      class="title"
      aria-expanded={expanded}
      on:click={() => (expanded = !expanded)}
    >
      <span class="name" title={source.path}>{source.name}</span>
      <span class="meta">
        <span class="badge" class:vector={source.vector}>
          {source.vector ? source.encoding.toUpperCase() : (source.format || 'raster').toUpperCase()}
        </span>
        <span class="file">{source.file}</span>
        {#if hiddenCount > 0}
          <span class="badge warn">{$_('n_hidden', { values: { count: hiddenCount } })}</span>
        {/if}
      </span>
    </button>

    <!-- its own target rather than a decoration: the arrow is the affordance
         people aim at, and it used to be the one part of the row that did
         nothing when clicked -->
    <button
      type="button"
      class="chevron"
      class:open={expanded}
      aria-expanded={expanded}
      aria-label={expanded ? $_('collapse') : $_('expand')}
      title={expanded ? $_('collapse') : $_('expand')}
      on:click={() => (expanded = !expanded)}
    >
      <ChevronDown size={16} />
    </button>

    <button
      type="button"
      class="kebab"
      aria-label={$_('source_actions')}
      title={$_('source_actions')}
      on:click={openMenu}
    >
      <OverflowMenuVertical size={16} />
    </button>
  </div>

  {#if expanded}
    <div class="body">
      <Slider
        compact
        label={$_('opacity')}
        readout="{Math.round(source.opacity * 100)}%"
        min={0}
        max={1}
        step={0.05}
        bind:value={source.opacity}
        on:input={onOpacity}
      />

      {#if source.vector}
        <div class="layers-head">
          <span class="count">{$_('n_layers', { values: { count: source.layers.length } })}</span>
          <span class="bulk">
            <button type="button" on:click={() => setAllLayers(true)}>{$_('all')}</button>
            <button type="button" on:click={() => setAllLayers(false)}>{$_('none')}</button>
          </span>
        </div>

        {#if source.layers.length >= SEARCH_THRESHOLD}
          <input
            class="search"
            type="text"
            placeholder={$_('filter_layers')}
            autocomplete="off"
            spellcheck="false"
            bind:value={search}
          />
        {/if}

        <ul class="layers">
          {#each visibleLayers as layer (layer.id)}
            <li>
              <button
                type="button"
                class="layer"
                class:off={!layer.visible}
                disabled={!source.visible}
                aria-pressed={layer.visible}
                on:click={() => toggleLayer(layer.id)}
              >
                <span class="swatch" style:background={layer.color} />
                <span class="layer-name">{layer.id}</span>
                {#if layer.minzoom != null || layer.maxzoom != null}
                  <span class="zooms">z{layer.minzoom ?? 0}–{layer.maxzoom ?? source.maxzoom ?? ''}</span>
                {/if}
                <span class="layer-eye">
                  <svelte:component this={layer.visible ? View : ViewOff} size={16} />
                </span>
              </button>
            </li>
          {/each}
          {#if visibleLayers.length === 0}
            <li class="empty">{$_('no_matching_layers')}</li>
          {/if}
        </ul>
      {/if}
    </div>
  {/if}
</div>

<style>
  .card {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-raised);
    overflow: hidden;
  }
  .card.dragging {
    border-color: var(--accent);
    box-shadow: var(--shadow);
    opacity: 0.9;
  }
  .card.off .name {
    color: var(--text-faint);
  }

  .head {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px 2px 2px 0;
  }
  .grip {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 20px;
    height: var(--tap);
    flex: 0 0 auto;
    color: var(--text-faint);
    cursor: grab;
    touch-action: none;
  }
  .eye,
  .kebab {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 auto;
    width: calc(var(--tap) - 4px);
    height: calc(var(--tap) - 4px);
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
  }
  .eye.on {
    color: var(--accent);
  }
  .eye:hover,
  .kebab:hover {
    background: var(--surface);
    color: var(--text);
  }

  .title {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
    min-height: var(--tap);
    padding: 4px 4px;
    border: none;
    background: transparent;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .name {
    overflow: hidden;
    font-size: 13px;
    font-weight: 500;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 5px;
    min-width: 0;
    color: var(--text-faint);
    font-size: 10px;
  }
  .file {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    flex: 0 0 auto;
    padding: 0 4px;
    border-radius: 3px;
    background: var(--surface-sunken);
    color: var(--text-muted);
    font-size: 9px;
    letter-spacing: 0.04em;
  }
  .badge.vector {
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .badge.warn {
    background: var(--warning-soft);
    color: var(--warning-text);
  }
  .chevron {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 auto;
    width: calc(var(--tap) - 8px);
    height: calc(var(--tap) - 4px);
    padding: 0;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
    transition: transform 140ms ease;
    transform: rotate(-90deg);
  }
  .chevron:hover {
    color: var(--text);
  }
  .chevron.open {
    transform: rotate(0deg);
  }

  .body {
    padding: 0 8px 8px 22px;
  }
  .layers-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 2px 0 4px;
    color: var(--text-faint);
    font-size: 11px;
  }
  .bulk {
    display: flex;
    gap: 4px;
  }
  .bulk button {
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--text-muted);
    font-size: 10px;
    cursor: pointer;
  }
  .bulk button:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }
  .search {
    width: 100%;
    min-height: calc(var(--control-h) - 6px);
    margin-bottom: 4px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-sunken);
    color: var(--text);
    font-size: 12px;
  }

  .layers {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .layer {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    min-height: calc(var(--tap) - 6px);
    padding: 0 4px;
    border: none;
    border-radius: calc(var(--radius) - 4px);
    background: transparent;
    color: var(--text);
    font-size: 12px;
    text-align: left;
    cursor: pointer;
  }
  .layer:hover:not(:disabled) {
    background: var(--surface);
  }
  .layer:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .layer.off .layer-name {
    color: var(--text-faint);
    text-decoration: line-through;
  }
  .swatch {
    flex: 0 0 auto;
    width: 10px;
    height: 10px;
    border-radius: 3px;
    box-shadow: inset 0 0 0 1px var(--border-strong);
  }
  .layer.off .swatch {
    opacity: 0.3;
  }
  .layer-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .zooms {
    flex: 0 0 auto;
    color: var(--text-faint);
    font-family: var(--mono);
    font-size: 10px;
  }
  .layer-eye {
    display: inline-flex;
    flex: 0 0 auto;
    color: var(--text-faint);
  }
  .empty {
    padding: 8px 4px;
    color: var(--text-faint);
    font-size: 12px;
  }
</style>
