<script lang="ts">
  import ChevronDown from 'carbon-icons-svelte/lib/ChevronDown.svelte';
  import Close from 'carbon-icons-svelte/lib/Close.svelte';
  import Copy from 'carbon-icons-svelte/lib/Copy.svelte';
  import Search from 'carbon-icons-svelte/lib/Search.svelte';
  import { _ } from 'svelte-i18n';
  import { highlightJSON } from '../lib/json';
  import type { Feature } from './Map';
  import IconButton from './ui/IconButton.svelte';

  export let features: Feature[] = [];
  export let onClose: () => void;
  export let onCopy: (text: string) => void;

  /** rows past this are not rendered; the count still reports the truth */
  const ROW_CAP = 500;

  let search = '';
  let sortKey: string | null = null;
  let sortDesc = false;
  let expanded: Record<string, boolean> = {};

  interface Row {
    key: string;
    layer: string;
    source: string;
    type: string;
    id: any;
    properties: Record<string, any>;
    feature: Feature;
  }

  $: rows = features.map((feature, index) => ({
    key: `${feature['source'] ?? ''}/${feature['sourceLayer'] ?? ''}/${feature.id ?? index}`,
    layer: feature['sourceLayer'] ?? '',
    source: feature['sourceName'] ?? '',
    type: feature.geometry?.type ?? '',
    id: feature.id,
    properties: (feature.properties ?? {}) as Record<string, any>,
    feature,
  })) as Row[];

  $: propertyKeys = Array.from(
    rows.reduce((acc, row) => {
      Object.keys(row.properties).forEach((key) => acc.add(key));
      return acc;
    }, new Set<string>())
  ).sort((a, b) => a.localeCompare(b));

  $: columns = [
    { key: '$layer', label: 'layer' },
    { key: '$type', label: 'type' },
    { key: '$id', label: 'id' },
    ...propertyKeys.map((key) => ({ key, label: key })),
  ];

  function cell(row: Row, key: string) {
    if (key === '$layer') return row.layer;
    if (key === '$type') return row.type;
    if (key === '$id') return row.id;
    return row.properties[key];
  }

  $: filtered = search
    ? rows.filter((row) => {
        const needle = search.toLowerCase();
        if (row.layer.toLowerCase().includes(needle)) return true;
        return Object.entries(row.properties).some(
          ([key, value]) =>
            key.toLowerCase().includes(needle) ||
            String(value ?? '').toLowerCase().includes(needle)
        );
      })
    : rows;

  $: sorted = sortKey
    ? [...filtered].sort((a, b) => {
        const left = cell(a, sortKey);
        const right = cell(b, sortKey);
        if (left === right) return 0;
        if (left === undefined || left === null) return 1;
        if (right === undefined || right === null) return -1;
        const result =
          typeof left === 'number' && typeof right === 'number'
            ? left - right
            : String(left).localeCompare(String(right), undefined, { numeric: true });
        return sortDesc ? -result : result;
      })
    : filtered;

  $: shown = sorted.slice(0, ROW_CAP);

  function sortBy(key: string) {
    if (sortKey === key) {
      sortDesc = !sortDesc;
    } else {
      sortKey = key;
      sortDesc = false;
    }
  }

  function copyAll() {
    onCopy(
      JSON.stringify(
        { type: 'FeatureCollection', features: sorted.map((row) => row.feature) },
        (_key, value) => (typeof value === 'bigint' ? value.toString() : value),
        2
      )
    );
  }
</script>

<div class="table-panel">
  <header>
    <div class="search">
      <Search size={16} />
      <input
        type="text"
        placeholder={$_('filter_features')}
        autocomplete="off"
        spellcheck="false"
        bind:value={search}
      />
    </div>
    <span class="count">
      {$_('n_features', { values: { count: sorted.length } })}
      {#if sorted.length > ROW_CAP}<em>· {$_('showing_first', { values: { count: ROW_CAP } })}</em>{/if}
    </span>
    <IconButton icon={Copy} label={$_('copy_geojson')} size={16} on:click={copyAll} />
    <IconButton icon={Close} label={$_('close')} size={16} on:click={onClose} />
  </header>

  <div class="scroll mb-scroll selectable">
    {#if shown.length === 0}
      <p class="empty">{$_('no_features_selected')}</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th class="expander" />
            {#each columns as column (column.key)}
              <th>
                <button type="button" on:click={() => sortBy(column.key)}>
                  {column.label}
                  {#if sortKey === column.key}<span class="arrow" class:desc={sortDesc}>▲</span>{/if}
                </button>
              </th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each shown as row (row.key)}
            <tr class:open={expanded[row.key]}>
              <td class="expander">
                <button
                  type="button"
                  class="chevron"
                  class:open={expanded[row.key]}
                  aria-label={$_('expand_row')}
                  on:click={() => (expanded[row.key] = !expanded[row.key])}
                >
                  <ChevronDown size={16} />
                </button>
              </td>
              {#each columns as column (column.key)}
                <td>{cell(row, column.key) ?? ''}</td>
              {/each}
            </tr>
            {#if expanded[row.key]}
              <tr class="detail">
                <td colspan={columns.length + 1}>
                  <pre>{@html highlightJSON(row.feature)}</pre>
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>

<style>
  .table-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--surface);
  }
  header {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    gap: 8px;
    padding: 4px 6px;
    border-bottom: 1px solid var(--border);
  }
  .search {
    display: flex;
    flex: 0 1 240px;
    align-items: center;
    gap: 6px;
    min-height: calc(var(--control-h) - 4px);
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-sunken);
    color: var(--text-faint);
  }
  .search input {
    width: 100%;
    border: none;
    background: transparent;
    color: var(--text);
    font-size: 12px;
  }
  .count {
    flex: 1;
    color: var(--text-faint);
    font-size: 11px;
  }
  .count em {
    font-style: normal;
    color: var(--warning-text);
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
  .empty {
    padding: 24px;
    color: var(--text-faint);
    font-size: 12px;
    text-align: center;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 11px;
  }
  th {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 0;
    border-bottom: 1px solid var(--border);
    background: var(--surface-raised);
    text-align: left;
    white-space: nowrap;
  }
  th button {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    padding: 5px 8px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
  }
  th button:hover {
    color: var(--text);
  }
  .arrow {
    font-size: 8px;
  }
  .arrow.desc {
    transform: rotate(180deg);
  }
  td {
    max-width: 260px;
    padding: 3px 8px;
    border-bottom: 1px solid var(--border);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  tbody tr:hover td {
    background: var(--surface-raised);
  }
  .expander {
    width: 26px;
    padding: 0;
  }
  .chevron {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 22px;
    border: none;
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
    transform: rotate(-90deg);
    transition: transform 120ms ease;
  }
  .chevron.open {
    transform: rotate(0deg);
  }
  .detail td {
    max-width: none;
    padding: 0;
    background: var(--surface-sunken);
    white-space: normal;
  }
  pre {
    max-height: 320px;
    margin: 0;
    overflow: auto;
    padding: 8px 12px;
    font-family: var(--mono);
    font-size: 11px;
    line-height: 1.45;
  }
  pre :global(.tok-key) {
    color: var(--accent-text);
  }
  pre :global(.tok-string) {
    color: var(--success-text);
  }
  pre :global(.tok-number) {
    color: var(--warning-text);
  }
  pre :global(.tok-boolean),
  pre :global(.tok-null) {
    color: var(--danger);
  }
</style>
