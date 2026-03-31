# bevy_simon

A Simon Says memory game built with [Bevy](https://bevyengine.org/) in Rust. The game displays a growing sequence of colored buttons (red, green, blue, yellow) that the player must memorize and repeat. Each round adds one more step to the sequence, and the game ends when the player makes a mistake. High scores are persisted locally (to a file on desktop, or to `localStorage` on WASM).

## Features

- Four colored triangle buttons with sound effects and hover highlighting
- Progressive difficulty — the pattern grows by one each round
- High score tracking with persistent storage
- Multiple scenes: title screen, main menu, game, score, and credits
- WASM build target support for playing in the browser
- Close the window with the Escape key (desktop)

## Limitations

- Mouse-only input — no keyboard or touch controls for gameplay
- No configurable difficulty or playback speed settings
- All game logic lives in a single source file (`src/main.rs`)
- Maximum pattern length of 255

## Building and Running

Requires Rust nightly (configured via `rust-toolchain.toml`).

**Desktop (development):**

```
cargo run --features bevy/dynamic_linking
```

**WASM release build:**

```
make release
```

This compiles for `wasm32-unknown-unknown`, runs `wasm-bindgen`, and packages the result into `game.zip`.

## History

bevy_simon was developed and released on 2024-03-30:

- 2024-03-30 — Initial implementation with the full Simon Says game, including four-color pattern gameplay, sound effects, scene management, high score persistence, and WASM build support
