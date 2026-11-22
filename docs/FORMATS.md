# File formats PrintCraft reads

These notes were worked out by comparing saved files, so PrintCraft can
open older projects people already have. "Known" = confirmed on real files;
"open" = not worked out yet. Corrections and new findings are welcome (see
"Learning more" below).

## Older project files (.sig, .car, .hcr, .ban, .cal, .lbl, .env, .biz, .cer, .not, .let, .nws, .bro, .pcr, .fax, .sti, .tsh, .cft, .sbp)

All kinds share one layout; only the extension differs.

Known:
- An OLE2 compound file. The main stream starts with a 28-byte text banner
  ending in CR LF 0x1A, then a 16-bit version marker (current 0x4E4A; 0x43A5
  is an older layout PrintCraft doesn't read).
- The stream is a small record store. Each record: u8 tag, u24 payload
  length, u32 link, payload. Tags: 1 free (old data left from earlier saves),
  2 live object, 3 index page, 4 directory. The directory (offset at 0x37)
  lists index pages; each holds 64 × (u16 kind, u32 payload offset); entry n
  of page p is object id p·64 + n + 1.
- Lengths are in 1/1800 inch.
- Kind 2, document info: u16, u16, u16 orientation (1 = landscape), u16, u16,
  u32 paper width, u32 paper height.
- Kind 1, page preview: u32 width @0x30, height @0x34, packed length @0x38,
  zlib-packed RGB (top-down) @0x3c.
- Kind 3, font: u16 length + name at +0x0c.
- Kind 11, picture: a whole BMP file at +0x0b.
- Kind 21, text: UTF-16LE.
- Kind 4, page object list: entries (u16 type, u32 length, body); center
  point (2 × i32) at body+0x0a, bounds (4 × i32) at body+0xa2.

Open:
- What each page-object type means and how text frames tie to their text
  and styles (so imported text is laid out in order rather than in place).
- The numeric project-kind codes in the document info.
- Writing these files back.

## Content packs (.CDT collections)

Known:
- A pack is `name.ini` (friendly name), `name.IDT` (items), `name.CDT`
  (picture data) plus index/thumbnail files PrintCraft doesn't need.
- `.IDT` item: u32 body length (not counting this 6-byte head), u16 keyword
  count, keyword ids (u32 each), file name, NUL, title, NUL. The title ends
  with a space and one capital letter (a category flag).
- `.CDT` entries sit back to back in the same order as the items:
  u32 original size, u32 stored size, u16 method (0 = stored, 2 = raw
  deflate), data. Photos are JPEG; clip art is placeable WMF.

## Learning more

The open questions above are best answered by comparing files: save a series
of tiny projects that differ by one change each (one text box; the same box
moved; a different font; one shape; one picture), then compare the files and
check what PrintCraft makes of them with

```sh
printcraft --export test01.sig test01.pdf
```

Every older project also stores a small preview picture of its first page,
which is a handy check of a new interpretation. Please don't attach other
people's projects or purchased art to issues; describe the bytes instead.
