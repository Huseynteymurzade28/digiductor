# digiductor

> A cyberpunk **Digimon encyclopedia** for your terminal.

Browse 1,400+ Digimon, walk their branching digivolution lines, and collect
favourites in your own Digi-Vault, all inside a neon-on-black "Digital World"
TUI built with [Ratatui](https://ratatui.rs). Data comes live from the public
[Digi-API](https://digi-api.com).

```
╔══════════════ ▟█ DIGIDUCTOR ░▒▓ DIGITAL MONSTER ENCYCLOPEDIA ▓▒░ ═══ ◉ READY ╗
╭ ◈ DIGI-INDEX ─────────────╮╭ ▣ DIGIMON ANALYZER ─────────────────────────────╮
│⌕ type / to search…        ││╭ SPRITE ───────────╮⟪ AGUMON ⟫ ★               │
│LVL:[ALL] ATR:[ALL]        │││     ▄▄▄▄▄▄▄▄▄     │#0001 · released 1997        │
│◤ 50 / 1488 records ◢      │││  ▄▄▄▄▀▀▀▄▄▄▄▄▄▄   │LEVEL      ▎ Rookie          │
│▶ #1    Agumon ★           │││ ▄▄▄▄▄▄▄▄▄▀▀▀▀▄▄▄  │ATTRIBUTE  ▎ Vaccine         │
│  #2    Airdramon          │││ ▀▀▄▄▄▄ ▄▄▄▄▄▄▄▄▄  │TYPE       ▎ Reptile         │
│  #3    Angemon            │││  ▀▀▀▀▀▀▀▀▀▄▄▄▀▀▀  │── SIGNATURE MOVES ──        │
│  #4    Betamon            ││╰────── PIXEL · p ─╯✦ Baby Flame                  │
│  …                        │╰─────────────────────────────────────────────────╯
│                           │╭ ⇄ EVOLUTION MATRIX ─────────────────────────────╮
│                           ││   PRIOR ├─ Koromon                              │
│                           ││         └─ Tsunomon                             │
│                           ││    ╔══════════════════╗                         │
│                           ││    ║      Agumon      ║                         │
│                           ││    ║ Rookie · Vaccine ║                         │
│                           ││    ╚══════════════════╝                         │
│                           ││    NEXT ├─ Greymon                              │
╰───────────────────────────╯╰─────────────────────────────────────────────────╯
 ▸ ★ Agumon added to the Digi-Vault    ↑↓ move · / search · s random · ? help
```

## Features

- **Search and filter** by name, level (Fresh → Mega) and attribute
  (Vaccine / Data / Virus). The index streams in more pages as you scroll.
- **Digimon Analyzer**: level, attribute, type, fields, X-Antibody badge,
  the field-guide entry and **signature moves**.
- **Evolution Matrix**: a branched graph of prior and next evolutions. Hop in
  with `e` and jump along the digivolution line, then `⌫` to walk back.
- **Digi-Vault**: star favourites with `f` and browse them with `v`. The vault
  is saved to disk and works offline.
- **Random encounter**: press `s` to land on a random Digimon.
- **Uniform pixel sprites**: see below.
- **Offline-friendly cache**: every record you open is kept on disk, so
  revisiting it is instant.
- **Never blocks**: all network work runs in the background with a braille
  spinner and "digitizing" boot animation.

## Sprites

The Digi-API serves whatever artwork exists for each Digimon. Some entries are
full-colour illustrations, others are black-and-white manga line art, and all of
them sit on a white background. Shown side by side, they don't match.

digiductor runs every image through the same pipeline so the whole dex looks
like one set of game sprites:

1. **Cut out**: the white background is flood-filled from the edges and made
   transparent. Enclosed whites such as eyes and wings are kept.
2. **Frame**: the image is cropped to the silhouette and centred, so every
   Digimon is drawn at the same size.
3. **Pixelate**: the image is reduced to a fixed 56×56 grid with hard edges.
4. **Colourise**: line art is gradient-mapped onto the Digimon's attribute
   colour (Vaccine green, Data blue, Virus purple, Free yellow).
5. **Posterize and outline**: the palette is reduced and a 1px glow outline
   is added.

Press `p` to switch between three styles:

| Style      | Look                                                         |
| ---------- | ------------------------------------------------------------ |
| `PIXEL`    | Uniform pixel sprites. This is the default.                  |
| `HOLO`     | Monochrome hologram in the attribute colour, with scanlines. |
| `ORIGINAL` | The untouched source artwork.                                |

On terminals with a graphics protocol (Kitty, Ghostty, WezTerm, iTerm2, foot)
you see the real pixels. Everywhere else the sprite falls back to coloured
Unicode half-blocks, which work in any truecolor terminal.

## Install & run

You need a recent stable [Rust toolchain](https://rustup.rs).

```bash
git clone <this repo> && cd digiductor
cargo run --release
```

To install the binary on your `PATH`, run `cargo install --path .`.

## Controls

Press `?` in the app to open this list.

| Key               | Action                                          |
| ----------------- | ----------------------------------------------- |
| `↑`/`↓` `j`/`k`   | Move selection (loads more as you scroll)       |
| `PgUp`/`PgDn`     | Jump 10                                         |
| `/`               | Search by name (`Enter` apply, `Esc` cancel)    |
| `l`               | Cycle level filter (Rookie / Champion / Mega …) |
| `a`               | Cycle attribute filter (Vaccine / Data / Virus) |
| `x`               | Clear all filters                               |
| `e` / `Tab`       | Focus the evolution matrix (`↑↓` pick, `Enter` jump) |
| `Backspace` / `b` | Go back after an evolution / random jump        |
| `s`               | Random Digimon                                  |
| `f`               | Add / remove from the Digi-Vault                |
| `v`               | Toggle the Digi-Vault view                      |
| `p`               | Cycle sprite style (pixel / holo / original)    |
| `[` / `]`         | Scroll the analyzer                             |
| `r`               | Reload                                          |
| `?`               | Help                                            |
| `q` / `Esc`       | Quit                                            |

## Configuration

| Variable              | Effect                                                                |
| --------------------- | --------------------------------------------------------------------- |
| `DIGIDUCTOR_GRAPHICS` | `on` always probes for a graphics protocol, `off` forces half-blocks. |

By default digiductor only probes the terminal when the environment points to
a graphics-capable one. If a terminal never answers the probe, the probe would
keep reading key presses. Inside tmux, image passthrough is handled by
ratatui-image.

### Files

| Path                                      | Contents                     |
| ----------------------------------------- | ---------------------------- |
| `~/.cache/digiductor/digimon_cache_v2.json` | Cached Digimon records      |
| `~/.local/share/digiductor/favorites.json`  | Your Digi-Vault             |

On macOS and Windows these live in the OS cache and data directories.

## Architecture

| Module      | Responsibility                                              |
| ----------- | ----------------------------------------------------------- |
| `network`   | Typed Digi-API models + async `reqwest` fetchers            |
| `cache`     | Two-tier cache: in-memory `HashMap` + JSON file on disk     |
| `favorites` | The persisted Digi-Vault                                    |
| `sprite`    | Image pipeline that turns mixed artwork into uniform sprites |
| `app/state` | State machine + reducer (input → state, net result → state) |
| `event`     | Multiplexes input, animation ticks, and network results     |
| `ui`        | Ratatui render layer (index / analyzer / evolution / help)  |
| `theme`     | Neon palette and shared widget styling                      |

All network work is spawned onto Tokio. Results come back to the main loop as
ordinary `Event::Net` messages, so a slow request never blocks rendering.

### Level names

The Digi-API uses the Japanese-tradition level names internally. digiductor
shows the English terms and translates them for you:

| English (shown) | API value |
| --------------- | --------- |
| Fresh           | Baby I    |
| In-Training     | Baby II   |
| Rookie          | Child     |
| Champion        | Adult     |
| Ultimate        | Perfect   |
| Mega            | Ultimate  |

## Credits & Attribution

- **Data and sprites** are provided at runtime by the [Digi-API](https://digi-api.com)
  (<https://github.com/digi-api/digimoji>), a free, open Digimon API. This
  project does **not** bundle or redistribute any Digimon data or artwork. Every
  record and image is fetched live from the API and only cached locally on the
  user's own machine.
- Built with [Ratatui](https://ratatui.rs),
  [ratatui-image](https://github.com/benjajaja/ratatui-image),
  [tokio](https://tokio.rs), and [reqwest](https://github.com/seanmonstar/reqwest).

## Disclaimer

"Digimon" (Digital Monsters), all related characters, names, sprites, and
imagery are trademarks of and © **Bandai / Toei Animation**. This project is an
unofficial, non-commercial fan-made tool created for **educational purposes**
and is **not affiliated with, endorsed by, or sponsored by** Bandai, Toei, or
the Digi-API maintainers. All Digimon intellectual property belongs to its
respective owners.

## License

The **source code** of digiductor is released under the [MIT License](LICENSE).
This license covers the code only. It does **not** grant any rights to the
Digimon data, names, or artwork served by the Digi-API, which remain the
property of their respective owners (see Disclaimer above).
