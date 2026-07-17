# Lid Awake

Lid Awake is a compact macOS menu bar app that lets you control lid-close sleep behavior with a simple ON/OFF switch.

Turn Lid Awake **ON** to keep your Mac awake when the lid is closed. Turn it **OFF** to restore the normal macOS sleep behavior. The app keeps the current system setting visible and synchronized, so there is no need to use Terminal or remember `pmset` commands.

It provides:

- A single, easy-to-use ON/OFF switch
- A compact 200 × 100 px menu bar window
- Automatic state synchronization every three seconds
- Immediate refresh when the window is shown or focused
- Standard macOS administrator authorization only when changing the setting
- No privileged mode, daemon, background helper, login item, or stored credentials

## Technology stack

- [Tauri 2](https://tauri.app/) for the native macOS window, menu bar integration, and Rust backend
- [Svelte 5](https://svelte.dev/) and TypeScript for the user interface
- [Vite 8](https://vite.dev/) for frontend development and builds
- Rust for reading and changing the macOS power setting

Svelte and the Tauri CLI are project-local npm dependencies. You do not need to install either one globally.

## Requirements

- macOS 11 or later
- [Node.js](https://nodejs.org/) and npm
- [Rust](https://www.rust-lang.org/tools/install) and Cargo
- Xcode Command Line Tools (`xcode-select --install`)
- An administrator account to approve changes to the system sleep setting

## Development

Install the dependencies and start the Tauri development app:

```bash
npm install
npm run check
npm run tauri dev
```

To work on the interface in a browser with preview data:

```bash
npm run dev
```

Run the frontend and Rust checks before packaging:

```bash
npm test
npm run check
npm run build
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

## Installation from source

Build the release app and copy it to the Applications folder:

```bash
npm install
npm run tauri build -- --bundles app
ditto "src-tauri/target/release/bundle/macos/Lid Awake.app" \
  "/Applications/Lid Awake.app"
open "/Applications/Lid Awake.app"
```

Quit an existing copy of Lid Awake before replacing it.

## How it works

Lid Awake reads the system-wide `SleepDisabled` value from `/usr/bin/pmset -g`.
On macOS versions that omit this line until `disablesleep` has been enabled,
the missing line is treated as the normal sleep state (`SleepDisabled=0`).

- **ON** sets `SleepDisabled=1`, keeping the Mac awake when the lid is closed.
- **OFF** sets `SleepDisabled=0`, restoring the normal macOS sleep behavior.

When you toggle the switch, the Rust backend starts `/usr/bin/osascript` with one of two fixed AppleScript commands. macOS handles administrator authorization and runs `/usr/bin/pmset -a disablesleep 0|1`. The interface updates only after reading the actual system state again; it does not assume that a change succeeded.

## Security and privacy

Lid Awake is designed to keep its privileged surface as small as possible. The app itself always runs as a normal, unprivileged menu bar app. It does not install or use a LaunchDaemon, LaunchAgent, privileged helper, `setuid` executable, login item, or persistent administrator mode.

Each setting change is submitted separately through the standard macOS authorization flow. Lid Awake never asks for, reads, stores, or logs your password or other authentication credentials. It invokes only fixed commands with absolute system paths and does not accept arbitrary shell commands.

The app makes no network requests and contains no analytics or telemetry. The three-second background task only reads the local macOS power setting and stops when the app quits.

## Important behavior

`SleepDisabled` is a system-wide macOS setting and remains in its last selected state after Lid Awake quits or is removed. Turn Lid Awake **OFF** before uninstalling it if you want to restore the normal lid-close sleep behavior.
