# Font Usage Guide

## Fonts to use

Use this font priority list throughout the application:

1. Tektur
2. Noto Sans
3. Noto Sans Symbols
4. Noto Sans Symbols 2
5. Noto Emoji

All active `.ttf` files are stored directly in `assets/fonts/` (no nested font folders).

Whenever there is a symbol that is not represented in an earlier font, move to the next font in the list.
Noto Sans is the preferred non-emoji fallback for unicode symbols.
If Noto Sans is not present in `assets/fonts`, runtime currently falls back to DejaVu Sans.

## Unicode symbols to use in UI

### Lives

⮝ - use for player lives

### Weapons/Tools

⛯ - use for blaster/primary weapon, rendered yellow
🚀 - use for missiles, rendered orange
🧲 - use for the ore magnet, rendered red
✦ - use for tractor beam, rendered cyan
⚛ - use for ion cannon, rendered light blue

### Collectable items

💎 - use for ore, rendered green

### Resource/Unit symbols

● / ○ - use for missile ammo slots (filled = available, empty = empty)
💎 - use as ore unit where ore is a spendable/countable unit
❤️ - use as HP unit in shop/readout text

### Upgrade Levels

Use single-char circled number symbols for upgrade levels, in the same color and right after the other symbol:

①
②
③
④
⑤
⑥
⑦
⑧
⑨
⑩

### Ability State Symbols

○ - off/inactive
⚡ - ready
⌛ - cooldown