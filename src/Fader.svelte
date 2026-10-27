<script lang="ts">
  // Vertical fader drawn by hand: native vertical range inputs render the fill past the thumb in WebKit.
  type Props = {
    value: number;
    min: number;
    max: number;
    onchange: (db: number) => void;
    ondrag?: (active: boolean) => void;
  };
  let { value, min, max, onchange, ondrag }: Props = $props();

  const THUMB = 36; // px, keep in sync with .thumb height
  // Audio taper: dB → travel. Most of the throw goes to the usable range around 0 dB.
  const TAPER: [number, number][] = [[-80, 0], [-40, 0.15], [-20, 0.4], [-10, 0.6], [0, 0.82], [6, 1]];

  function dbToPos(db: number) {
    db = Math.min(Math.max(db, TAPER[0][0]), TAPER[TAPER.length - 1][0]);
    for (let i = 1; i < TAPER.length; i++) {
      const [d0, p0] = TAPER[i - 1], [d1, p1] = TAPER[i];
      if (db <= d1) return p0 + ((db - d0) / (d1 - d0)) * (p1 - p0);
    }
    return 1;
  }

  function posToDb(pos: number) {
    pos = Math.min(Math.max(pos, 0), 1);
    for (let i = 1; i < TAPER.length; i++) {
      const [d0, p0] = TAPER[i - 1], [d1, p1] = TAPER[i];
      if (pos <= p1) return d0 + ((pos - p0) / (p1 - p0)) * (d1 - d0);
    }
    return TAPER[TAPER.length - 1][0];
  }

  const clamp = (db: number) => Math.min(Math.max(db, min), max);
  const step = (db: number) => clamp(Math.round(db * 2) / 2);

  let el: HTMLDivElement;
  let pos = $derived(dbToPos(clamp(value)));
  let zeroPos = dbToPos(0);

  function fromPointer(e: PointerEvent) {
    const r = el.getBoundingClientRect();
    const p = 1 - (e.clientY - r.top - THUMB / 2) / (r.height - THUMB);
    onchange(step(posToDb(p)));
  }

  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    el.setPointerCapture(e.pointerId);
    ondrag?.(true);
    fromPointer(e);
  }

  function move(e: PointerEvent) {
    if (el.hasPointerCapture(e.pointerId)) fromPointer(e);
  }

  function key(e: KeyboardEvent) {
    const d = { ArrowUp: 0.5, ArrowDown: -0.5, PageUp: 3, PageDown: -3 }[e.key];
    if (d === undefined) return;
    e.preventDefault();
    onchange(step(value + d));
  }

  // Wheel needs a non-passive listener to stop the page from scrolling.
  $effect(() => {
    const wheel = (e: WheelEvent) => {
      e.preventDefault();
      onchange(step(value + (e.deltaY < 0 ? 0.5 : -0.5)));
    };
    el.addEventListener('wheel', wheel, { passive: false });
    return () => el.removeEventListener('wheel', wheel);
  });
</script>

<div
  class="fader"
  bind:this={el}
  style="--pos: {pos}; --zero: {zeroPos}; --thumb: {THUMB}px"
  role="slider"
  tabindex="0"
  aria-valuemin={min}
  aria-valuemax={max}
  aria-valuenow={value}
  onpointerdown={down}
  onpointermove={move}
  onlostpointercapture={() => ondrag?.(false)}
  ondblclick={() => onchange(clamp(0))}
  onkeydown={key}
>
  <div class="track"><div class="fill"></div></div>
  <div class="zero"></div>
  <div class="thumb"></div>
</div>

<style>
  .fader {
    position: relative;
    width: 44px;
    height: 280px;
    touch-action: none;
    cursor: ns-resize;
    outline: none;
  }
  .fader:focus-visible .thumb { box-shadow: 0 0 0 2px #4c9aff; }
  /* Track, fill and thumb share one coordinate: thumb centre = calc(pos * (100% - thumb) + thumb/2). */
  .track {
    position: absolute;
    left: 50%;
    top: calc(var(--thumb) / 2);
    bottom: calc(var(--thumb) / 2);
    width: 6px;
    transform: translateX(-50%);
    background: #33373e;
    border-radius: 3px;
  }
  .fill {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: calc(var(--pos) * 100%);
    background: #4c9aff;
    border-radius: 3px;
  }
  .zero {
    position: absolute;
    left: 8px;
    right: 8px;
    height: 1px;
    bottom: calc(var(--zero) * (100% - var(--thumb)) + var(--thumb) / 2);
    background: #6b717b;
  }
  .thumb {
    position: absolute;
    left: 50%;
    width: 34px;
    height: var(--thumb);
    transform: translateX(-50%);
    bottom: calc(var(--pos) * (100% - var(--thumb)));
    background: linear-gradient(#f2f2f2, #c9c9c9);
    border-radius: 6px;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.6);
  }
  .thumb::after {
    content: '';
    position: absolute;
    left: 5px;
    right: 5px;
    top: 50%;
    height: 2px;
    margin-top: -1px;
    background: #555;
  }
</style>
