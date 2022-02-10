<script lang="ts">
  export let layers;
  export let map;
  export let wantPopup;
  export let wantTileBounds;
  let menuBtn;
  let menu;
  function switchSourceLayersVisibility(sid, layerIds) {
    const visible = menu.querySelector(`#show-source-${sid.replace(/\./g, '-')}`).checked;
    Object.keys(layers.colors).forEach((layerId) => {
      menu.querySelector(`#show-layer-${layerId}`).checked = visible;
      applyLayerVisibility(layerId, visible);
    });
  }

  function applyLayerVisibility(layerId, visible) {
    if (showingLayers.polygons === 'visible') {
      layers.polygons
        .filter((s) => s.startsWith(layerId + '-'))
        .forEach((l) => {
          map.setLayoutProperty(l, 'visibility', visible ? 'visible' : 'none');
        });
    }

    if (showingLayers.lines === 'visible') {
      layers.lines
        .filter((s) => s.startsWith(layerId + '-'))
        .forEach((l) => {
          map.setLayoutProperty(l, 'visibility', visible ? 'visible' : 'none');
        });
    }
    if (showingLayers.pts === 'visible') {
      layers.pts
        .filter((s) => s.startsWith(layerId + '-'))
        .forEach((l) => {
          map.setLayoutProperty(l, 'visibility', visible ? 'visible' : 'none');
        });
    }
  }
  function switchLayerVisibility(layerId) {
    const visible = menu.querySelector(`#show-layer-${layerId}`).checked;
    applyLayerVisibility(layerId, visible);
  }

  function updateLayersColors() {
    Object.keys(layers.colors).forEach((layerId) => {
      menu.querySelector(
        `#show-layer-${layerId}`
      ).parentElement.style.border = `4px solid ${layers.colors[layerId]}`;
    });
  }

  menuBtn.addEventListener(
    'click',
    function () {
      popup.remove();
      if (menuBtn.className.indexOf('active') > -1) {
        //Hide Menu
        menuBtn.className = '';
        menu.style.display = 'none';
      } else {
        //Show Menu
        menuBtn.className = 'active';
        menu.style.display = 'block';
      }
    },
    false
  );

  let showingLayers = {
    pts: 'visible',
    lines: 'visible',
    polygons: 'visible',
  };

  //Menu-Filter Module
  function menuFilter() {
    if (document.querySelector('#filter-all').checked) {
      showingLayers.pts = 'visible';
      showingLayers.lines = 'visible';
      showingLayers.polygons = 'visible';
    } else if (document.querySelector('#filter-pts').checked) {
      showingLayers.pts = 'visible';
      showingLayers.lines = 'none';
      showingLayers.polygons = 'none';
    } else if (document.querySelector('#filter-lines').checked) {
      showingLayers.pts = 'none';
      showingLayers.lines = 'visible';
      showingLayers.polygons = 'none';
    } else if (document.querySelector('#filter-polygons').checked) {
      showingLayers.pts = 'none';
      showingLayers.lines = 'none';
      showingLayers.polygons = 'visible';
    }
    Object.keys(showingLayers).forEach((k) => {
      paint(k, layers[k], showingLayers[k]);
    });

    function paint(type, layers, val) {
      layers.forEach(function (layerMapId) {
        // visibility is val or 'none' if layer is hidden
        const layerId = layerMapId.split('-' + type)[0];
        const visible = menu.querySelector(`#show-layer-${layerId}`).checked;
        map.setLayoutProperty(layerMapId, 'visibility', visible ? val : 'none');
      });
    }
  }

  function menuPopup(event) {
    wantPopup = document.querySelector('#show-popup').checked;
  }

  function menuTiles() {
    wantTileBounds = document.querySelector('#show-tiles').checked;
    map.showTileBoundaries = wantTileBounds;
  }
</script>

<div>
  <div id="menu" bind:this={menuBtn}><span class="icon menu big" /></div>

  <div id="menu-container" bind:this={menu}>
    <h4>Filter</h4>
    <div id="menu-filter" on:change={() => menuFilter()} class="rounded-toggle short inline">
      <input id="filter-all" type="radio" name="rtoggle" value="all" checked="{false}" />
      <label for="filter-all">all</label>
      <input id="filter-polygons" type="radio" name="rtoggle" value="polygons" />
      <label for="filter-polygons">polygons</label>
      <input id="filter-lines" type="radio" name="rtoggle" value="lines" />
      <label for="filter-lines">lines</label>
      <input id="filter-pts" type="radio" name="rtoggle" value="pts" />
      <label for="filter-pts">points</label>
    </div>
    <h4>Popup</h4>
    <div on:change={menuPopup} class="rounded-toggle short inline">
      <input id="show-popup" type="checkbox" name="ptoggle" checked="{true}" />
      <label for="show-popup">show attributes</label>
    </div>
    <h4>Tiles</h4>
    <div on:change={menuTiles} class="rounded-toggle short inline">
      <input id="show-tiles" type="checkbox" name="ttoggle" checked="{true}" />
      <label for="show-tiles">show tile boundaries</label>
    </div>
    <!-- <% Object.keys(sources).forEach(sid=>renderSource(sid, sources[sid])) %> -->
  </div>
</div>

<style>
    #menu {
      position: absolute;
      top:10px;
      right:10px;
      z-index: 1;
      color: white;
      cursor: pointer;
    }
    #menu-container {
      position: absolute;
      display: none;
      top: 50px;
      right: 10px;
      max-height: 90%;
      z-index: 1;
      background-color: white;
      overflow: auto;
      padding: 20px;
    }
    .icon.big                     { line-height:40px; }
    .icon:not(.big):before        { margin-right:5px; }
    .icon:empty:before            { margin:0; }
    .icon.menu:before {
      content: "\e964";
    }
    
    menu {
      margin:0;
      padding:0;
      border:0;
      font-size:100%;
      font:inherit;
      vertical-align:baseline;
      display:block;
      } */
</style>
