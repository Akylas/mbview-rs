<script lang="ts">
  export let value: any;
  export let label: string | undefined = undefined;
  export let options: { value: any; label: string }[] = [];
  export let disabled = false;
</script>

<label class="field">
  {#if label}<span class="label">{label}</span>{/if}
  <select bind:value {disabled} on:change>
    {#each options as option (option.value)}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
</label>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    padding: 4px 0;
  }
  .label {
    font-size: 12px;
    color: var(--text-muted);
  }
  select {
    width: 100%;
    min-height: var(--control-h);
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-sunken);
    color: var(--text);
    font-size: 13px;
    touch-action: manipulation;
  }
  select:focus {
    border-color: var(--accent);
  }
  select:disabled {
    opacity: 0.45;
  }
</style>
