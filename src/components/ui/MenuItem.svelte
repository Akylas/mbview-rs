<script lang="ts">
  export let label: string;
  export let icon: any = undefined;
  export let disabled = false;
  export let danger = false;
  export let shortcut: string | undefined = undefined;
  export let checked: boolean | undefined = undefined;
</script>

<li role="none">
  <button
    type="button"
    role="menuitem"
    class="item"
    class:danger
    class:checked
    {disabled}
    on:click
  >
    <span class="icon">
      {#if icon}<svelte:component this={icon} size={16} />{/if}
      {#if checked}<span class="dot" />{/if}
    </span>
    <span class="label">{label}</span>
    {#if shortcut}<span class="shortcut">{shortcut}</span>{/if}
  </button>
</li>

<style>
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    min-height: var(--tap);
    padding: 0 8px;
    border: none;
    border-radius: calc(var(--radius) - 4px);
    background: transparent;
    color: var(--text);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
  }
  .item:not(:disabled):hover {
    background: var(--surface-raised);
  }
  .item:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .item.danger {
    color: var(--danger);
  }
  .item.danger:not(:disabled):hover {
    background: var(--danger-soft);
  }
  .icon {
    display: inline-flex;
    flex: 0 0 16px;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }
  .label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .shortcut {
    color: var(--text-faint);
    font-family: var(--mono);
    font-size: 11px;
  }
</style>
