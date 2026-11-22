# PrintCraft design notes

The sizes, numbers and rules PrintCraft uses, collected in one place so the
code and the documentation agree. Code comments point at the section names
below. The file formats PrintCraft reads are described in
[FORMATS.md](FORMATS.md).

## Editor layout

| Part | Contents |
|---|---|
| Menu bar | File, Edit, View, Insert, Arrange, Effects, Text, Table, Web, Tools, Window, Help |
| Tool panel (Classic layout) | New Project, Pictures, Text Tools, Effects, Decorations, Photo Tools, Drawing Tools, Addresses & Dates, Print & Share, Online |
| Modern layout | a short tab bar of the same tools, and a context bar that shows only what fits the selection |
| Zoom presets | 25, 50, 100 (actual size), 200, 400, 600 %, fit to window |
| Tint steps | 100 % down to 10 % in 10 % steps |
| Card views | Front, Inside, Back |
| Right-click menu | cut / copy / delete, position (centre on page, horizontally, vertically), size (full page), flip, layer |
| Status bar | page n of m, zoom buttons, help text, pointer position |
| Pages | every page shown in the workspace (in a list or side by side); Settings > Legacy Pages shows one page at a time with page buttons along the bottom |
| Rulers | marks down to 1/16 inch when there's room; click a ruler to add a guideline (moves in 1/16-inch steps; removed with ×, Delete or by dragging it off the ruler; never printed) |

## Project sizes

Inches. Code: `src/kinds.rs`.

| Kind | Size | Panels |
|---|---|---|
| Sign / Poster | 8.5 × 11 | 1 |
| Banner | 11 × 8.5 per sheet, 4 sheets long | 1 |
| Quarter-fold card | 4.25 × 5.5 panel | Front, Inside, Back |
| Half-fold card | 5.5 × 8.5 panel | Front, Inside, Back |
| Calendar | 11 × 8.5 | 1 |
| Label (30 per sheet) | 2.625 × 1 | 1 |
| Envelope #10 | 9.5 × 4.125 | 1 |
| Business card | 3.5 × 2 | 1 |
| Certificate | 11 × 8.5 | 1 |
| Note card | 5.5 × 4.25 | Front, Inside, Back |
| Letterhead / Newsletter / Fax | 8.5 × 11 | 1 |
| Brochure (tri-fold) | 11 × 8.5 | Outside, Inside |
| Postcard | 6 × 4 on the default 2-per-sheet layout | Front, Back |
| Scrapbook page | 12 × 12 | 1 |

Fold layouts (`src/export.rs`): a quarter-fold card prints on one sheet
twice the panel size each way: back (left) and front (right) on the bottom
half, the inside upside down on the top half. Fold the top half back, then
fold in half. Half-fold and note cards print two sheets: back (left) and
front (right) on the first, the inside on the right half of the second, to
print on the other side.

## Frames and soft effects

Code: `src/effects.rs`.

| Setting | Value |
|---|---|
| Frame styles | thin, medium, thick, double thin, double medium, double thick, round corner, drop shadow, picture frame |
| Thin / medium / thick line | 1 pt (1/72 in) / 1.5 pt (1/48 in) / 2.25 pt (1/32 in) |
| Double lines | line, gap, line: 1 + 3 + 1 pt, 1.5 + 4.5 + 1.5 pt, 2.25 + 6 + 2.25 pt |
| Single frame | rectangle inset by half the line width |
| Round corner | 1 pt line, corner 1/24 in + 1 pt |
| Drop-shadow frame | 1 pt outline plus 1/16 in solid bars on the right and bottom |
| Effect sliders | 0..1 in two straight pieces: low → middle over 0–0.5, middle → high over 0.5–1 |
| Shadow distance | 0.029 / 0.125 / 0.22 × object size, along the shadow's angle |
| Shadow blur | 0 / 0.03 / 0.06 × object size + 4/96 in; none when softness is 0 |
| New shadow | distance 0.08, angle 45°, strength 1.0, softness 0 |
| Glow | reach 2 × (0.04 / 0.135 / 0.23) × object size; new glow size 0.3 |
| Tint | below 100 % each channel moves toward white by (100 − tint) %: c += (255 − c) × amount |
| Undo | at least 10 steps; the setting starts at 100 |

"Object size" is the longer side of the item. Hairline is 0.25 pt.

## Paper sheets

