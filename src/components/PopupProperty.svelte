<script lang="ts">
  import Checkmark from 'carbon-icons-svelte/lib/Checkmark.svelte';
  import Copy from 'carbon-icons-svelte/lib/Copy.svelte';
  import { _ } from 'svelte-i18n';
  import { copyText } from '../lib/clipboard';

  export let name: string;
  export let value: any;

  function display(raw: any, propName: string) {
    if (raw === undefined || raw === null) return '—';
    if (propName === '@timestamp' || propName === 'timestamp') {
      const seconds = Number(raw);
      if (!Number.isNaN(seconds)) {
        return `${raw} (${new Date(seconds * 1000).toISOString()})`;
      }
    }
    if (typeof raw === 'object') return JSON.stringify(raw);
    return String(raw);
  }

  let copied = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function copy() {
    const text = `${name}:${typeof value === 'object' && value !== null ? JSON.stringify(value) : value}`;
    if (!(await copyText(text))) return;
    copied = true;
    clearTimeout(timer);
    timer = setTimeout(() => (copied = false), 1200);
  }
</script>

<div class="property">
  <div class="name">{name}</div>
  <div class="value selectable">{display(value, name)}</div>
  <div class="action">
    <button type="button" title={$_('copy')} aria-label={$_('copy')} on:click={copy}>
      {#if copied}<Checkmark size={16} />{:else}<Copy size={16} />{/if}
    </button>
  </div>
</div>

<style>
  .property {
    display: table-row;
  }
  .name {
    display: table-cell;
    padding-right: 10px;
    color: var(--text-faint);
    white-space: nowrap;
  }
  .value {
    display: table-cell;
    max-width: 220px;
    overflow-wrap: anywhere;
    font-family: var(--mono);
  }
  .action {
    display: table-cell;
    width: 1%;
    padding-left: 6px;
    text-align: right;
    vertical-align: middle;
  }
  button {
    display: inline-flex;
    padding: 2px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-faint);
    cursor: pointer;
  }
  button:hover {
    background: var(--surface-raised);
    color: var(--text);
  }
</style>
