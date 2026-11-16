<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { isRemote } from '../lib/sources';
  import Button from './ui/Button.svelte';
  import Overlay from './ui/Overlay.svelte';
  import TextField from './ui/TextField.svelte';

  export let open = false;
  /** the address is being reached: the dialog stays up so a failure can be corrected */
  export let busy = false;
  export let onClose: () => void;
  export let onOpen: (url: string) => void;

  let url = '';
  let touched = false;

  $: trimmed = url.trim();
  $: valid = isRemote(trimmed);
  $: if (!open) touched = false;

  function submit() {
    touched = true;
    if (valid && !busy) onOpen(trimmed);
  }
</script>

<Overlay {open} title={$_('open_url')} {onClose}>
  <form on:submit|preventDefault={submit}>
    <TextField
      label={$_('url')}
      placeholder="https://tiles.openfreemap.org/planet"
      autofocus
      bind:value={url}
      disabled={busy}
    />
    {#if touched && !valid}
      <p class="error">{$_('open_url_invalid')}</p>
    {/if}
    <p class="hint">{$_('open_url_help')}</p>
    <div class="actions">
      <Button kind="ghost" on:click={onClose}>{$_('cancel')}</Button>
      <Button kind="primary" type="submit" disabled={busy || !trimmed}>
        {busy ? $_('connecting') : $_('open')}
      </Button>
    </div>
  </form>
</Overlay>

<style>
  .hint,
  .error {
    margin: 8px 0 0;
    font-size: 12px;
    line-height: 1.5;
  }
  .hint {
    color: var(--text-muted);
  }
  .error {
    color: var(--danger);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }
</style>
