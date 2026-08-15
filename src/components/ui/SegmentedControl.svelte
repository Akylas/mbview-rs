<script lang="ts">
  /**
   * A small set of mutually exclusive choices, all visible at once.
   *
   * Preferred over a Select for two- to four-way settings: on touch it is one
   * tap instead of a native picker sheet, and the alternatives stay readable
   * rather than hidden behind the current value.
   */
  export let value: any;
  export let label: string | undefined = undefined;
  export let options: { value: any; label: string }[] = [];
</script>

<div class="field">
  {#if label}<span class="label">{label}</span>{/if}
  <div class="segments" role="radiogroup" aria-label={label}>
    {#each options as option (option.value)}
      <button
        type="button"
        class="segment"
        class:active={value === option.value}
        role="radio"
        aria-checked={value === option.value}
        on:click={() => (value = option.value)}
      >
        {option.label}
      </button>
    {/each}
  </div>
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 4px 0;
  }
  .label {
    font-size: 12px;
    color: var(--text-muted);
  }
  .segments {
    display: flex;
    gap: 2px;
    padding: 2px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-sunken);
  }
  .segment {
    flex: 1;
    min-width: 0;
    min-height: calc(var(--control-h) - 6px);
    padding: 0 8px;
    border: none;
    border-radius: calc(var(--radius) - 3px);
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease;
  }
  .segment:hover:not(.active) {
    color: var(--text);
  }
  .segment.active {
    background: var(--accent);
    color: var(--on-accent);
  }
</style>
