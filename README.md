# digiductor

A cyberpunk terminal **Digimon encyclopedia** built with [Ratatui](https://ratatui.rs).
Neon-on-black "Digital World" aesthetic, async data from the public
[Digi-API](https://digi-api.com), in-terminal **sprites**, and a branched
evolution graph.

## Sprites

Each Digimon's artwork is downloaded and rendered right in the terminal. On
terminals with a graphics protocol (Kitty, iTerm2, Sixel) you get the real
image; everywhere else it falls back to colored Unicode half-blocks — which
still works in any truecolor terminal and suits the pixel-sprite aesthetic.
Decoded sprites are cached in memory for the session.

```
▟█ DIGIDUCTOR ░▒▓ DIGITAL MONSTER ENCYCLOPEDIA ▓▒░
┌ DIGI-INDEX ─────────┐┌ DIGIMON ANALYZER ───────────────┐
│ ⌕ greymon           ││ ⟪ AGUMON ⟫                       │
│ LVL:[Rookie] ATR... ││ #0001 · released 1997            │
│ ▶ #1   Agumon       ││ LEVEL     ▎ Rookie               │
│   #246 Greymon      ││ ATTRIBUTE ▎ Vaccine              │
│   ...               │└──────────────────────────────────┘
│                     │┌ EVOLUTION MATRIX ────────────────┐
│                     ││   PRIOR ├─ Koromon               │
│                     ││         └─ Tsunomon              │
│                     ││         ╔════════╗               │
│                     ││         ║ Agumon ║               │
│                     ││         ╚════════╝               │
│                     ││    NEXT ├─ Greymon               │
└─────────────────────┘└──────────────────────────────────┘
```

## Run

```bash
cargo run --release
```

## Controls

| Key            | Action                                   |
| -------------- | ---------------------------------------- |
| `↑`/`↓` `j`/`k`| Move selection (auto-loads more on scroll) |
| `PgUp`/`PgDn`  | Jump 10                                  |
| `/`            | Search by name (`Enter` apply, `Esc` cancel) |
| `l`            | Cycle level filter (Rookie / Champion / Mega …) |
| `a`            | Cycle attribute filter (Vaccine / Data / Virus) |
| `[` / `]`      | Scroll the analyzer description          |
| `x`            | Clear all filters                        |
| `r`            | Reload index                             |
| `q` / `Esc`    | Quit                                     |

## Architecture

Clean, layered separation:

| Module      | Responsibility                                              |
| ----------- | ---------------------------------------------------------- |
| `network`   | Typed Digi-API models + async `reqwest` fetchers           |
| `cache`     | Two-tier cache: in-memory `HashMap` + JSON file on disk    |
| `app/state` | State machine + reducer (input → state, net result → state) |
| `event`     | Multiplexes input, animation ticks, and network results    |
| `ui`        | Ratatui render layer (index / analyzer / evolution matrix) |
| `theme`     | Neon palette and shared widget styling                     |

### How it stays responsive

All network work is **spawned** onto Tokio. Its results re-enter the main loop
as just another `Event::Net`, so a slow request never blocks rendering — a
braille spinner and "digitizing" boot animation keep playing throughout.

### Notes on the API

The Digi-API filters by its internal (Japanese-tradition) level names. digiductor
maps the familiar English terms to them transparently:

| English (shown) | API value |
| --------------- | --------- |
| Fresh           | Baby I    |
| In-Training     | Baby II   |
| Rookie          | Child     |
| Champion        | Adult     |
| Ultimate        | Perfect   |
| Mega            | Ultimate  |

Fetched records are cached to `~/.cache/digiductor/digimon_cache.json`, so
revisiting a Digimon (even across restarts) is instant and works offline.

## Credits & Attribution

- **Data & sprites** are provided at runtime by the [Digi-API](https://digi-api.com)
  (<https://github.com/digi-api/digimoji>) — a free, open Digimon API. This
  project does **not** bundle or redistribute any Digimon data or artwork; every
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
This license covers the code only — it does **not** grant any rights to the
Digimon data, names, or artwork served by the Digi-API, which remain the
property of their respective owners (see Disclaimer above).
