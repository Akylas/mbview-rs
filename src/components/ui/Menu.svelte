<script lang="ts">
  import { tick } from 'svelte';

  /**
   * A floating menu positioned at a point, kept inside the viewport.
   *
   * Deliberately unaware of what opened it: a right-click on the map and a
   * click on a toolbar button both just hand it a coordinate.
   */
  export let open = false;
  export let x = 0;
  export let y = 0;
  /** anchor the menu's right edge to `x` instead of its left */
  export let alignRight = false;
  export let onClose: () => void = () => undefined;

  let ref: HTMLElement;
  let left = 0;
  let top = 0;

  $: if (open) position(x, y, ref);

  async function position(nextX: number, nextY: number, node: HTMLElement) {
    await tick();
    if (!node) return;
    const { width, height } = node.getBoundingClientRect();
    left = alignRight ? nextX - width : nextX;
    top = nextY;
    // keep it on screen whichever edge it was opened near
    left = Math.max(4, Math.min(left, window.innerWidth - width - 4));
    top = Math.max(4, Math.min(top, window.innerHeight - height - 4));
  }
</script>

{#if open}
  <!-- a transparent catcher rather than a window listener: it swallows the
       click that dismisses the menu, so it cannot also fall through to the map -->
  <div
    class="catcher"
    role="presentation"
    on:pointerdown|self={onClose}
    on:contextmenu|preventDefault|self={onClose}
  />
  <!-- svelte-ignore a11y-click-events-have-key-events -->
  <!-- a click anywhere in the list dismisses it; the items themselves are
       buttons, so the keyboard path already works without a handler here -->
  <ul
    bind:this={ref}
    class="menu"
    role="menu"
    tabindex="-1"
    style="left:{left}px; top:{top}px"
    on:click={onClose}
  >
    <slot />
  </ul>
{/if}

<svelte:window
  on:keydown={(event) => {
    if (open && event.key === 'Escape') onClose();
  }}
/>

<style>
  .catcher {
    position: fixed;
    inset: 0;
    z-index: 80;
  }
  .menu {
    position: fixed;
    z-index: 81;
    min-width: 180px;
    max-width: 90vw;
    margin: 0;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: var(--shadow);
    list-style: none;
  }
</style>
