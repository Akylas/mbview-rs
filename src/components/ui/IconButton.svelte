<script lang="ts">
  /** the carbon icon component to render */
  export let icon: any;
  export let label: string;
  export let active = false;
  export let disabled = false;
  export let size = 18;
  /** shrink to fit a dense list row rather than a toolbar */
  export let compact = false;
  export let danger = false;
</script>

<button
  type="button"
  class="icon-btn"
  class:active
  class:compact
  class:danger
  {disabled}
  aria-label={label}
  title={label}
  aria-pressed={active}
  on:click
>
  <svelte:component this={icon} {size} />
</button>

<style>
  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 auto;
    width: var(--tap);
    height: var(--tap);
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-muted);
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }
  .icon-btn.compact {
    width: calc(var(--tap) - 6px);
    height: calc(var(--tap) - 6px);
  }
  .icon-btn:not(:disabled):hover {
    background: var(--surface-raised);
    color: var(--text);
  }
  .icon-btn.active {
    color: var(--accent);
    background: var(--accent-soft);
  }
  .icon-btn.danger:not(:disabled):hover {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .icon-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }
</style>
