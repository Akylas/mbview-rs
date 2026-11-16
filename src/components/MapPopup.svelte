<script lang="ts">
  import { Popup, type Map, type MapMouseEvent } from 'maplibre-gl';
  import { onDestroy } from 'svelte';
  import { _ } from 'svelte-i18n';
  import { OWN_PREFIX, type SourceEntry } from '../lib/sources';
  import type { InspectMode } from '../lib/settings';
  import type { Feature } from './Map';
  import PopupProperty from './PopupProperty.svelte';

  export let features: Feature[] = [];
  export let map: Map = null;
  export let sources: SourceEntry[] = [];
  export let mode: InspectMode = 'click';
  /** close the popup as soon as the user moves the camera */
  export let hideOnMove = false;

  /** how far off the pointer a feature can be and still be picked, in px */
  const PICK_RADIUS = 3;

  let main: HTMLDivElement;
  let colors: Record<string, string> = {};
  let hasMultipleSources = false;

  const popup = new Popup({
    closeButton: true,
    closeOnClick: false,
    className: 'map_popup',
  });
  // the outline lives and dies with the popup, however the popup was closed
  popup.on('close', () => select(null));

  /* ---------------------------------------------------------------------- */
  /* selection highlight                                                    */
  /* ---------------------------------------------------------------------- */

  // own-layer prefix: otherwise "show basemap" hides it with the rest of the
  // layers that are not ours, which happens on every settings change
  const HL_SOURCE = `${OWN_PREFIX}highlight`;
  const FALLBACK_COLOR = '#ff2d95';
  const HL_FILL = `${HL_SOURCE}-fill`;
  const HL_LINE = `${HL_SOURCE}-line`;
  const HL_POINT = `${HL_SOURCE}-point`;

  const emptyCollection = { type: 'FeatureCollection', features: [] } as any;

  /**
   * Created on demand rather than once: a basemap or reload rebuilds the style
   * and takes our layers with it. Always moved to the top so layers added
   * since cannot cover it.
   */
  function ensureHighlight(target: Map) {
    if (!target.getSource(HL_SOURCE)) {
      target.addSource(HL_SOURCE, { type: 'geojson', data: emptyCollection });
    }
    if (!target.getLayer(HL_FILL)) {
      target.addLayer({
        id: HL_FILL,
        type: 'fill',
        source: HL_SOURCE,
        filter: ['==', '$type', 'Polygon'],
        paint: { 'fill-color': ['get', 'color'], 'fill-opacity': 0.3 },
      });
      target.addLayer({
        id: HL_LINE,
        type: 'line',
        source: HL_SOURCE,
        filter: ['!=', '$type', 'Point'],
        layout: { 'line-join': 'round', 'line-cap': 'round' },
        paint: { 'line-color': ['get', 'color'], 'line-width': 4, 'line-opacity': 0.9 },
      });
      target.addLayer({
        id: HL_POINT,
        type: 'circle',
        source: HL_SOURCE,
        filter: ['==', '$type', 'Point'],
        paint: {
          'circle-color': ['get', 'color'],
          'circle-radius': 6,
          'circle-stroke-color': '#ffffff',
          'circle-stroke-width': 1.5,
        },
      });
    }
    [HL_FILL, HL_LINE, HL_POINT].forEach((id) => target.moveLayer(id));
  }

  type Pt = [number, number];

  /** distance from `p` to the segment `a`-`b`, all in screen px */
  function segmentDistance(p: Pt, a: Pt, b: Pt) {
    const dx = b[0] - a[0];
    const dy = b[1] - a[1];
    const len = dx * dx + dy * dy;
    const t = len ? Math.max(0, Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len)) : 0;
    return Math.hypot(p[0] - (a[0] + t * dx), p[1] - (a[1] + t * dy));
  }

  function toScreen(coords: number[][]): Pt[] {
    return coords.map((c) => {
      const px = map!.project([c[0], c[1]]);
      return [px.x, px.y] as Pt;
    });
  }

  function lineDistance(p: Pt, coords: number[][]) {
    const pts = toScreen(coords);
    if (pts.length === 1) return Math.hypot(p[0] - pts[0][0], p[1] - pts[0][1]);
    let best = Infinity;
    for (let i = 1; i < pts.length; i++) best = Math.min(best, segmentDistance(p, pts[i - 1], pts[i]));
    return best;
  }

  function insideRing(p: Pt, coords: number[][]) {
    const pts = toScreen(coords);
    let inside = false;
    for (let i = 0, j = pts.length - 1; i < pts.length; j = i++) {
      const [xi, yi] = pts[i];
      const [xj, yj] = pts[j];
      if (yi > p[1] !== yj > p[1] && p[0] < ((xj - xi) * (p[1] - yi)) / (yj - yi) + xi) {
        inside = !inside;
      }
    }
    return inside;
  }

  function polygonDistance(p: Pt, rings: number[][][]) {
    if (insideRing(p, rings[0]) && !rings.slice(1).some((hole) => insideRing(p, hole))) return 0;
    return Math.min(...rings.map((ring) => lineDistance(p, ring)));
  }

  function nearest<T>(parts: T[], distance: (part: T) => number): T {
    let best = parts[0];
    let bestDistance = Infinity;
    parts.forEach((part) => {
      const d = distance(part);
      if (d < bestDistance) {
        bestDistance = d;
        best = part;
      }
    });
    return best;
  }

  /**
   * A tile feature is often a Multi* that merges a whole stretch of network
   * into one object, so outlining it all would light up far more than what
   * was tapped. Keep only the part under the pointer.
   */
  function pickPart(geometry: any, point: Pt) {
    if (!geometry?.coordinates?.length) return geometry;
    switch (geometry.type) {
      case 'MultiPoint':
        return {
          type: 'Point',
          coordinates: nearest(geometry.coordinates as number[][], (c) => lineDistance(point, [c])),
        };
      case 'MultiLineString':
        return {
          type: 'LineString',
          coordinates: nearest(geometry.coordinates as number[][][], (l) => lineDistance(point, l)),
        };
      case 'MultiPolygon':
        return {
          type: 'Polygon',
          coordinates: nearest(geometry.coordinates as number[][][][], (r) =>
            polygonDistance(point, r)
          ),
        };
      default:
        return geometry;
    }
  }

  function setHighlight(items: { geometry: any; color: string }[]) {
    if (!map) return;
    try {
      if (!items.length && !map.getSource(HL_SOURCE)) return;
      ensureHighlight(map);
      (map.getSource(HL_SOURCE) as any).setData({
        type: 'FeatureCollection',
        features: items.map((item) => ({
          type: 'Feature',
          properties: { color: item.color },
          geometry: item.geometry,
        })),
      });
    } catch (error) {
      /* the style may be mid-reload; the next click will redraw it */
    }
  }

  /** index into `features` of the one outlined on the map, if any */
  let selected: number | null = null;
  /** where the map was clicked, in px: picks which part of a Multi* to outline */
  let tapped: Pt = [0, 0];

  function select(index: number | null) {
    selected = index;
    if (index === null || !features[index]) {
      setHighlight([]);
      return;
    }
    const feature = features[index];
    setHighlight([
      {
        geometry: pickPart(feature.geometry, tapped),
        color: colors[feature['sourceLayer']] ?? FALLBACK_COLOR,
      },
    ]);
  }

  function toggle(index: number) {
    // a drag-select of the layer name ends in a click: leave that alone
    if (window.getSelection()?.toString()) return;
    select(selected === index ? null : index);
  }


  $: {
    hasMultipleSources = sources.length > 1;
    colors = sources.reduce((acc, source) => {
      source.layers.forEach((layer) => {
        if (!acc[layer.id]) acc[layer.id] = layer.color;
      });
      return acc;
    }, {} as Record<string, string>);
  }

  /**
   * Only what is actually drawn right now, so a hidden layer cannot answer a
   * query and put a feature in the popup that is nowhere on screen.
   */
  function queryableLayers() {
    const ids: string[] = [];
    sources.forEach((source) => {
      if (!source.visible) return;
      source.layers.forEach((layer) => {
        if (!layer.visible) return;
        ids.push(
          // the polygon outline would double every polygon hit
          ...layer.mapLayers.polygons.filter((id) => !id.endsWith('-outline')),
          ...layer.mapLayers.lines,
          ...layer.mapLayers.points
        );
      });
    });
    return ids.filter((id) => map?.getLayer(id));
  }

  function inspect(event: MapMouseEvent) {
    if (!map || !sources.length) return;
    const box: [[number, number], [number, number]] = [
      [event.point.x - PICK_RADIUS, event.point.y + PICK_RADIUS],
      [event.point.x + PICK_RADIUS, event.point.y - PICK_RADIUS],
    ];

    const layers = queryableLayers();
    const found = layers.length ? map.queryRenderedFeatures(box, { layers }) : [];

    features = (found ?? [])
      .sortBy({ prop: 'sourceLayer' })
      .map((feature) => {
        const source = sources.find((entry) => entry.id === feature['source']);
        feature['sourceName'] = source?.name ?? feature['source'];
        return feature as unknown as Feature;
      });

    if (!features.length) {
      popup.remove();
    } else {
      tapped = [event.point.x, event.point.y];
      // re-adding an open popup fires its `close`, which clears any outline
      // a fresh inspection starts where it was tapped, not where the last one was dragged to
      (popup.options as { anchor?: string }).anchor = undefined;
      offset = [0, 0];
      popup.setOffset(offset);
      popup.setLngLat(event.lngLat).setDOMContent(main).addTo(map);
    }
  }

  /* ---------------------------------------------------------------------- */
  /* dragging the popup                                                     */
  /* ---------------------------------------------------------------------- */

  let dragging: { x: number; y: number; offset: [number, number] } | null = null;
  let offset: [number, number] = [0, 0];

  function dragStart(event: PointerEvent) {
    if (event.button !== 0) return;
    const grip = event.currentTarget as HTMLElement;
    // Freeze the side the popup hangs from. Left to itself maplibre re-picks it
    // as the popup nears an edge, and the box would jump across the pointer.
    const element = grip.closest('.maplibregl-popup');
    const anchor = element?.className.match(/maplibregl-popup-anchor-([a-z-]+)/)?.[1];
    if (anchor) (popup.options as { anchor?: string }).anchor = anchor;
    dragging = { x: event.clientX, y: event.clientY, offset: [...offset] as [number, number] };
    grip.setPointerCapture(event.pointerId);
    event.preventDefault();
  }

  function dragMove(event: PointerEvent) {
    if (!dragging) return;
    offset = [
      dragging.offset[0] + event.clientX - dragging.x,
      dragging.offset[1] + event.clientY - dragging.y,
    ];
    popup.setOffset(offset);
  }

  function dragEnd(event: PointerEvent) {
    if (!dragging) return;
    dragging = null;
    (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
  }

  /* ---------------------------------------------------------------------- */
  /* listener lifecycle                                                     */
  /* ---------------------------------------------------------------------- */

  // The handlers are bound once per map and read `mode` when they fire.
  // Re-registering them from a reactive block — which is what this component
  // used to do — stacked a fresh pair on every render and leaked the old ones.
  let attached: Map | null = null;
  let shiftWasDown = false;

  function onMove(event: MapMouseEvent) {
    if (mode === 'off') return;
    const shift = (event.originalEvent as MouseEvent)?.shiftKey;
    if (mode === 'hover' || shift) {
      shiftWasDown = shift;
      inspect(event);
    } else if (shiftWasDown) {
      // shift released: drop the popup it was holding open
      shiftWasDown = false;
      popup.remove();
    }
  }

  function onClick(event: MapMouseEvent) {
    if (mode === 'click') inspect(event);
  }

  // `originalEvent` is only set when the user caused the move, so a fly-to or a
  // zoom-to-extent from the panel does not close a popup on its own.
  function onMoveStart(event: { originalEvent?: unknown }) {
    if (hideOnMove && event.originalEvent) popup.remove();
  }

  function attach(next: Map | null) {
    if (attached === next) return;
    detach();
    attached = next;
    if (!next) return;
    next.on('mousemove', onMove);
    next.on('click', onClick);
    next.on('movestart', onMoveStart);
  }

  function detach() {
    if (!attached) return;
    attached.off('mousemove', onMove);
    attached.off('click', onClick);
    attached.off('movestart', onMoveStart);
    try {
      popup.remove();
      setHighlight([]);
    } catch (error) {
      /* the map may already be gone */
    }
    attached = null;
  }

  $: attach(map);
  $: if (map?.getCanvas()) {
    map.getCanvas().style.cursor = mode === 'off' ? '' : 'crosshair';
  }
  $: if (mode === 'off') popup.remove();

  onDestroy(detach);
</script>

<!-- The content lives here until maplibre re-parents it into the popup, so the
     host stays hidden and never shows up as stray text over the map. -->
<div class="popup-host">
<div bind:this={main} class="mbview_popup">
  <div
    class="grip"
    title={$_('drag_popup')}
    on:pointerdown={dragStart}
    on:pointermove={dragMove}
    on:pointerup={dragEnd}
    on:pointercancel={dragEnd}
  >
    <span />
  </div>
  {#each features as feature, index}
    {@const layerId = feature['sourceLayer']}
    <div class="feature">
      <div
        class="layer"
        class:active={selected === index}
        role="button"
        tabindex="0"
        title={$_('highlight_feature')}
        on:click={() => toggle(index)}
        on:keydown={(event) => (event.key === 'Enter' || event.key === ' ') && toggle(index)}
      >
        <span class="dot" style:background={colors[layerId] ?? 'var(--text-faint)'} />
        <span class="layer-name selectable">{layerId}</span>
        {#if hasMultipleSources}
          <span class="source-name">{feature['sourceName']}</span>
        {/if}
      </div>
      <div class="props">
        {#if feature.geometry.type === 'Point'}
          <PopupProperty
            name="lat,lon"
            value={`${feature.geometry.coordinates[1]},${feature.geometry.coordinates[0]}`}
          />
        {/if}
        {#if feature.id !== undefined}
          <PopupProperty name="$id" value={feature.id} />
        {/if}
        <PopupProperty name="$type" value={feature.geometry.type} />
        {#if feature.properties}
          {#each Object.entries(feature.properties) as [name, value]}
            <PopupProperty {name} {value} />
          {/each}
        {/if}
      </div>
    </div>
  {/each}
  {#if features.length === 0}
    <div class="feature"><div class="props">{$_('no_features')}</div></div>
  {/if}
</div>
</div>

<style>
  .popup-host {
    display: none;
  }
  .mbview_popup {
    max-height: 44vh;
    overflow-y: auto;
    color: var(--text);
    font-size: 11px;
  }
  .feature {
    padding: 6px 10px;
  }
  .feature:not(:last-child) {
    border-bottom: 1px solid var(--border);
  }
  .layer {
    /* room for the popup's close button */
    padding-right: 18px;
    display: flex;
    align-items: center;
    gap: 5px;
    margin-bottom: 3px;
  }
  .grip {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 12px;
    /* room for the popup's close button */
    padding-right: 24px;
    background: var(--surface);
    cursor: grab;
    touch-action: none;
  }
  .grip:active {
    cursor: grabbing;
  }
  .grip span {
    width: 28px;
    height: 3px;
    border-radius: 2px;
    background: var(--border-strong);
  }
  .grip:hover span {
    background: var(--text-faint);
  }
  .layer {
    cursor: pointer;
  }
  .layer:hover .layer-name,
  .layer.active .layer-name {
    text-decoration: underline;
  }
  .layer.active .dot {
    box-shadow: 0 0 0 2px var(--surface), 0 0 0 3px currentColor;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 2px;
  }
  .layer-name {
    font-weight: 500;
  }
  .source-name {
    color: var(--text-faint);
    font-size: 10px;
  }
  .props {
    display: table;
    width: 100%;
    border-collapse: collapse;
  }
</style>
