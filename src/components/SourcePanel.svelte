<script lang="ts">
  import Add from 'carbon-icons-svelte/lib/Add.svelte';
  import CenterToFit from 'carbon-icons-svelte/lib/CenterToFit.svelte';
  import ChevronDown from 'carbon-icons-svelte/lib/ChevronDown.svelte';
  import Copy from 'carbon-icons-svelte/lib/Copy.svelte';
  import Information from 'carbon-icons-svelte/lib/Information.svelte';
  import Layers from 'carbon-icons-svelte/lib/Layers.svelte';
  import Renew from 'carbon-icons-svelte/lib/Renew.svelte';
  import TrashCan from 'carbon-icons-svelte/lib/TrashCan.svelte';
  import type { Map } from 'maplibre-gl';
  import { createEventDispatcher } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { settings } from '../lib/settings';
  import { basename, type SourceEntry } from '../lib/sources';
  import SourceCard from './SourceCard.svelte';
  import Menu from './ui/Menu.svelte';
  import MenuItem from './ui/MenuItem.svelte';
  import SegmentedControl from './ui/SegmentedControl.svelte';

  export let sources: SourceEntry[] = [];
  export let map: Map = null;

  const dispatch = createEventDispatcher();

  let cardEls: HTMLElement[] = [];
  let dragIndex = -1;
  let expanded: Record<string, boolean> = {};

  /* ---------------------------------------------------------------------- */
  /* reordering                                                             */
  /* ---------------------------------------------------------------------- */

  /**
   * Pointer-driven rather than HTML5 drag-and-drop: the same code then works
   * under a finger, which `dragstart` never does.
   */
  function startDrag(index: number, event: PointerEvent, node: HTMLElement) {
    dragIndex = index;
    node.setPointerCapture(event.pointerId);
    event.preventDefault();

    const move = (moveEvent: PointerEvent) => {
      if (dragIndex < 0) return;
      const target = indexAt(moveEvent.clientY);
      if (target >= 0 && target !== dragIndex) {
        const [entry] = sources.splice(dragIndex, 1);
        sources.splice(target, 0, entry);
        sources = sources;
        dragIndex = target;
        dispatch('reorder');
      }
    };
    const end = () => {
      dragIndex = -1;
      node.removeEventListener('pointermove', move);
      node.removeEventListener('pointerup', end);
      node.removeEventListener('pointercancel', end);
      dispatch('reordered');
    };
    node.addEventListener('pointermove', move);
    node.addEventListener('pointerup', end);
    node.addEventListener('pointercancel', end);
  }

  function indexAt(clientY: number) {
    // `cardEls` can outlive a removed card, so the list length is the authority
    const count = sources.length;
    for (let index = 0; index < count; index++) {
      const rect = cardEls[index]?.getBoundingClientRect();
      if (!rect) continue;
      if (clientY < rect.top + rect.height / 2) return index;
    }
    return count - 1;
  }

  /* ---------------------------------------------------------------------- */
  /* per-source menu                                                        */
  /* ---------------------------------------------------------------------- */

  let menuOpen = false;
  let menuX = 0;
  let menuY = 0;
  let menuSource: SourceEntry | null = null;

  function openMenu(source: SourceEntry, rect: DOMRect) {
    menuX = rect.right;
    menuY = rect.bottom + 4;
    menuSource = source;
    menuOpen = true;
  }

  let addOpen = false;
  let addX = 0;
  let addY = 0;
  function openAdd(event: MouseEvent) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    addX = rect.right;
    addY = rect.top - 4;
    addOpen = true;
  }
</script>

<div class="panel">
  <div class="filters">
    <SegmentedControl
      bind:value={$settings.geometryFilter}
      options={[
        { value: 'all', label: $_('all') },
        { value: 'polygons', label: $_('polygons') },
        { value: 'lines', label: $_('lines') },
        { value: 'points', label: $_('points') },
      ]}
    />
  </div>

  <div class="list mb-scroll">
    {#if sources.length === 0}
      <div class="empty">
        <Layers size={24} />
        <p>{$_('no_sources_here')}</p>
      </div>
    {/if}

    {#each sources as source, index (source.id)}
      <div class="card-wrap" bind:this={cardEls[index]}>
        <SourceCard
          {source}
          {map}
          filter={$settings.geometryFilter}
          bind:expanded={expanded[source.id]}
          dragging={dragIndex === index}
          on:dragstart={(event) => startDrag(index, event.detail.event, event.detail.node)}
          on:menu={(event) => openMenu(source, event.detail)}
        />
      </div>
    {/each}
  </div>

  <div class="actions">
    <button type="button" class="add" on:click={() => dispatch('add', {})}>
      <Add size={16} />
      <span>{$_('add_mbtiles')}</span>
    </button>
    <button
      type="button"
      class="add more"
      aria-label={$_('add_as')}
      title={$_('add_as')}
      on:click={openAdd}
    >
      <ChevronDown size={16} />
    </button>
  </div>
</div>

<Menu bind:open={addOpen} x={addX} y={addY} alignRight onClose={() => (addOpen = false)}>
  <MenuItem
    label={$_('add_hillshade')}
    on:click={() => dispatch('add', { sourceType: 'raster-dem', layerType: 'hillshade' })}
  />
  <MenuItem
    label={$_('add_terrain_rgb')}
    on:click={() => dispatch('add', { sourceType: 'raster-dem', layerType: 'raster' })}
  />
  {#if $settings.recent.length}
    {#each $settings.recent.slice(0, 8) as path (path)}
      <MenuItem label={basename(path)} on:click={() => dispatch('add', { path })} />
    {/each}
  {/if}
</Menu>

<Menu bind:open={menuOpen} x={menuX} y={menuY} alignRight onClose={() => (menuOpen = false)}>
  <MenuItem
    label={$_('zoom_to_source')}
    icon={CenterToFit}
    on:click={() => dispatch('zoom', menuSource)}
  />
  <MenuItem
    label={$_('source_info')}
    icon={Information}
    on:click={() => dispatch('info', menuSource)}
  />
  <MenuItem
    label={$_('copy_path')}
    icon={Copy}
    on:click={() => dispatch('copyPath', menuSource)}
  />
  <MenuItem
    label={$_('reload_source')}
    icon={Renew}
    on:click={() => dispatch('reloadSource', menuSource)}
  />
  <MenuItem
    label={$_('remove_mbtiles')}
    icon={TrashCan}
    danger
    on:click={() => dispatch('remove', menuSource)}
  />
</Menu>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .filters {
    flex: 0 0 auto;
    padding: 6px 8px 2px;
  }
  .list {
    display: flex;
    flex: 1;
    flex-direction: column;
    gap: 6px;
    min-height: 0;
    padding: 6px 8px;
  }
  .card-wrap {
    flex: 0 0 auto;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 28px 8px;
    color: var(--text-faint);
    text-align: center;
  }
  .empty p {
    margin: 0;
    font-size: 12px;
  }

  .actions {
    display: flex;
    flex: 0 0 auto;
    gap: 4px;
    padding: 6px 8px calc(6px + var(--safe-bottom));
    border-top: 1px solid var(--border);
  }
  .add {
    display: inline-flex;
    flex: 1;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: var(--control-h);
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-muted);
    font-size: 13px;
    cursor: pointer;
  }
  .add:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .add.more {
    flex: 0 0 auto;
    width: var(--control-h);
  }
</style>
