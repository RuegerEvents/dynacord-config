// Thin wrapper around aes70.js for the Dynacord KFIX30:8 OCA tree.
import { WebSocketConnection, RemoteDevice, observeProperty, Types } from 'aes70';

export type Discovered = {
  name: string;
  hostname: string;
  addresses: string[];
  port: number;
  serial?: string;
};

export async function listDevices(): Promise<Discovered[]> {
  const r = await fetch('/api/devices');
  return r.ok ? r.json() : [];
}

/** Rejects if `p` doesn't settle in time. aes70.js has no per-command timeout. */
export function withTimeout<T>(p: Promise<T>, ms: number, what: string): Promise<T> {
  return new Promise((resolve, reject) => {
    const t = setTimeout(() => reject(new Error(`${what}: no answer after ${ms} ms`)), ms);
    p.then(
      (v) => (clearTimeout(t), resolve(v)),
      (e) => (clearTimeout(t), reject(e))
    );
  });
}

// The KFIX30:8 occasionally drops a response during the burst of requests that
// reading the object tree causes (~1 in 5 attempts, also on plain TCP). A good read
// takes ~250 ms, so give up quickly and retry on a fresh connection.
export async function connect(host: string, port = 55555, onAttempt?: (n: number) => void) {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  const query = `host=${encodeURIComponent(host)}&port=${port}`;
  const url = `${proto}://${location.host}/ws?${query}`;
  let lastError: unknown;
  for (let attempt = 1; attempt <= 8; attempt++) {
    onAttempt?.(attempt);
    const conn = await WebSocketConnection.connect({ url }).catch(async () => {
      // The browser's error Event carries no reason; ask the server for the TCP error.
      const r = await fetch(`/api/probe?${query}`).catch(() => null);
      throw new Error(r ? await r.text() : 'Bridge server not reachable');
    });
    const device = new RemoteDevice(conn);
    try {
      const roles: Map<string, any> = await withTimeout(device.get_role_map('/'), 1500, 'Reading object tree');
      device.set_keepalive_interval(1);
      return { device, roles, conn };
    } catch (e) {
      lastError = e;
      device.close();
    }
  }
  throw lastError;
}

/** Subscribe to a property; callback gets the current value. Returns unsubscribe. */
export function watch(obj: any, prop: string, cb: (v: any) => void): () => void {
  return observeProperty(obj, prop, (ok: boolean, v: any) => {
    if (!ok) return console.warn(prop, v);
    cb(v?.values ? v.values[0] : v);
  });
}

export const MuteState = Types.OcaMuteState;
