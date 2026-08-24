# ElectronNotIncluded UI Direction

## Goal

The UI should be a dense-but-readable simulation interface: information dense
enough to read tile state, temperature, and substance composition at a
glance, while the gameplay view (the tile grid) stays spatially legible.
Avoid a generic translucent-black egui demo look, and avoid a modern SaaS
dashboard look — this is a tool for reading a physical simulation.

## Visual tokens

All screens and components should share one token direction rather than
inventing colors per-function:

- `background`: near-black, cool-toned.
- `panel`: semi-transparent dark panel fill.
- `border`: a muted mid-tone line.
- `text`: warm-neutral light text; secondary text uses a muted variant.
- `accent`: a single accent color for selection/focus/primary action.
- `danger`: used for critical/blocked/unsafe states (e.g. overheating).
- `positive`: used for stable/available/completed states.

Spacing, padding, border width, and type scale should stay consistent across
screens. Prefer framed rows and separators over ad hoc rounded cards for
repeated content (e.g. a substance list).

## Layout principles

- Keep the gameplay center (the tile grid) as unobstructed as possible.
- Top-left: calendar/time and the most important current status.
- Bottom or a side panel: selected-tile inspector (composition, temperature).
- Modal screens (pause, menu) use a dark overlay but keep enough context that
  the player isn't confused about what's paused.
- Panel titles use a section header and a border, not a large highlighted
  background block.

## Shared components

### `Panel`

Background, border, padding, and an optional title area. Should support
normal, focused, and disabled states using the shared tokens.

### `SectionHeader`

Larger type, neutral text color, a thin divider, and a small accent touch.
Keep titles short.

### `TileInspector` (planned)

Shows the substances present in the tile under the cursor: each substance's
id, mass, and the tile's shared temperature. This is the primary way a player
reads what the simulation is doing; substance names and colors must come from
`GameData`, never be hardcoded per screen.

## Persistent HUD

The HUD shows the calendar (`Year / Day HH:MM:SS`) advanced by the server's
`GameClock`. The client only displays this state and must never advance time
itself.

## Main menu and pause

Main menu and pause menu share the same frame/border/token language:

- `Start`: enters the simulation.
- `Resume`: closes the pause overlay.
- `Quit to menu`: cleans up gameplay entities and returns to the menu.

## Readability

- Text over the tile grid must keep sufficient contrast against arbitrary
  substance colors.
- Panels, HUD elements, and any inspector must keep a stable size — text must
  not overflow, overlap, or shift sibling layout on window resize.
- Keyboard/mouse focus and selection must be visible without relying on
  hover alone.

## Implementation boundaries

- `eni-client` owns egui layout, input, camera-facing presentation, and UI
  state.
- `eni-domain` / `eni-server` own authoritative tile chemistry, time, and
  simulation rules.
- The UI must not invent world/tile data or advance the simulation clock; all
  substance/reaction identity shown in the UI must come from `GameData`.
