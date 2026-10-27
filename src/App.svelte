<script lang="ts">
  import { onMount } from 'svelte';
  import { listDevices, connect, watch, withTimeout, MuteState, type Discovered } from './amp';
  import Fader from './Fader.svelte';
  import DbInput from './DbInput.svelte';

  type Channel = { n: number; gain: number; min: number; max: number; muted: boolean; dragging: boolean; level: any; mute: any };

  let devices = $state<Discovered[]>([]);
  let manualHost = $state('');
  let status = $state<'idle' | 'connecting' | 'connected' | 'error'>('idle');
  let errorMsg = $state('');
  let step = $state('');
  let info = $state<{ name: string; model: string; version: string; serial: string } | null>(null);
  let power = $state<boolean | null>(null);
  let flags = $state<Record<string, boolean | null>>({ ready: null, powerState: null, error: null });
  let channels = $state<Channel[]>([]);

  let session: { device: any; conn: any } | null = null;
  let cleanups: (() => void)[] = [];
  let powerObj: any = null;

  onMount(() => {
    const poll = async () => (devices = await listDevices().catch(() => []));
    poll();
    const t = setInterval(poll, 3000);
    return () => clearInterval(t);
  });

  function disconnect() {
    cleanups.forEach((c) => c());
    cleanups = [];
    session?.device.close();
    session = null;
    channels = [];
    info = null;
    power = null;
    status = 'idle';
  }

  async function open(host: string, port = 55555) {
    disconnect();
    status = 'connecting';
    errorMsg = '';
    try {
      const { device, roles, conn } = await connect(host, port, (n) => (step = n > 1 ? `retry ${n}` : ''));
      session = { device, conn };
      const lost = (msg: string) => {
        if (session?.conn !== conn) return;
        disconnect();
        status = 'error';
        errorMsg = msg;
      };
      conn.on('close', () => lost('Connection lost'));
      conn.on('error', (e: any) => lost(`Connection error: ${e?.message ?? e}`));

      const dm = device.DeviceManager;
      const [md, name, serial] = await withTimeout(Promise.all([dm.GetModelDescription(), dm.GetDeviceName(), dm.GetSerialNumber()]), 3000, 'Device info');
      info = { name, model: `${md.Manufacturer} ${md.Name}`, version: md.Version, serial };

      powerObj = roles.get('device/power');
      if (powerObj) cleanups.push(watch(powerObj, 'Setting', (v) => (power = v)));
      for (const k of ['ready', 'powerState', 'error']) {
        const o = roles.get(`device/${k}`);
        if (o) cleanups.push(watch(o, 'Reading', (v) => (flags[k] = v)));
      }

      const chs: Channel[] = [];
      for (let n = 1; roles.has(`device/channel/${n}/level`); n++) {
        chs.push({ n, gain: 0, min: -80, max: 6, muted: false, dragging: false, level: roles.get(`device/channel/${n}/level`), mute: roles.get(`device/channel/${n}/mute`) });
      }
      channels = chs;
      for (const ch of channels) {
        const r: any = await withTimeout(ch.level.GetGain(), 3000, `Channel ${ch.n}`);
        ch.min = r.values[1];
        ch.max = r.values[2];
        cleanups.push(watch(ch.level, 'Gain', (v) => { if (!ch.dragging) ch.gain = v; }));
        if (ch.mute) cleanups.push(watch(ch.mute, 'State', (v) => (ch.muted = v?.value === MuteState.Muted.value)));
      }
      status = 'connected';
    } catch (e: any) {
      disconnect();
      status = 'error';
      errorMsg = e?.message ?? String(e) ?? 'Connection failed';
    }
  }

  // window.confirm() is a no-op in the Tauri webview, so standby needs a second click instead.
  let armStandby = $state(false);
  let armTimer: ReturnType<typeof setTimeout> | undefined;

  async function setPower(on: boolean) {
    if (!powerObj) return;
    if (!on && !armStandby) {
      armStandby = true;
      clearTimeout(armTimer);
      armTimer = setTimeout(() => (armStandby = false), 3000);
      return;
    }
    armStandby = false;
    await powerObj.SetSetting(on).catch((e: any) => (errorMsg = `Power: ${e?.message ?? e}`));
  }

  // Fader moves arrive faster than the amp needs them; send only the latest value per channel per frame.
  const pendingGain = new Map<Channel, number>();
  let flushQueued = false;

  function setGain(ch: Channel, v: number) {
    ch.gain = v;
    pendingGain.set(ch, v);
    if (flushQueued) return;
    flushQueued = true;
    requestAnimationFrame(() => {
      flushQueued = false;
      for (const [c, g] of pendingGain) c.level.SetGain(g).catch((e: any) => console.warn(e));
      pendingGain.clear();
    });
  }

  async function setMute(ch: Channel, muted: boolean) {
    await ch.mute.SetState(muted ? MuteState.Muted : MuteState.Unmuted);
  }

  // "All" strip: absolute level for every channel. Shows the common value, or "—" when they differ.
  let allGain = $derived(channels.length && channels.every((c) => c.gain === channels[0].gain) ? channels[0].gain : null);
  let allAvg = $derived(channels.reduce((a, c) => a + c.gain, 0) / (channels.length || 1));
  let allMuted = $derived(channels.length > 0 && channels.every((c) => c.muted));
  let allMin = $derived(Math.max(...channels.map((c) => c.min)));
  let allMax = $derived(Math.min(...channels.map((c) => c.max)));
