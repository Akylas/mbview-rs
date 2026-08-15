<script lang="ts">
  import { onDestroy } from 'svelte';
  import Menu from './ui/Menu.svelte';

  /** The element whose right-click (or long-press) raises this menu. */
  export let target: HTMLElement | null = null;

  let open = false;
  let x = 0;
  let y = 0;
  let attached: HTMLElement | null = null;
  /** where the menu was opened, so an action can act on that point */
  export let point: { x: number; y: number } = { x: 0, y: 0 };

  function onContextMenu(event: MouseEvent) {
    event.preventDefault();
    const rect = attached.getBoundingClientRect();
    point = { x: event.clientX - rect.left, y: event.clientY - rect.top };
    x = event.clientX;
    y = event.clientY;
    open = true;
  }

  function attach(next: HTMLElement | null) {
    if (attached === next) return;
    detach();
    attached = next;
    attached?.addEventListener('contextmenu', onContextMenu);
  }

  function detach() {
    attached?.removeEventListener('contextmenu', onContextMenu);
    attached = null;
  }

  $: attach(target);
  onDestroy(detach);
</script>

<Menu bind:open {x} {y} onClose={() => (open = false)}>
  <slot />
</Menu>
