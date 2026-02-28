# Font Usage Guide

## Fonts to use

For text, use Tektur font throughout the application.

All active `.ttf` files are stored directly in `assets/fonts/` (no nested font folders).

Whenever there is a symbol that is not represented in Tektur, use symbol fallbacks in this order:

1. Noto Sans Symbols
2. Noto Sans Symbols 2
3. Targeted fallback fonts for known missing glyphs:
	- DejaVu Sans for specific unicode symbols (for example ↭)
	- Noto Emoji for emoji glyphs (for example 🚀 🧲 💎)

## Unicode symbols to use in UI

### Lives

⮝ - use for player lives

### Weapons/Tools

⛯ - use for blaster/primary weapon, rendered yellow
🚀 - use for missiles, rendered orange
🧲 - use for the ore magnet, rendered red
↭ - use for tractor beam, rendered cyan
⚛ - use for ion cannon, rendered light blue

### Collectable items

💎 - use for ore, rendered green

### Resource/Unit symbols

🚀 - use as missile unit where missile is a countable unit
💎 - use as ore unit where ore is a spendable/countable unit
❤️ - use as HP unit in shop/readout text

### Upgrade Levels

Use single-char roman numeral symbols for upgrade levels, in the same color and right after the other symbol:

Ⅰ
Ⅱ
Ⅲ
Ⅳ
Ⅴ
Ⅵ
Ⅶ
Ⅷ
Ⅸ
Ⅹ