<script lang="ts">
  export let kind: 'primary' | 'secondary' | 'ghost' | 'danger' = 'secondary';
  export let size: 'small' | 'medium' = 'medium';
  export let disabled = false;
  export let type: 'button' | 'submit' = 'button';
  export let title: string | undefined = undefined;
  /** stretch to the row, which is what a sheet's action buttons want */
  export let block = false;
</script>

<button {type} {title} {disabled} class="btn {kind} {size}" class:block on:click>
  <slot />
</button>

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: var(--control-h);
    padding: 0 12px;
    border: 1px solid transparent;
    border-radius: var(--radius);
    background: transparent;
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    cursor: pointer;
    transition: background 120ms ease, border-color 120ms ease;
  }
  .btn.block {
    display: flex;
    width: 100%;
  }
  .btn.small {
    min-height: calc(var(--control-h) - 6px);
    padding: 0 8px;
    font-size: 12px;
  }
  .btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .primary {
    background: var(--accent);
    color: var(--on-accent);
  }
  .primary:not(:disabled):hover {
    background: var(--accent-strong);
  }

  .secondary {
    background: var(--surface-raised);
    border-color: var(--border);
    color: var(--text);
  }
  .secondary:not(:disabled):hover {
    border-color: var(--border-strong);
  }

  .ghost {
    color: var(--text-muted);
  }
  .ghost:not(:disabled):hover {
    background: var(--surface-raised);
    color: var(--text);
  }

  .danger {
    background: transparent;
    border-color: var(--danger);
    color: var(--danger);
  }
  .danger:not(:disabled):hover {
    background: var(--danger-soft);
  }
</style>
