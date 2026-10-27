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

export async function connect(host: string, port = 55555) {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  const url = `${proto}://${location.host}/ws?host=${encodeURIComponent(host)}&port=${port}`;
  const conn = await WebSocketConnection.connect({ url });
  const device = new RemoteDevice(conn);
  device.set_keepalive_interval(1);
  const roles: Map<string, any> = await device.get_role_map('/');
  return { device, roles, conn };
}

/** Subscribe to a property; callback gets the current value. Returns unsubscribe. */
export function watch(obj: any, prop: string, cb: (v: any) => void): () => void {
  return observeProperty(obj, prop, (ok: boolean, v: any) => {
    if (!ok) return console.warn(prop, v);
    cb(v?.values ? v.values[0] : v);
  });
}

export const MuteState = Types.OcaMuteState;
