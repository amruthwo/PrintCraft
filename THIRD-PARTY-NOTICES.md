# Third-party notices

PrintCraft itself is licensed under the GNU General Public License, version 3
or later (see [LICENSE](LICENSE)). It bundles the following fonts, which keep
their own license:

| Font | Files | License |
|---|---|---|
| Nunito (Copyright 2014 The Nunito Project Authors) | `assets/fonts/Nunito-*.ttf` | SIL Open Font License 1.1, see [assets/fonts/OFL-Nunito.txt](assets/fonts/OFL-Nunito.txt) |
| Dancing Script (Copyright 2016 The Dancing Script Project Authors, Reserved Font Name "Dancing Script") | `assets/fonts/DancingScript-*.ttf` | SIL Open Font License 1.1, see [assets/fonts/OFL-DancingScript.txt](assets/fonts/OFL-DancingScript.txt) |

The egui default fonts (Ubuntu Light and Hack) are built in through the
`epaint_default_fonts` crate under their own open licenses.

Rust libraries PrintCraft is built from (egui/eframe, image, tiny-skia,
pdf-writer, ab_glyph, ttf-parser, cfb, wry, tao and others) are under the
MIT, Apache-2.0 or similar permissive licenses; `Cargo.lock` lists every one.

Fonts downloaded through **Online > Online Fonts** come from
[Fontsource](https://fontsource.org/) and are open-licensed; each keeps its
own license. Pictures found online or made with AI belong to their owners or
follow the service's terms. PrintCraft ships no clip art, templates or
pictures of its own beyond its icons.
