<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { highlightJSON } from '../lib/json';
  import type { SourceEntry } from '../lib/sources';
  import Overlay from './ui/Overlay.svelte';

  export let source: SourceEntry | null = null;
  export let onClose: () => void;

  $: rows = source
    ? [
        [$_('file_path'), source.path],
        [$_('name'), source.name],
        [$_('format'), source.vector ? `${source.format ?? 'vector'} (${source.encoding})` : source.format ?? 'raster'],
        [$_('zoom_range'), `${source.minzoom ?? '?'} – ${source.maxzoom ?? '?'}`],
        [$_('bounds'), source.bounds ? source.bounds.map((n) => n.toFixed(5)).join(', ') : '—'],
        [$_('center'), source.center ? source.center.join(', ') : '—'],
        [$_('layers'), source.vector ? String(source.layers.length) : '—'],
        [$_('attribution'), source.attribution || '—'],
        [$_('description'), source.description || '—'],
        [$_('tiles'), source.tiles[0] ?? '—'],
      ]
    : [];
</script>

<Overlay open={!!source} title={$_('source_info')} {onClose}>
  {#if source}
    <dl class="facts selectable">
      {#each rows as [label, value]}
        <dt>{label}</dt>
        <dd>{value}</dd>
      {/each}
    </dl>
    <h3>tiles.json</h3>
    <pre class="selectable">{@html highlightJSON(source.data)}</pre>
  {/if}
</Overlay>

<style>
  .facts {
    display: grid;
    grid-template-columns: minmax(90px, auto) 1fr;
    gap: 4px 12px;
    margin: 8px 0 16px;
    font-size: 12px;
  }
  dt {
    color: var(--text-faint);
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
    font-family: var(--mono);
    font-size: 11px;
  }
  h3 {
    margin: 0 0 6px;
    color: var(--text-faint);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  pre {
    max-height: 320px;
    margin: 0;
    padding: 8px 10px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-sunken);
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
