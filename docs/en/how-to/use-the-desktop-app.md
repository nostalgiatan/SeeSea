# Use the desktop app

The desktop app is a Tauri program in `desktop/`: a window, a tray icon and a global shortcut around the [workbench](use-the-workbench.md). It starts the same `seesea` program, so the command line and the desktop app share data and settings.

## Run it

Build `seesea` first, then build and run the desktop app:

```bash
cargo build --release --manifest-path seesea-core/Cargo.toml -p seesea-cli
cd desktop
SEESEA_BIN=../seesea-core/target/release/seesea cargo run --release
```

On Linux you need the development packages of `webkit2gtk-4.1` and `gtk3`; for other systems see the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

The app looks for `seesea` in this order: the environment variable `SEESEA_BIN`, a `seesea` in the same directory as the app, the `PATH`. If it finds none, the window says so.

## Day to day

- Closing the window only sends it to the tray; the HTTP API and MCP services started from the workbench keep running.
- A left click on the tray icon shows or hides the window; the right-click menu has Show, Open in browser and Quit.
- `Ctrl Shift Space` (`Cmd Shift Space` on macOS) brings the window up with the search box focused, and a second press hides it. If the system does not allow registering that shortcut, only that one feature is missing.
- Only one desktop app runs at a time; starting another brings the existing window to the front.
- Links in results open in the default browser.

On Quit the app first lets `seesea` close the services it started, then ends. If the app is killed hard, `seesea` still receives a stop signal on Linux; macOS and Windows do not have that protection yet.

## Package it

```bash
cargo install tauri-cli --locked   # once
desktop/scripts/bundle.sh
```

The script builds a release `seesea`, adds it to the package as a sidecar, and makes the installers for the current system.

## Security

The app starts `seesea desk` on a free port of `127.0.0.1`, a different one each time. To Tauri the page in the window is an external page: it is granted no permission and cannot call the app's functions.

Addresses other than the workbench's own do not open in the window; they go to the default browser.