Labels, cards and postcards printed several to a sheet. Inches. The presets
live in `src/kinds.rs` (`LABEL_SHEETS`, `CARD_SHEETS`, `POSTCARD_SHEETS`,
`ENVELOPES`) and follow the published sizes of the common stock sheets
(the product numbers are the familiar ones printed on the package), with
the labels centred across the page.

| Sheet | Label size | Across × down | First at (left, top) | Step | Corner |
|---|---|---|---|---|---|
| 5160 address | 2.625 × 1 | 3 × 10 | 0.1875, 0.5 | 2.75, 1 | 0.1875 |
| 8160 address | 2.625 × 1 | 3 × 10 | 0.1875, 0.5 | 2.75, 1 | 0.25 |
| 5161 address | 4 × 1 | 2 × 10 | 0.15625, 0.5 | 4.1875, 1 | 0.1875 |
| 5162 address | 4 × 1.3333 | 2 × 7 | 0.15625, 0.8333 | 4.1875, 1.3333 | 0.1875 |
| 5159 address | 4 × 1.5 | 2 × 7 | 0.15625, 0.25 | 4.1875, 1.5 | 0.1875 |
| 5163 shipping | 4 × 2 | 2 × 5 | 0.15625, 0.5 | 4.1875, 2 | 0.375 |
| 5164 shipping | 4 × 3.3333 | 2 × 3 | 0.15625, 0.5 | 4.1875, 3.3333 | 0.375 |
| 5168 shipping | 3.5 × 5 | 2 × 2 | 0.5, 0.5 | 4, 5 | 0.09375 |
| 5267 return address | 1.75 × 0.5 | 4 × 20 | 0.28125, 0.5 | 2.0625, 0.5 | 0.1875 |
| 8257 return address | 2.25 × 0.75 | 3 × 10 | 0.375, 0.625 | 2.75, 1 | 0.09375 |
| 5266 file folder | 3.4375 × 0.6667 | 2 × 15 | 0.53125, 0.5 | 4, 0.6667 | 0.25 |
| 5395 name badge | 3.375 × 2.3333 | 2 × 4 | 0.6875, 0.5833 | 3.75, 2.5 | 0.25 |
| 5293 round | 1.625 | 4 × 6 | 0.4375, 0.5 | 2, 1.6667 | round |
| 5294 round | 2.5 | 3 × 4 | 0.25, 0.5 | 2.75, 2.5 | round |
| 5165 full sheet | 8.5 × 11 | 1 × 1 | 0, 0 | — | 0 |
| L7160 (A4) | 63.5 × 38.1 mm | 3 × 7 | 7.21, 15.15 mm | 66.04, 38.1 mm | 6 mm |
| L7163 (A4) | 99.1 × 38.1 mm | 2 × 7 | 4.65, 15.15 mm | 101.6, 38.1 mm | 6 mm |
| 8871 business cards | 3.5 × 2 | 2 × 5 | 0.75, 0.5 | 3.5, 2 | 0 |
| 8386 postcards | 6 × 4 | 1 × 2 | 1.25, 1.25 | 6, 4.5 | 0 |
| 8387 postcards | 4.25 × 5.5 | 2 × 2 | 0, 0 | 4.25, 5.5 | 0 |
| 5388 index cards | 5 × 3 | 1 × 3 | 1.75, 1 | 5, 3 | 0 |
| 4 × 6 card stock | 6 × 4 on 4 × 6 paper | 1 × 1 | 0, 0 | — | 0 |
| A4 postcards | 146.5 × 110.4 mm | 1 × 2 | 1.25, 1.25 | —, 4.8465 | 0 |

Envelopes (long × short side, inches): Card 5.75 × 4.375, Half-fold card
8.875 × 5.5, Monarch 7.5 × 3.875, Personal 6.5 × 3.625, #9 8.875 × 3.875,
#10 9.5 × 4.125, #11 10.375 × 4.5, #12 11 × 4.5, #14 11.5 × 5, A6 6.5 × 4.75,
A7 7.25 × 5.25, and the ISO sizes DL 220 × 110 mm, C5 229 × 162 mm,
C6 162 × 114 mm, C65 229 × 114 mm. Larger papers offered: 11 × 17 and
8.5 × 14.

Rules: labels cycle through the project's pages (one design per page,
repeated across the sheet); business cards and postcards print one full
sheet per side (fronts, then backs). "Start at label" skips that many places
on the first sheet. Note cards print 2 to an 8.5 × 5.5 sheet.

## Headline warp

Before an arch, wave or bulge is applied, letter outlines are flattened into
short straight pieces, so the warp only ever moves points and curves bend
smoothly (`src/raster.rs`, `draw_headline`).
