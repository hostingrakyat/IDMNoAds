# Sidecar binaries

Tauri bundles the binaries in this folder next to the app executable
(`externalBin` in `tauri.conf.json`). They are **not committed** — CI produces
them on the `windows-latest` runner:

| File (built by CI)                              | Source                                  |
| ----------------------------------------------- | --------------------------------------- |
| `aria2c-x86_64-pc-windows-msvc.exe`             | Downloaded from aria2 1.37.0 release zip |
| `idmnoads-host-x86_64-pc-windows-msvc.exe`      | `cargo build -p idmnoads-host`          |

Tauri installs them into the app directory as `aria2c.exe` and
`idmnoads-host.exe`. To build locally on Windows, drop those two files here with
the target-triple suffix shown above, then run `tauri build`.
