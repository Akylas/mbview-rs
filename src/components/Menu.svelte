<script lang="ts">
  import {
    Button,
    Checkbox,
    ExpandableTile,
    FormGroup,
    HeaderPanelDivider,
    OrderedList,
    RadioButton,
    RadioButtonGroup,
    Tile,
    Toggle,
  } from 'carbon-components-svelte';
  import Add16 from 'carbon-icons-svelte/lib/Add16';
  import TrashCan16 from 'carbon-icons-svelte/lib/TrashCan16';
  import type { Map } from 'maplibre-gl';
  import { createEventDispatcher } from 'svelte';
  import { _ } from 'svelte-i18n';
  export let mbtiles;
  export let id;
  export let map: Map;
  export let wantPopup;
  export let wantTileBounds;
  export let popupOnClick;
  export let showBackgroundLayer;

  const dispatch = createEventDispatcher();
  // export let drawerOpened;

  let layersVisibility = {};
  let mbtilesVisibility = {};
  function switchSourceLayersVisibility(event, source) {
    const visible = event.target.checked;
    const sid = source.id;
    const layerIds = source.vector_layers?.map((l) => l.id) || [`${source.id}-layer`];
    mbtilesVisibility[sid] = visible;
    console.log('switchSourceLayersVisibility', sid, source, layerIds);
    layerIds.forEach((layerId) => {
      applyLayerVisibility(source, layerId, visible);
    });
  }

  function applyLayerVisibility(source, layerId, sourceVisible) {
    const sId = source.id;
    const layers = source.layers;
    const prefix = `___${sId}___${layerId}`;
    const visible = sourceVisible && (layersVisibility[prefix] ?? true);
    console.log(
      'applyLayerVisibility',
      sId,
      layerId,
      prefix,
      sourceVisible,
      visible,
      layers.rasters
    );
    layers.polygons
      ?.filter((s) => s.startsWith(prefix))
      .forEach((l) => {
        map.setLayoutProperty(l, 'visibility', visible ? showingLayers.polygons : 'none');
      });
    layers.rasters
      ?.filter((s) => s === layerId)
      .forEach((l) => {
        map.setLayoutProperty(l, 'visibility', visible ? 'visible' : 'none');
      });

    layers.lines
      ?.filter((s) => s.startsWith(prefix))
      .forEach((l) => {
        map.setLayoutProperty(l, 'visibility', visible ? showingLayers.lines : 'none');
      });
    layers.points
      ?.filter((s) => s.startsWith(prefix))
      .forEach((l) => {
        map.setLayoutProperty(l, 'visibility', visible ? showingLayers.points : 'none');
      });
  }
  function switchLayerVisibility(event, source, layerId) {
    const sid = source.id;
    const prefix = `___${sid}___${layerId}`;
    layersVisibility[prefix] = event.target.checked;
    applyLayerVisibility(source, layerId, mbtilesVisibility[sid] ?? true);
  }

  let showingLayers = {
    points: 'visible',
    lines: 'visible',
    polygons: 'visible',
  };

  $: {
    if (mbtiles) {
      layersVisibility = {};
      showingLayers = {
        points: 'visible',
        lines: 'visible',
        polygons: 'visible',
      };
    }
  }
  $: {
    if (mbtiles) {
      mbtilesVisibility = {};
    }
  }

  let filter = 'all';
  $: {
    switch (filter) {
      case 'all':
        showingLayers.points = 'visible';
        showingLayers.lines = 'visible';
        showingLayers.polygons = 'visible';
        break;
      case 'points':
        showingLayers.points = 'visible';
        showingLayers.lines = 'none';
        showingLayers.polygons = 'none';
        break;
      case 'lines':
        showingLayers.points = 'none';
        showingLayers.lines = 'visible';
        showingLayers.polygons = 'none';
        break;
      case 'polygons':
        showingLayers.points = 'none';
        showingLayers.lines = 'none';
        showingLayers.polygons = 'visible';
        break;
    }
    mbtiles.forEach((source) => {
      const layers = source.layers;
      Object.keys(layers).forEach((k) => {
        if (k !== 'colors' && k !== 'rasters') {
          paint(k, layers[k], filter);
        }
      });
    });
  }

  function paint(type, layers, val) {
    layers.forEach(function (layerMapId) {
      const layerId = layerMapId.split('-' + type)[0];
      const sId = layerId.split('___')[1];
      const visible = (mbtilesVisibility[sId] ?? true) && (layersVisibility[layerId] ?? true);
      map.setLayoutProperty(
        layerMapId,
        'visibility',
        visible && (val === 'all' || val === type) ? 'visible' : 'none'
      );
    });
  }
  $: {
    if (map) {
      map.showTileBoundaries = wantTileBounds;
    }
  }
  $: {
    const toShow = showBackgroundLayer;
    if (map) {
      Object.keys(map.style._layers)
        .filter((l) => !l.startsWith('___'))
        .forEach((l) => {
          map.setLayoutProperty(l, 'visibility', toShow ? 'visible' : 'none');
        });
    }
  }
  const options = [
    { value: 'all', name: $_('all') },
    { value: 'polygons', name: $_('polygons') },
    { value: 'lines', name: $_('lines') },
    { value: 'points', name: $_('points') },
  ];
