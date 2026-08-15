<script lang="ts">
  export let value = '';
  export let label: string | undefined = undefined;
  export let placeholder = '';
  export let disabled = false;
  export let autofocus = false;
</script>

<label class="field">
  {#if label}<span class="label">{label}</span>{/if}
  <!-- svelte-ignore a11y-autofocus -->
  <input
    type="text"
    {placeholder}
    {disabled}
    {autofocus}
    autocomplete="off"
    autocorrect="off"
    autocapitalize="off"
    spellcheck="false"
    bind:value
    on:input
    on:change
    on:keydown
    on:blur
  />
</label>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 2px 0;
  }
  .label {
    font-size: 12px;
    color: var(--text-muted);
  }
  input {
    width: 100%;
    min-height: var(--control-h);
    padding: 0 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-sunken);
    color: var(--text);
    font-size: 13px;
    /* a tap has to land in the text, not be swallowed as a pan */
    touch-action: manipulation;
  }
  input:hover:not(:disabled) {
    border-color: var(--border-strong);
  }
  input:focus {
    border-color: var(--accent);
  }
  input:disabled {
    opacity: 0.45;
  }
</style>
