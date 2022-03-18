<script lang="ts">
  import { H6 } from '@smui/common/elements';
  import Drawer, { Content, Header, Subtitle, Title } from '@smui/drawer';
  import FormField from '@smui/form-field';
  import List, { Item, Meta, Separator, Subheader } from '@smui/list';
  import Radio from '@smui/radio';
  import Switch from '@smui/switch';
  import { isLoading, _ } from 'svelte-i18n';
  export let layers;
  export let sources;
  export let map;
  export let wantPopup;
  export let wantTileBounds;
  export let popupOnClick;
  export let showBackgroundLayer;
  // export let drawerOpened;
  let menu;

  let layersVisibility = {};
  let sourcesVisibility = {};
  function switchSourceLayersVisibility(event, sid, layerIds) {
    const visible = event.detail.selected;
    sourcesVisibility[sid] = visible;
    Object.keys(layers.colors).forEach((layerId) => {
      // layersVisibility[layerId] =
      //   menu.querySelector(`#show-layer-${layerId}`).checked = visible;
      applyLayerVisibility(layerId, visible);
    });
  }

  function applyLayerVisibility(layerId, sourceVisible) {
    const visible = sourceVisible && (layersVisibility[layerId] ?? true);
    // if (showingLayers.polygons === 'visible') {
    layers.polygons
      .filter((s) => s.startsWith(layerId + '-'))
      .forEach((l) => {
        map.setLayoutProperty(l, 'visibility', visible ? showingLayers.polygons : 'none');
      });
    // }

    // if (showingLayers.lines === 'visible') {
    layers.lines
      .filter((s) => s.startsWith(layerId + '-'))
      .forEach((l) => {
        map.setLayoutProperty(l, 'visibility', visible ? showingLayers.lines : 'none');
      });
    // }
    // if (showingLayers.points === 'visible') {
    layers.points
      .filter((s) => s.startsWith(layerId + '-'))
      .forEach((l) => {
        map.setLayoutProperty(l, 'visibility', visible ? showingLayers.points : 'none');
      });
    // }
  }
  function switchLayerVisibility(event, sid, layerId) {
    layersVisibility[layerId] = event.detail.selected;
    // const visible = menu.querySelector(`#show-layer-${layerId}`).checked;
    applyLayerVisibility(layerId, sourcesVisibility[sid] ?? true);
  }

  //   function updateLayersColors() {
  //     Object.keys(layers.colors).forEach((layerId) => {
  //       menu.querySelector(
  //         `#show-layer-${layerId}`
  //       ).parentElement.style.border = `4px solid ${layers.colors[layerId]}`;
  //     });
  //   }

  let showingLayers = {
    points: 'visible',
    lines: 'visible',
    polygons: 'visible',
  };

  $: {
    if (layers) {
      layersVisibility = {};
      showingLayers = {
        points: 'visible',
        lines: 'visible',
        polygons: 'visible',
      };
    }
  }
  $: {
    if (sources) {
      sourcesVisibility = {};
    }
  }

  let filter = 'all';
  //Menu-Filter Module
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
    Object.keys(layers).forEach((k) => {
      if (k !== 'colors') {
        paint(k, layers[k], filter);
      }
    });
  }

  function paint(type, layers, val) {
    layers.forEach(function (layerMapId) {
      // visibility is val or 'none' if layer is hidden
      const layerId = layerMapId.split('-' + type)[0];
      //   const visible = menu.querySelector(`#show-layer-${layerId}`).checked;
      const visible =
        (sourcesVisibility[sources[0].id] ?? true) && (layersVisibility[layerId] ?? true);
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
  // $: {
  //   console.log('sources', sources);
  // }

  const options = [
    { value: 'all', name: $_('all') },
    { value: 'polygons', name: $_('polygons') },
    { value: 'lines', name: $_('lines') },
    { value: 'points', name: $_('points') },
  ];
</script>
<Drawer class="drawer" variant="dismissible" fixed={true} open={true}>
  <Content class="drawer-content" bind:this={menu}>
    {#if sources.length > 0}
      <Subheader component={H6}>{$_('filters')}</Subheader>
      {#each options as option}
        <FormField>
          <Radio bind:group={filter} value={option.value} />
          <span slot="label">
            {option.name}
          </span>
        </FormField>
      {/each}
      <Separator />
      <FormField>
        <Switch bind:checked={showBackgroundLayer} />
        <span slot="label">{$_('show_background_layer')}</span>
      </FormField>
      <FormField>
        <Switch bind:checked={wantPopup} />
        <span slot="label">{$_('show_attribute_popup')}</span>
      </FormField>
      <FormField>
        <Switch bind:checked={popupOnClick} />
        <span slot="label">{$_('show_popup_only_click')}</span>
      </FormField>
    {/if}
    <FormField>
      <Switch bind:checked={wantTileBounds} />
      <span slot="label">{$_('show_tile_boundaries')}</span>
    </FormField>
    <Separator />
    {#each sources as source}
      <Item style="margin:0px;height:80px;">
        <Header>
          <Title>{source.name}</Title>
          <Subtitle>{source.id}</Subtitle>
        </Header>
        <Meta>
          <Switch
            checked={true}
            on:SMUISwitch:change={(event) =>
              switchSourceLayersVisibility(
                event,
                source.id,
                source.vector_layers.map((l) => "'" + l.id + "'")
              )}
          />
        </Meta>
      </Item>
      <Content>
        <List>
          {#each source.vector_layers.sort( (a, b) => (a.id > b.id ? 1 : b.id > a.id ? -1 : 0) ) as layer}
            <Item>
              <FormField>
                <Switch
                  class="colored-switch"
                  checked={true}
                  --mdc-switch-selected-track-color={layers.colors[layer.id]}
                  --mdc-switch-hover-track-color={layers.colors[layer.id]}
                  --mdc-switch-handle-surface-color={layers.colors[layer.id]}
                  --mdc-switch-selected-handle-color={layers.colors[layer.id]}
                  --mdc-switch-hover-handle-color={layers.colors[layer.id]}
                  on:SMUISwitch:change={(e) => switchLayerVisibility(e, source.id, layer.id)}
                />
                <span slot="label">{layer.id}</span>
              </FormField>
            </Item>
          {/each}
        </List>
      </Content>
    {/each}
  </Content>
</Drawer>

<style>
</style>
