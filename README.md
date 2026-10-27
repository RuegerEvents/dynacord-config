# Dynacord Config

Web GUI for Dynacord amps (tested on a KFIX30:8) over AES70/OCA, as a stand-in for SONICUE on non-Windows machines.

- Power on/standby, level and mute for each channel, status flags
- Finds devices automatically via mDNS (`_oca._tcp`)
- Tauri v2 app. The Rust side runs an axum server that serves the UI and bridges WebSocket to the amp's OCP.1 TCP port (55555). aes70.js handles the protocol in the frontend.

DSP settings and input routing aren't exposed over AES70 on this amp.

## Download

Grab the build for your platform from [Releases](../../releases). On macOS the app is ad-hoc signed only, so clear the quarantine flag once:

```sh
xattr -dr com.apple.quarantine "/Applications/Dynacord Config.app"
```

Allow local network access when macOS asks, otherwise discovery and connections fail.

## Release

Bump `version` in `src-tauri/tauri.conf.json` (and `package.json`, `Cargo.toml`), commit, then push a matching tag (`git tag -s v0.2.0 && git push --tags`). GitHub Actions builds macOS (arm64 + x64), Linux and Windows and publishes the release.

## Run from source

```sh
npm install
npx tauri dev
```

Headless (browser only): `npm run build && cd src-tauri && cargo run --bin dynacord-server`, then open http://127.0.0.1:7070.
`DYNACORD_LAN=1` binds to 0.0.0.0 so phones and tablets can connect; `DYNACORD_PORT` changes the port.
