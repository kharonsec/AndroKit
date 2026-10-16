# AndroKit

A cross-platform desktop GUI front-end for **ADB** and **Fastboot**, built with
Rust and [egui](https://github.com/emilk/egui). AndroKit wraps the standard
Android platform-tools in a single window: detect a connected device, flash
images, back up and restore partitions, transfer files, stream logs, and
inspect boot/DTBO images — without memorising command-line flags.

> **AndroKit drives `adb` and `fastboot`. It does not bypass device locks,
> carrier locks, FRP, or anti-theft protections.** Use it only on devices you
> own or are explicitly authorised to service. See [Legal & ethical use](#legal--ethical-use).

## Features

- **Dashboard** — live device detection (ADB / recovery / sideload / fastboot),
  manufacturer/model/serial readout, and per-vendor warranty warnings.
- **Flash** — unlock/lock bootloader (with confirmation), flash boot/recovery
  images, optional Magisk patching, ADB sideload, and flash profiles.
- **Backup & Restore** — pull partitions to disk (optionally AES-256-GCM
  encrypted), restore `.img`/`.img.enc` via fastboot, read-only **forensic
  dump** with a SHA-256 manifest, and quick/secure wipe (both confirmed).
- **Files** — ADB push/pull with configurable remote paths.
- **Advanced** — live `logcat`, `dmesg` capture, boot-image header analyzer,
  DTBO header viewer, and a read-only partition hex viewer (rooted devices).
- **Protocols** — run arbitrary `adb`/`fastboot` commands; optional
  informational exploit/bypass reference database (hidden in compliance mode).
- **Settings** — Wi-Fi ADB connect/disconnect, compliance mode, Docker file
  generation, and a tamper-evident (hash-chained) action audit log.
- **Console** — a persistent, filterable log panel showing all command and
  stream output.

## Requirements

- `adb` and `fastboot` (Android platform-tools) on your `PATH`.
- Optional: `magiskboot` on your `PATH` for the Magisk patch feature.
- Linux GUI dependencies (Debian/Ubuntu):
  ```bash
  sudo apt install -y libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
      libxkbcommon-dev libssl-dev libgtk-3-dev libudev-dev
  ```

## Building

```bash
cargo build --release
```

The binary is written to `target/release/androkit`.

## Running

Place `devices.yaml` (and optionally `blocklist.yaml`, `warranty_warnings.yaml`,
`exploits.yaml`) next to the binary, then:

```bash
./target/release/androkit
```

### Configuration & data files

| File                     | Purpose                                                            |
| ------------------------ | ------------------------------------------------------------------ |
| `devices.yaml`           | Per-device unlock/lock commands, partition names, warnings.        |
| `warranty_warnings.yaml` | Vendor-specific warranty notices shown on the dashboard.           |
| `blocklist.yaml`         | Filename patterns of known malware; matches raise a console warning. |
| `exploits.yaml`          | Informational reference entries shown on the Protocols tab.        |

User state is stored under `~/.androidtool/`:

- `settings.json` — persisted preferences (Wi-Fi ADB, compliance mode, backup options).
- `backups/<serial>/<timestamp>/` — partition backups.
- `profiles/` — saved flash profiles.
- `action_log.json` — the hash-chained audit log.

## Testing

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

These are the same checks enforced by CI (`.github/workflows/ci.yml`), which
also builds release binaries for Linux, Windows, and macOS.

## Legal & ethical use

AndroKit is intended for device owners, repair technicians, developers, and
researchers operating on hardware they own or are authorised to modify.
Flashing firmware, unlocking bootloaders, and wiping storage are destructive
and can permanently brick a device or void its warranty. You are responsible
for ensuring you have the legal right to perform any operation on a given
device. Enable **compliance mode** in Settings to hide the informational
exploit reference database.

## License

Licensed under the **GNU General Public License v3.0 or later**. See
[`LICENSE`](LICENSE).
