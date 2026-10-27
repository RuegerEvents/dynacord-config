<script lang="ts">
  // dB readout that turns into a text field on focus. Enter commits, Escape reverts.
  type Props = {
    value: number | null; // null = mixed (e.g. the "All" strip when channels differ)
    min: number;
    max: number;
    onchange: (db: number) => void;
  };
  let { value, min, max, onchange }: Props = $props();

  let editing = $state(false);
  let text = $state('');
  let cancelled = false;

  const fmt = (g: number | null) => (g === null ? '—' : g <= min ? '-∞' : g.toFixed(1));

  function parse(s: string): number | null {
    s = s.trim().toLowerCase().replace(/db$/, '').replace(',', '.').replace('−', '-').trim();
    if (['-∞', '-inf', 'inf', 'off', '∞'].includes(s)) return min;
    const n = Number(s);
    if (s === '' || !Number.isFinite(n)) return null;
    return Math.min(Math.max(Math.round(n * 10) / 10, min), max);
  }

  function commit() {
    editing = false;
    if (cancelled) return void (cancelled = false);
    const db = parse(text);
    if (db !== null) onchange(db);
  }
</script>

<label class="db" class:editing>
  <input
    value={editing ? text : fmt(value)}
    inputmode="decimal"
    spellcheck="false"
    onfocus={(e) => {
      editing = true;
      text = value === null ? '' : value.toFixed(1);
      queueMicrotask(() => e.currentTarget.select());
    }}
    oninput={(e) => (text = e.currentTarget.value)}
    onblur={(e) => {
      commit();
      // Svelte skips the DOM write when the bound string is unchanged, so reset the text by hand.
      e.currentTarget.value = fmt(value);
    }}
    onkeydown={(e) => {
      if (e.key === 'Enter') e.currentTarget.blur();
      if (e.key === 'Escape') {
        cancelled = true;
        e.currentTarget.blur();
      }
    }}
  />
  <span>dB</span>
</label>

<style>
  .db {
    display: flex;
    align-items: baseline;
    gap: 3px;
    font-size: 13px;
    color: #b9c0ca;
  }
  input {
    width: 48px;
    padding: 3px 4px;
    text-align: right;
    font: inherit;
    font-variant-numeric: tabular-nums;
    color: inherit;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 4px;
    cursor: text;
  }
  input:hover { border-color: #3a3f48; }
  .editing input { background: #1e2126; border-color: #4c9aff; color: #fff; }
  span { color: #6b717b; font-size: 11px; }
</style>
