# PrintCraft roadmap

## Done so far

- Classic and Modern layouts, light and dark themes, combined title bar.
- Project tabs (rename, duplicate, drag to reorder, close with save
  reminder) and page tabs (Front / Inside / Back or Page 1, 2, ...).
- 18 project kinds with drawn pictures in the New Project window.
- Text boxes, text in 28 shapes (lines wrap to the shape, alignment works
  inside it, Fit grows or shrinks words to fill), headlines with 17 effects,
  5 bends and an on-page bend handle, signatures in a bundled handwriting
  font, seals, initial capitals, numbers.
- 16 adjustable borders, page color, shadows, glows, frames, tint and
  see-through, groups, align and space evenly, snapping with adjustable pull,
  printable-area guide.
- Shared color chooser: basics, last 10 colors, eyedropper, hue / strength /
  lightness sliders, color families, copy and paste color codes.
- Fonts: computer fonts, bundled fonts, online fonts with previews; the font
  list shows each font and arrow keys try fonts live on the selected text.
- Pictures: content packs (with or without their index files), Google
  Images search (in a PrintCraft window or the browser), My Art library,
  files, paste, drag and drop, optional AI pictures.
- Photo Editor: crop, turn, color, recolor, brightness, sharpen, red eye,
  effects, cut out by shape, own outline, magic wand, color pick, background
  removal (with New / Add / Subtract, tolerance, soft edges).
- Printing: printer choice, copies, page ranges, quality, black and white,
  shrink to printable area, cut marks, two-sided, test page, preview.
- Safety copies every few minutes with recovery after a crash; save
  reminders on close; settings; searchable help.

## Tables (planned)

A table is a grid item: rows and columns of text cells with their own fonts,
fill colors and borders. Plan: a `Table` content type (cells hold the same
text look as text boxes), Insert Table with a row/column picker, click a cell
to type, Tab to move, toolbar buttons for add/remove row or column, merge and
split, and a few ready-made looks. The renderer draws cell fills, grid lines
and each cell's text with the existing text layout, so printing works without
extra work. Calendars will then be built from tables (a month grid with dates
filled in).

## Web pages (planned, small)

Publish a design as a simple web page. Plan: File > Save as Web Page
writes a folder with `index.html` and pictures: each page becomes a picture with its words also placed as real HTML text (so it can be read and
searched), plus clickable links for any item given a web address. No web
hosting is built in; the folder can be uploaded anywhere.

## Still to come

- Ready-made designs (templates) and exact placement when opening older
  projects (see "Opening older projects" below).
- Mixed styles inside one text box (one font/size/color per box today),
  bullets and numbering, drop caps, text wrap around pictures, spell check.
- Freehand drawing, gradient fills for shapes, picture frames.
- Tables and web pages (above).

## Ideas worth adding

- Ready-made designs to start from, searchable (PrintCraft's own templates, still to be made).
- Resize a design to another kind (for example, a sign into a card).
- Layers list to pick hidden or stacked items.
- Text effects presets (curve, outline, shadow) in one click — partly done
  with headlines.
- Brand colors and fonts saved as a set.
- Shareable PDF/PNG export presets.

## Opening older projects properly

Older projects open with their paper size, words, pictures and fonts, but
text is laid out in order down the page rather than where it was. Next:
work out what each page-object type is, where text frames sit and how text
is styled, by comparing many saved files and checking each result against
the small preview picture stored in every file (see docs/FORMATS.md
"Learning more").

## AI pictures

No AI company lets other apps sign in with a regular account to make
pictures, so the simplest path is a free key: PrintCraft opens Google AI
Studio in the browser, the person copies the key once, and pastes it. Google
Gemini's image model has a free tier; OpenAI keys work too (paid). Claude
doesn't create pictures, so it isn't offered. The key stays on the computer.
Calling these services over the internet doesn't affect PrintCraft's own
license (GPL-3.0-or-later).
