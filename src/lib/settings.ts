import { derived, type Readable } from 'svelte/store';
import { persisted } from './persisted';

export type ThemePreference = 'auto' | 'light' | 'dark';
export type GeometryFilter = 'all' | 'points' | 'lines' | 'polygons';
/** How the attribute popup is summoned. */
export type InspectMode = 'click' | 'hover' | 'off';

export interface Settings {
  theme: ThemePreference;
  /** basemap style name under resources/styles, or `none` for no basemap */
  basemap: string;
  showBackground: boolean;
  inspect: InspectMode;
  showTileBoundaries: boolean;
  showCollisionBoxes: boolean;
  geometryFilter: GeometryFilter;
  /** width of the docked side panel, in px */
  panelWidth: number;
  /** height of the docked feature table, as a percentage of the viewport */
  tableHeight: number;
  showTable: boolean;
  panelOpen: boolean;
  /** most recently opened files, newest first */
  recent: string[];
}

export const DEFAULT_SETTINGS: Settings = {
  theme: 'auto',
  basemap: 'basic',
  showBackground: false,
  inspect: 'click',
  showTileBoundaries: false,
  showCollisionBoxes: false,
  geometryFilter: 'all',
  panelWidth: 300,
  tableHeight: 35,
  showTable: false,
  panelOpen: true,
  recent: [],
};

export const BASEMAPS = ['none', 'basic', 'streets', 'darkmatter', 'terrain'];

export const settings = persisted<Settings>('mbview.settings', DEFAULT_SETTINGS);

const RECENT_MAX = 12;

export function rememberRecent(path: string) {
  settings.update((current) => ({
    ...current,
    recent: [path, ...current.recent.filter((p) => p !== path)].slice(0, RECENT_MAX),
  }));
}

export function forgetRecent(path: string) {
  settings.update((current) => ({
    ...current,
    recent: current.recent.filter((p) => p !== path),
  }));
}

const DARK_QUERY = '(prefers-color-scheme: dark)';

/**
 * What the OS is asking for right now — live rather than read-once, because
 * every desktop OS flips this while the app is open and an app on Auto is
 * expected to follow without a restart.
 */
const systemPrefersDark: Readable<boolean> = {
  subscribe(run) {
    const query = typeof matchMedia === 'function' ? matchMedia(DARK_QUERY) : null;
    run(query ? query.matches : false);
    if (!query) return () => undefined;
    const update = () => run(query.matches);
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  },
};

export const resolvedTheme: Readable<'light' | 'dark'> = derived(
  [settings, systemPrefersDark],
  ([$settings, $dark]) =>
    $settings.theme === 'auto' ? ($dark ? 'dark' : 'light') : $settings.theme,
);

// A live subscription rather than a call from a component: `derived` only
// recomputes while something is listening, and the whole point of Auto is that
// it keeps tracking the system with no screen mounted to watch it.
resolvedTheme.subscribe((theme) => {
  if (typeof document !== 'undefined') {
    document.documentElement.dataset.theme = theme;
  }
});