</script>

<header>
  <h1>Dynacord Config</h1>
  {#if status === 'connected' && info}
    <span class="pill ok">{info.name}</span>
    <span class="muted">{info.model} · fw {info.version} · SN {info.serial}</span>
    <button class="ghost" onclick={disconnect}>Disconnect</button>
  {/if}
</header>

{#if status !== 'connected'}
  <section class="picker">
    <h2>Devices</h2>
    {#if devices.length === 0}<p class="muted">Searching for AES70 devices (_oca._tcp)…</p>{/if}
    {#each devices as d}
      <button class="device" onclick={() => open(d.addresses[0] ?? d.hostname, d.port)} disabled={status === 'connecting'}>
        <strong>{d.name}</strong>
        <span class="muted">{d.addresses.join(', ')}:{d.port} · {d.hostname}</span>
      </button>
    {/each}
    <form onsubmit={(e) => { e.preventDefault(); if (manualHost) open(manualHost); }}>
      <input placeholder="IP address" bind:value={manualHost} />
      <button type="submit" disabled={status === 'connecting'}>Connect</button>
    </form>
    {#if status === 'connecting'}<p>Connecting & reading object tree…{step ? ` (${step})` : ''}</p>{/if}
    {#if status === 'error'}<p class="err">{errorMsg}</p>{/if}
  </section>
{:else}
  <section class="top">
    <div class="power">
      <span>Power</span>
      <button class="big" class:on={power} class:armed={armStandby} onclick={() => setPower(!power)}>
        {armStandby ? 'Click again → standby' : power ? 'ON' : 'STANDBY'}
      </button>
      <span class="muted">reported: {flags.powerState === null ? '?' : flags.powerState ? 'on' : 'off'}</span>
    </div>
    {#if errorMsg}<span class="err">{errorMsg}</span>{/if}
    <div class="flags">
      {#each Object.entries(flags) as [k, v]}
        <span class="flag" class:lit={v} class:bad={k === 'error' && v}>{k}</span>
      {/each}
    </div>
  </section>

  <section class="strips">
    {#each channels as ch (ch.n)}
      <div class="strip" class:muted-strip={ch.muted}>
        <div class="label">Ch {ch.n}</div>
        <DbInput value={ch.gain} min={ch.min} max={ch.max} onchange={(v) => setGain(ch, v)} />
        <Fader value={ch.gain} min={ch.min} max={ch.max} onchange={(v) => setGain(ch, v)} ondrag={(a) => (ch.dragging = a)} />
        <button class="mute" class:active={ch.muted} onclick={() => setMute(ch, !ch.muted)}>MUTE</button>
      </div>
    {/each}
    {#if channels.length}
      <div class="strip all" class:muted-strip={allMuted}>
        <div class="label">All</div>
        <DbInput value={allGain} min={allMin} max={allMax} onchange={(v) => channels.forEach((c) => setGain(c, v))} />
        <Fader
          value={allGain ?? allAvg}
          min={allMin}
          max={allMax}
          onchange={(v) => channels.forEach((c) => setGain(c, v))}
          ondrag={(a) => channels.forEach((c) => (c.dragging = a))}
        />
        <button class="mute" class:active={allMuted} onclick={() => channels.forEach((c) => setMute(c, !allMuted))}>MUTE ALL</button>
      </div>
    {/if}
  </section>
  <p class="muted hint">
    Drag, scroll or use arrow keys on a fader; double-click for 0 dB. Click a dB value to type one (<code>-inf</code> = off).
    "All" sets every channel to the same level. DSP (EQ, limiter, delay, presets) isn't exposed over AES70 on this amp.
  </p>
{/if}
