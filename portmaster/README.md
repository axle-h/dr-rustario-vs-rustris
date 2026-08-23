## Notes

Dr Rustario vs Rustris is an open-source pair of falling-block puzzle games written in Rust
on a shared SDL2 engine. Both games come with NES, SNES, N64 and modern themes, can be played
one or two player, mixed (each player picks a game) or as a playlist that alternates games a
stage at a time.

Source: https://github.com/axle-h/dr-rustario-vs-rustris

The game links the device's own SDL2 and reads the gamepad natively through SDL's
GameController API (no gptokeyb). Config (`config.yml`) and the high score tables
(`high_scores.*.yml`) are written to the port folder.

## Controls

| Button | Menu | Game |
|--|--|--|
| D-pad / left stick | Navigate | Move, soft drop (down), hard drop (up) |
| A | Select | Rotate clockwise |
| B | Back | Rotate anticlockwise |
| X / L1 / R1 | | Hold |
| Y | | Next theme |
| Start | Start | Pause |
| Select | | Return to menu |

## Thanks

Alex Haslehurst for the games and the PortMaster team for the framework.
