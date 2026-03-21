<p align="center">
  <img src="apps/desktop/src-tauri/icons/icon.png" alt="Prismatic" width="128" height="128">
</p>

<h1 align="center">Prismatic</h1>

<p align="center">
  A cross-platform game save editor for 10+ engines.<br>
  Edit party stats, inventory, variables, switches, and raw save data through a unified interface.
</p>

<p align="center">
  <a href="https://github.com/Patrick-Batenburg/prismatic/actions/workflows/ci.yml"><img src="https://github.com/Patrick-Batenburg/prismatic/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/Patrick-Batenburg/prismatic/actions/workflows/build.yml"><img src="https://github.com/Patrick-Batenburg/prismatic/actions/workflows/build.yml/badge.svg" alt="Build"></a>
  <a href="https://github.com/Patrick-Batenburg/prismatic/releases/latest"><img src="https://img.shields.io/github/v/release/Patrick-Batenburg/prismatic?label=release" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/Patrick-Batenburg/prismatic" alt="License"></a>
</p>

---

## Supported Engines

| Engine | Extensions | Debug Patches |
|--------|-----------|:---:|
| RPG Maker MV/MZ | `.rpgsave`, `.rmmzsave` | Yes |
| RPG Maker VX Ace | `.rvdata2` | Yes |
| Ren'Py | `.save` | Yes |
| Wolf RPG Editor | `.sav` | Yes |
| Unity | `.json`, `.xml`, `.es3` | |
| Unreal Engine | `.sav` | |
| Pixel Game Maker MV | `.json` | |
| Flash / AIR | `.sol` | |
| SugarCube / Twine 2 | `.save` | |
| SQLite | `.db`, `.sqlite`, `.sqlite3` | |

## Features

- **Structured editing** — party, inventory, currency, variables, switches
- **Raw JSON editor** — edit any value in the save data directly
- **Save comparison** — diff two saves with patience diff algorithm
- **Debug patches** — inject in-game debug consoles for supported engines
- **File watching** — auto-detect save changes
- **Backup manager** — automatic backups before writes
- **Cross-platform** — Windows and Linux with platform-specific save directory hints

## Tech Stack

- **Frontend:** Svelte 5, SvelteKit, TypeScript, Vite
- **Backend:** Rust, Tauri v2
- **Build:** pnpm, Turborepo
- **Testing:** Vitest (frontend), `cargo test` (backend)

## Development

```bash
pnpm install

# Run the desktop app with hot-reload
cd apps/desktop
pnpm tauri dev

# Lint
pnpm lint

# Test
pnpm test
cd src-tauri && cargo test
```

## Building

```bash
cd apps/desktop

# Windows (MSI + NSIS installer)
pnpm tauri build --bundles msi
pnpm tauri build --bundles nsis

# Linux
pnpm tauri build --bundles deb
pnpm tauri build --bundles rpm
pnpm tauri build --bundles appimage
```

## Project Structure

```
prismatic/
├── apps/desktop/           # Tauri desktop application
│   ├── src/                # SvelteKit frontend
│   │   ├── lib/            # Shared modules, stores, components
│   │   └── routes/         # Page routes (home, editor)
│   └── src-tauri/          # Rust backend
│       └── src/engines/    # 10 engine plugins
├── crates/marshal-rs/      # Ruby Marshal serialization library
├── packages/
│   ├── eslint-config/      # Shared ESLint config
│   └── prettier-config/    # Shared Prettier config
└── .github/workflows/      # CI/CD pipelines
```

## License

[GPL-3.0](LICENSE)
