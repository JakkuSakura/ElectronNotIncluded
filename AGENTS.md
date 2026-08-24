# ElectronNotIncluded developer conventions

## Project identity

- Project name: `ElectronNotIncluded`
- Short name / crate prefix: `eni`
- A 2D tile-based sandbox in the spirit of Oxygen Not Included, built around an
  advanced, realistic chemistry and fluid-dynamics simulation. No
  devices/machines in the current pass — the focus is substances, mixing,
  reactions, heat, and mass transfer.

## Architecture

The project is a Rust + Bevy in-process client/server architecture. The
server owns authoritative state and rules; the client owns input, camera, and
presentation. Both run as plugins of the same Bevy `App` inside the
launcher. Cross-boundary types (components, resources, messages) live in
`eni-domain` — the server must never depend on client rendering types.

- `crates/eni-domain`: shared components, resources, messages, and domain
  rules (chunk/tile grid, chemistry: substances/reactions/mixing, calendar).
- `crates/eni-server`: authoritative simulation — chunk generation/streaming,
  player movement, and the per-tick chemistry pipeline.
- `crates/eni-client`: input, camera, tile rendering, and UI presentation.
- `crates/eni`: desktop launcher combining server and client, plus CLI/REST
  tooling.

## Data-driven content

Game content is loaded from `assets/data/*.json`:

- `assets/data/substances.json`: `SubstanceDefinition` collection (id, phase
  at STP, density, specific heat, melting/boiling points, thermal
  conductivity).
- `assets/data/reactions.json`: `ReactionRule` collection (reactants,
  optional temperature window, products, energy delta).
- `eni_domain::GameData::load`: reads, parses, and validates both files
  (unique substance ids, reactions reference known substance ids).

When adding new chemistry, update the JSON data first; only touch Rust code
when new behavior or a new field is required. A tile's contents (`Composition`)
merge substances by mass when they mix — there is intentionally no
"separate"/"extract" operation. Once water and salt mix into saltwater, that
mixture stays a mixture; undoing it requires modeling an explicit process
(e.g. evaporation), never an inverse of `mix`.

World generation must stay deterministic: the same `(seed, chunk coordinate)`
must always produce the same tile contents. Do not let the client regenerate
world data — the client only consumes chunk data streamed by the server.

Game time is advanced by the server's authoritative `GameClock` using
`Time<Real>`. The client only reads `GameClock` to display the calendar and
must not accumulate wall time itself.

Use `cargo run -p eni --bin dump_world_preview` to inspect generated chunk
contents as a PNG; the preview binary must use the same `GameData` and
generator as the runtime.

## CLI / REST conventions

The CLI's `--headless`, `--serve`, and `--start-state menu|playing` are
runtime options, not subcommands — `serve` is never a subcommand. Normal mode
can run the Bevy client, authoritative server, and Axum REST server together
via `--serve`; headless mode skips the primary window and drives the app loop
with `ScheduleRunnerPlugin`. The REST bind address is set with `--bind`;
headless state can be switched via `POST /api/control/state` with
`{"state":"menu"}` or `{"state":"playing"}`.

`GET /api/render/world.png` renders the current chunk from server-side tile
data (colored by dominant substance) and is for data preview only. `GET
/api/render/framebuffer.png` must request the real Bevy offscreen
framebuffer, never a desktop screenshot or a redrawn approximation.

## Rust conventions

- Use `cargo fmt`, `cargo clippy --all-targets --all-features`, and
  `cargo test --workspace`.
- New public APIs return `Result<T, E>`; prefer `thiserror` for error types.
  Avoid unjustified `unwrap`/`expect`.
- Unhandled event branches use `tracing::warn!`, never a silent drop.
- Keep modules small, organized via `mod.rs`; avoid speculative abstraction
  for requirements that do not exist yet.
- Comments explain *why*, not *what* — never restate the code in prose.
- A placeholder implementation uses `todo!()` and must not be reachable from
  any test or the running simulation; never commit an empty function or file.

## Change boundaries

New gameplay enters the domain layer first, then is driven by the server,
and finally observed/rendered by the client. Asset loading, save data, and
networking should stay in their own module boundaries rather than living in
the launcher.