</script>

<div class="drawer">
  {#if mbtiles.length > 0}
    <FormGroup legendText={$_('filters')}>
      <RadioButtonGroup bind:selected={filter}>
        {#each options as option}
          <RadioButton
            id={id + '-radio-' + option.value}
            value={option.value}
            labelText={option.name}
          />
        {/each}
      </RadioButtonGroup>
    </FormGroup>

    <HeaderPanelDivider />
    <Checkbox bind:checked={showBackgroundLayer} labelText={$_('show_background_layer')} />
    <Checkbox bind:checked={wantPopup} labelText={$_('show_attribute_popup')} />
    <Checkbox bind:checked={popupOnClick} labelText={$_('show_popup_only_click')} />
  {/if}
  <Checkbox bind:checked={wantTileBounds} labelText={$_('show_tile_boundaries')} />
  <HeaderPanelDivider />
  {#each mbtiles as source}
    <ExpandableTile
      expanded={!!source.vector_layers}
      tileCollapsedLabel={!!source.vector_layers
        ? $_('collapsed_sources')
        : $_('collapsed_options')}
      style="width:100%;margin-bottom:20px"
    >
      <div slot="above">
        <!-- <div style="display:block;flex-grow:1;"> -->
        <h3>{source.name || source.id}</h3>
        <h6 style="word-break: break-all;">{source.path}</h6>
        <!-- </div> -->
        <Toggle
          style="margin-bottom:20px;"
          toggled={true}
          labelA={$_('show_source')}
          labelB={$_('show_source')}
          on:change={(event) => switchSourceLayersVisibility(event, source)}
        />
      </div>
      <div slot="below" style="overflow: hidden;">
        {#if source.vector_layers}
          {#each source.vector_layers.sort( (a, b) => (a.id > b.id ? 1 : b.id > a.id ? -1 : 0) ) as layer}
            <Toggle
              toggled={true}
              labelA={layer.id}
              labelB={layer.id}
              --cds-support-02={source.layers.colors[layer.id]}
              disabled={mbtilesVisibility[source.id] === false}
              on:change={(e) => switchLayerVisibility(e, source, layer.id)}
            />
          {/each}
        {/if}

        <Button
          kind="danger"
          style="margin-top:10px;"
          icon={TrashCan16}
          on:click={() => dispatch('remove_source', source)}>{$_('remove_mbtiles')}</Button
        >
      </div>
    </ExpandableTile>
  {/each}
  <Button icon={Add16} on:click={() => dispatch('add_source')}>{$_('add_mbtiles')}</Button>
</div>
