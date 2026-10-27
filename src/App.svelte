<script lang="ts">
  import { onMount } from 'svelte';
  import { listDevices, connect, watch, MuteState, type Discovered } from './amp';

  type Channel = { n: number; gain: number; min: number; max: number; muted: boolean; dragging: boolean; level: any; mute: any };

  let devices = $state<Discovered[]>([]);
  let manualHost = $state('');
  let status = $state<'idle' | 'connecting' | 'connected' | 'error'>('idle');
  let errorMsg = $state('');
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
      const { device, roles, conn } = await connect(host, port);
      session = { device, conn };
      conn.on('close', () => {
        if (session?.conn === conn) {
          status = 'error';
          errorMsg = 'Connection lost';
        }
      });

      const dm = device.DeviceManager;
      const [md, name, serial] = await Promise.all([dm.GetModelDescription(), dm.GetDeviceName(), dm.GetSerialNumber()]);
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
        const r = await ch.level.GetGain();
        ch.min = r.values[1];
        ch.max = r.values[2];
        cleanups.push(watch(ch.level, 'Gain', (v) => { if (!ch.dragging) ch.gain = v; }));
        if (ch.mute) cleanups.push(watch(ch.mute, 'State', (v) => (ch.muted = v?.value === MuteState.Muted.value)));
      }
      status = 'connected';
    } catch (e: any) {
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

  function setGain(ch: Channel, v: number) {
    ch.gain = v;
    ch.level.SetGain(v).catch((e: any) => console.warn(e));
  }

  async function toggleMute(ch: Channel) {
    await ch.mute.SetState(ch.muted ? MuteState.Unmuted : MuteState.Muted);
  }

  const fmt = (g: number) => (g <= -80 ? '-∞' : g.toFixed(1));
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
    {#if status === 'connecting'}<p>Connecting & reading object tree…</p>{/if}
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
        <div class="val">{fmt(ch.gain)} dB</div>
        <input
          class="fader"
          type="range"
          min={ch.min}
          max={ch.max}
          step="0.5"
          value={ch.gain}
          onpointerdown={() => (ch.dragging = true)}
          onpointerup={() => (ch.dragging = false)}
          oninput={(e) => setGain(ch, +e.currentTarget.value)}
          ondblclick={() => setGain(ch, 0)}
        />
        <button class="mute" class:active={ch.muted} onclick={() => toggleMute(ch)}>MUTE</button>
      </div>
    {/each}
  </section>
  <p class="muted hint">Double-click a fader for 0 dB. DSP (EQ, limiter, delay, presets) isn't exposed over AES70 on this amp.</p>
{/if}
