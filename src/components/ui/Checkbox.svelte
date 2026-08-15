<script lang="ts">
  export let checked = false;
  export let label: string;
  export let disabled = false;
</script>

<label class="check" class:disabled>
  <input type="checkbox" bind:checked {disabled} on:change />
  <span class="box" aria-hidden="true">
    <svg viewBox="0 0 16 16" width="12" height="12"
      ><path d="M2 8.5 6 12.5 14 4" fill="none" stroke="currentColor" stroke-width="2" /></svg
    >
  </span>
  <span class="label">{label}</span>
</label>

<style>
  .check {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: var(--tap);
    cursor: pointer;
    user-select: none;
  }
  .check.disabled {
    opacity: 0.45;
    cursor: default;
  }
  input {
    position: absolute;
    width: 0;
    height: 0;
    opacity: 0;
  }
  .box {
    display: inline-flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    background: var(--surface-raised);
    color: transparent;
    transition: background 120ms ease, border-color 120ms ease;
  }
  input:checked ~ .box {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
  input:focus-visible ~ .box {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .label {
    font-size: 13px;
    line-height: 1.3;
  }
</style>
