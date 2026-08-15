<script lang="ts">
  import ChevronDown from 'carbon-icons-svelte/lib/ChevronDown.svelte';

  export let title: string;
  export let open = true;
  /** headings that only ever carry one control read better without a chevron */
  export let collapsible = true;
</script>

<section class="section" class:open>
  {#if collapsible}
    <button type="button" class="head" aria-expanded={open} on:click={() => (open = !open)}>
      <span>{title}</span>
      <span class="chevron"><ChevronDown size={16} /></span>
    </button>
  {:else}
    <div class="head static"><span>{title}</span></div>
  {/if}
  {#if open}
    <div class="body"><slot /></div>
  {/if}
</section>

<style>
  .section {
    border-top: 1px solid var(--border);
  }
  .section:first-child {
    border-top: none;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    min-height: var(--tap);
    padding: 0;
    border: none;
    background: transparent;
    color: var(--text-faint);
    font-size: 11px;
    font-weight: 500;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    cursor: pointer;
  }
  .head.static {
    cursor: default;
  }
  .chevron {
    display: inline-flex;
    transform: rotate(-90deg);
    transition: transform 140ms ease;
  }
  .open .chevron {
    transform: rotate(0deg);
  }
  .body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding-bottom: 10px;
  }
</style>
