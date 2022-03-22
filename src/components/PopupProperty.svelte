<script lang="ts">
  export let name: string;
  export let value: any;
  function displayValue(value, propName) {
    if (propName === '@timestamp') {
      return value.toString() + '<br>[ ' + new Date(value * 1000).toISOString() + ' ]';
    }
    if (value === undefined || value === null) return null;
    if (value.toString)
      return value.toString();
    return value;
  }
</script>

<div class="mbview_property">
  <div class="mbview_property-name">{name}</div>
  <div class="mbview_property-value">
    {displayValue(value, name)}
  </div>
</div>

<style>
  .mbview_property {
    display: table-row;
  }

  .mbview_property-value {
    display: table-cell;
  }

  .mbview_property-name {
    display: table-cell;
    padding-right: 10px;
  }
</style>
