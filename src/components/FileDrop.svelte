<script lang="ts">
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import type { PhysicalPosition } from '@tauri-apps/api/dpi';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import { onDestroy } from 'svelte';

  export let extensions: string[] | null = null;
  export let handleFiles: (paths: string[]) => void = () => {};
  export let handleOneFile: (path: string) => void = () => {};
  export let files: string[] = [];
  export let position: PhysicalPosition | null = null;

  function getValidPaths(paths: string[]) {
    if (extensions === null) {
      return paths;
    }
    return paths.filter((path) => extensions.some((ext) => path.endsWith('.' + ext)));
  }

  // `onDragDropEvent` replaces the beta-era `tauri://drag` / `tauri://drop`
  // events. Guarded so `vite dev` in a plain browser — where there is no
  // webview to ask — still renders the app instead of dying on import.
  const unlistening: Promise<UnlistenFn> | null = tryListen();

  function tryListen(): Promise<UnlistenFn> | null {
    try {
      return listenForDrops();
    } catch (error) {
      console.warn('drag and drop unavailable outside the app shell', error);
      return null;
    }
  }

  function listenForDrops() {
    return getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;
      switch (payload.type) {
        case 'enter':
          position = payload.position;
          files = getValidPaths(payload.paths);
          break;
        case 'over':
          position = payload.position;
          break;
        case 'drop': {
          const dropped = getValidPaths(payload.paths);
          files = dropped;
          if (dropped.length > 0) {
            handleFiles(dropped);
          }
          if (payload.paths.length === 1 && dropped.length === 1) {
            handleOneFile(dropped[0]);
          }
          files = [];
          position = null;
          break;
        }
        default:
          files = [];
          position = null;
          break;
      }
    });
  }

  onDestroy(async () => {
    (await unlistening)?.();
  });
</script>

<slot {files} {position} />
