import { writable } from 'svelte/store';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';

/** What was just copied, for a moment — drives the "Copied" pill. */
export const lastCopied = writable<string | null>(null);

let timer: ReturnType<typeof setTimeout> | undefined;

/**
 * Copies through the native plugin and falls back to the web API, so a
 * webview that refuses one still gets the other. Returns whether it worked.
 */
export async function copyText(text: string): Promise<boolean> {
  let ok = false;
  try {
    await writeText(text);
    ok = true;
  } catch (error) {
    try {
      await navigator.clipboard.writeText(text);
      ok = true;
    } catch (fallbackError) {
      console.error(error, fallbackError);
    }
  }
  if (ok) {
    lastCopied.set(text);
    clearTimeout(timer);
    timer = setTimeout(() => lastCopied.set(null), 1400);
  }
  return ok;
}
