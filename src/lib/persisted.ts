import { writable, type Writable } from 'svelte/store';

/**
 * A writable that mirrors itself into localStorage.
 *
 * Objects are merged over the defaults on read rather than replacing them, so
 * a settings key added in a later version still has a value for someone whose
 * stored blob predates it.
 */
export function persisted<T>(key: string, initial: T): Writable<T> {
  let start = initial;
  try {
    const raw = localStorage.getItem(key);
    if (raw != null) {
      const parsed = JSON.parse(raw);
      start =
        parsed && typeof parsed === 'object' && !Array.isArray(parsed)
          ? { ...(initial as any), ...parsed }
          : (parsed as T);
    }
  } catch (error) {
    /* unreadable storage just means defaults */
  }

  const store = writable<T>(start);
  store.subscribe((value) => {
    try {
      localStorage.setItem(key, JSON.stringify(value));
    } catch (error) {
      /* private mode / quota — the app still works, it just forgets */
    }
  });
  return store;
}

/** Read a persisted blob without subscribing to it. */
export function readJSON<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw == null ? fallback : (JSON.parse(raw) as T);
  } catch (error) {
    return fallback;
  }
}

export function writeJSON(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch (error) {
    /* see above */
  }
}
