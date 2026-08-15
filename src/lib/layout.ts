import { readable } from 'svelte/store';

/** Below this the side panel becomes a bottom sheet and the chrome shrinks. */
export const COMPACT_BREAKPOINT = 840;

function media(query: string) {
  return readable(typeof matchMedia === 'function' ? matchMedia(query).matches : false, (set) => {
    if (typeof matchMedia !== 'function') return;
    const mq = matchMedia(query);
    const update = () => set(mq.matches);
    mq.addEventListener('change', update);
    update();
    return () => mq.removeEventListener('change', update);
  });
}

/**
 * True when the window is narrow enough to want the touch shell.
 *
 * Deliberately a width query and not a platform check: a phone-sized desktop
 * window gets the same layout, which is the only practical way to work on it
 * without a device in hand.
 */
export const compact = media(`(max-width: ${COMPACT_BREAKPOINT - 1}px)`);

export const coarsePointer = media('(pointer: coarse)');
