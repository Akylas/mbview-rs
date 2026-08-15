<script lang="ts">
  /**
   * The touch shell's panel: a bottom sheet with two detents, dragged by its
   * grabber. It sits over the map rather than pushing it, so the map keeps the
   * whole screen and the sheet only ever covers part of it.
   */
  export let open = false;
  export let title: string;
  export let detent: 'half' | 'full' = 'half';
  export let onClose: () => void;

  let dragging = false;
  let startY = 0;
  let startHeight = 0;
  let height = 0;

  const HEIGHTS = { half: 0.45, full: 0.88 };

  $: if (!dragging) height = window.innerHeight * HEIGHTS[detent];

  function down(event: PointerEvent) {
    dragging = true;
    startY = event.clientY;
    startHeight = height;
    // otherwise the drag also starts a text selection in the sheet's content
    event.preventDefault();
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function move(event: PointerEvent) {
    if (!dragging) return;
    height = Math.max(80, Math.min(window.innerHeight * 0.95, startHeight - (event.clientY - startY)));
  }

  function up(event: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
    // snap to whichever detent the drag ended nearest, or dismiss if pulled down
    const ratio = height / window.innerHeight;
    if (ratio < 0.18) {
      onClose();
      detent = 'half';
    } else {
      detent = ratio > (HEIGHTS.half + HEIGHTS.full) / 2 ? 'full' : 'half';
    }
    height = window.innerHeight * HEIGHTS[detent];
  }
</script>

{#if open}
  <div class="sheet" style="height:{height}px" class:dragging>
    <div
      class="grabber"
      role="separator"
      aria-label={title}
      on:pointerdown={down}
      on:pointermove={move}
      on:pointerup={up}
      on:pointercancel={up}
    >
      <span class="bar" />
    </div>
    <div class="body mb-scroll"><slot /></div>
  </div>
{/if}

<style>
  .sheet {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    z-index: 40;
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--border);
    border-radius: var(--radius-lg) var(--radius-lg) 0 0;
    background: var(--surface);
    box-shadow: var(--shadow);
    transition: height 160ms ease;
  }
  .sheet.dragging {
    transition: none;
  }
  .grabber {
    display: flex;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    height: 24px;
    touch-action: none;
    cursor: grab;
  }
  .bar {
    width: 36px;
    height: 4px;
    border-radius: 2px;
    background: var(--border-strong);
  }
  .body {
    flex: 1;
    min-height: 0;
    padding: 0 12px calc(12px + var(--safe-bottom));
  }
</style>
