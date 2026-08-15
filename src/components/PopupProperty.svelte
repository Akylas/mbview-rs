<script lang="ts">
  export let name: string;
  export let value: any;

  function display(raw: any, propName: string) {
    if (raw === undefined || raw === null) return '—';
    if (propName === '@timestamp' || propName === 'timestamp') {
      const seconds = Number(raw);
      if (!Number.isNaN(seconds)) {
        return `${raw} (${new Date(seconds * 1000).toISOString()})`;
      }
    }
    if (typeof raw === 'object') return JSON.stringify(raw);
    return String(raw);
  }
</script>

<div class="property">
  <div class="name">{name}</div>
  <div class="value">{display(value, name)}</div>
</div>

<style>
  .property {
    display: table-row;
  }
  .name {
    display: table-cell;
    padding-right: 10px;
    color: var(--text-faint);
    white-space: nowrap;
  }
  .value {
    display: table-cell;
    max-width: 220px;
    overflow-wrap: anywhere;
    font-family: var(--mono);
  }
</style>
