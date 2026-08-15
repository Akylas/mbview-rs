<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  /**
   * A drag handle between two panes. Reports pointer deltas in px and leaves
   * the sizing itself to the caller, which is the only thing that knows what
   * the pane's limits are.
   *
   * Pointer events rather than mouse events, so the same handle works under a
   * finger; capture keeps the drag alive when the pointer outruns the bar.
   */
  export let orientation: 'vertical' | 'horizontal' = 'vertical';
  export let label = 'Resize';

  const dispatch = createEventDispatcher<{ move: number; done: void }>();
  let dragging = false;
  let last = 0;

  function down(event: PointerEvent) {
    dragging = true;
    last = orientation === 'vertical' ? event.clientX : event.clientY;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    event.preventDefault();
  }

  function move(event: PointerEvent) {
    if (!dragging) return;
    const current = orientation === 'vertical' ? event.clientX : event.clientY;
    dispatch('move', current - last);
    last = current;
  }

  function up(event: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
    dispatch('done');
  }
</script>

<div
  class="resizer {orientation}"
  class:dragging
  role="separator"
  aria-label={label}
  aria-orientation={orientation}
  on:pointerdown={down}
  on:pointermove={move}
  on:pointerup={up}
  on:pointercancel={up}
/>

<style>
  .resizer {
    position: relative;
    z-index: 5;
    flex: 0 0 auto;
    background: var(--border);
    touch-action: none;
  }
  .resizer::after {
    /* the bar reads as a hairline but grabs like a real target */
    content: '';
    position: absolute;
    inset: -4px;
  }
  .vertical {
    width: 1px;
    cursor: col-resize;
  }
  .horizontal {
    height: 1px;
    cursor: row-resize;
  }
  .resizer:hover,
  .dragging {
    background: var(--accent);
  }
</style>
