<script lang="ts">
  import { _ } from 'svelte-i18n';
  import { BASEMAPS, settings } from '../lib/settings';
  import Overlay from './ui/Overlay.svelte';
  import Section from './ui/Section.svelte';
  import SegmentedControl from './ui/SegmentedControl.svelte';
  import Select from './ui/Select.svelte';
  import Toggle from './ui/Toggle.svelte';

  export let open = false;
  export let onClose: () => void;
  /** changing the basemap has to rebuild the styles, which only App can do */
  export let onBasemap: (name: string) => void;

  let basemap = $settings.basemap;
  $: if (open) basemap = $settings.basemap;

  function pickBasemap(event: Event) {
    onBasemap((event.target as HTMLSelectElement).value);
  }
</script>

<Overlay {open} title={$_('settings')} {onClose}>
  <Section title={$_('appearance')}>
    <SegmentedControl
      bind:value={$settings.theme}
      label={$_('theme')}
      options={[
        { value: 'auto', label: $_('theme_auto') },
        { value: 'light', label: $_('theme_light') },
        { value: 'dark', label: $_('theme_dark') },
      ]}
    />
    <Select
      label={$_('basemap')}
      value={basemap}
      on:change={pickBasemap}
      options={BASEMAPS.map((name) => ({ value: name, label: $_(`basemap_${name}`) }))}
    />
    <Toggle label={$_('show_background_layer')} bind:checked={$settings.showBackground} />
  </Section>

  <Section title={$_('inspection')}>
    <SegmentedControl
      bind:value={$settings.inspect}
      label={$_('show_attribute_popup')}
      options={[
        { value: 'click', label: $_('inspect_click') },
        { value: 'hover', label: $_('inspect_hover') },
        { value: 'off', label: $_('inspect_off') },
      ]}
    />
    <p class="hint">{$_('inspect_help')}</p>
  </Section>

  <Section title={$_('debug')}>
    <Toggle label={$_('show_tile_boundaries')} bind:checked={$settings.showTileBoundaries} />
    <Toggle label={$_('show_collision_boxes')} bind:checked={$settings.showCollisionBoxes} />
  </Section>

  <Section title={$_('shortcuts')}>
    <ul class="keys">
      <li><kbd>L</kbd><span>{$_('layers')}</span></li>
      <li><kbd>S</kbd><span>{$_('opens_split')}</span></li>
      <li><kbd>F</kbd><span>{$_('open_bottom_panel')}</span></li>
      <li><kbd>B</kbd><span>{$_('show_background_layer')}</span></li>
      <li><kbd>T</kbd><span>{$_('show_tile_boundaries')}</span></li>
      <li><kbd>I</kbd><span>{$_('show_attribute_popup')}</span></li>
    </ul>
  </Section>
</Overlay>

<style>
  .hint {
    margin: 2px 0 0;
    color: var(--text-faint);
    font-size: 11px;
  }
  .keys {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
    gap: 4px 12px;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .keys li {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-muted);
    font-size: 12px;
  }
  kbd {
    min-width: 20px;
    padding: 1px 5px;
    border: 1px solid var(--border-strong);
    border-radius: 4px;
    background: var(--surface-sunken);
    font-family: var(--mono);
    font-size: 10px;
    text-align: center;
  }
</style>
