//! A small sample sign used for the check picture and as a demo.

use crate::kinds::Kind;
use crate::model::*;

pub fn poster() -> Project {
    let mut p = Project::new(Kind::Sign, false);
    let mut add = |p: &mut Project, frame: Frame, content: Content, f: &dyn Fn(&mut Item)| {
        let id = p.fresh_id();
        let mut it = Item::new(id, frame, content);
        f(&mut it);
        p.panels[0].items.push(it);
    };
    p.panels[0].background = Rgba::hex("#fffaf0");
    add(&mut p, Frame::new(0.3, 0.3, 7.9, 10.4), Content::Border { style: 2 }, &|i| {
        i.stroke = Rgba::hex("#2f6fb0");
        i.fill = Rgba::hex("#f2b631");
    });
    add(&mut p, Frame::new(2.6, 1.3, 3.3, 3.3), Content::Shape { outline: Outline::Burst }, &|i| {
        i.fill = Rgba::hex("#f9d36b");
        i.stroke = Rgba::hex("#e08a1e");
        i.stroke_pt = 2.0;
    });
    let look = TextLook { color: Rgba::hex("#d1495b"), bold: true, align: Align::Center, ..Default::default() };
    add(
        &mut p,
        Frame::new(0.9, 4.6, 6.7, 1.9),
        Content::Headline { text: "Spring Bake Sale".into(), look, flair: Flair::BlendDown, warp: Warp::ArchUp, second: Rgba::hex("#7b2cbf"), outline: Rgba::hex("#3a1450"), stretch: false, bend: 1.0, bulge: 0.0 },
        &|_| {},
    );
    let body = TextLook { size: 20.0, align: Align::Center, color: Rgba::hex("#2b2d42"), ..Default::default() };
    add(
        &mut p,
        Frame::new(1.4, 6.8, 5.7, 1.6),
        Content::Text { text: "Saturday, 10 am to 2 pm\nCommunity Hall\nPies, cookies and fresh bread".into(), look: body, shape: None, fit: false, runs: Vec::new() },
        &|_| {},
    );
    for (x, c) in [(1.0, "#ef476f"), (6.6, "#06d6a0")] {
        add(&mut p, Frame::new(x, 8.9, 0.9, 0.9), Content::Shape { outline: Outline::Heart }, &move |i| {
            i.fill = Rgba::hex(c);
            i.shadow = Some(Shadow { softness: 0.5, ..Shadow::new(Rgba(0, 0, 0, 90)) });
        });
    }
    let small = TextLook { size: 14.0, italic: true, align: Align::Center, valign: VAlign::Middle, ..Default::default() };
    add(&mut p, Frame::new(2.4, 8.7, 3.7, 1.3), Content::Text { text: "Everyone welcome! Bring a friend".into(), look: small, shape: Some(Outline::Balloon), fit: true, runs: Vec::new() }, &|i| {
        i.fill = Rgba::WHITE;
        i.stroke = Rgba::hex("#2b2d42");
        i.stroke_pt = 1.5;
    });
    // Two guidelines from the rulers, to show they're there (they never print).
    p.panels[0].guides = vec![Guide { vertical: true, at: 4.25 }, Guide { vertical: false, at: 4.5 }];
    p
}

/// A folded card with words on several pages, for trying the every-page view.
pub fn card() -> Project {
    let mut p = Project::new(Kind::CardQuarter, false);
    let n = p.panels.len();
    for (k, words) in ["Happy Birthday!", "Wishing you a wonderful year", "Made with love"].iter().enumerate().take(n) {
        let id = p.fresh_id();
        let look = TextLook { size: 26.0, bold: k == 0, align: Align::Center, valign: VAlign::Middle, color: Rgba::hex("#d1495b"), ..Default::default() };
        let (w, h) = (p.width, p.height);
        p.panels[k].items.push(Item::new(id, Frame::new(0.4, h * 0.35, w - 0.8, h * 0.3), Content::Text { text: (*words).into(), look, shape: None, fit: false, runs: Vec::new() }));
    }
    p.panels[0].background = Rgba::hex("#fff4e8");
    p.panels[0].guides = vec![Guide { vertical: true, at: p.width / 2.0 }];
    p
}

/// A made-up photo (sky, sun, hills and two balls) for checking the Photo
/// Editor's look: see `PRINTCRAFT_OPEN=photo` in docs/DEVELOPING.md.
pub fn test_photo() -> Vec<u8> {
    let (w, h) = (600u32, 400u32);
    let img = image::RgbaImage::from_fn(w, h, |x, y| {
        let (fx, fy) = (x as f32, y as f32);
        let ball = |cx: f32, cy: f32, r: f32| (fx - cx).hypot(fy - cy) < r;
        let c: [u8; 3] = if ball(200.0, 285.0, 58.0) {
            [214, 52, 64]
        } else if ball(392.0, 300.0, 42.0) {
            [52, 102, 214]
        } else if ball(480.0, 88.0, 46.0) {
            [250, 206, 60]
        } else if fy > 250.0 + 30.0 * (fx / 90.0).sin() {
            [72, 160 - (fy as u32 / 8) as u8, 82]
        } else {
            let t = fy / 260.0;
            [(150.0 + 80.0 * t) as u8, (200.0 + 40.0 * t) as u8, 245]
        };
        image::Rgba([c[0], c[1], c[2], 255])
    });
    crate::photo::to_png(&image::DynamicImage::ImageRgba8(img))
}

/// Every line style on shapes, a script headline with an outline, and every
/// flourish: for checking how they draw (PRINTCRAFT_SAMPLE=lines).
pub fn lines() -> Project {
    let mut p = Project::new(Kind::Sign, false);
    let mut add = |p: &mut Project, frame: Frame, content: Content, f: &dyn Fn(&mut Item)| {
        let id = p.fresh_id();
        let mut it = Item::new(id, frame, content);
        f(&mut it);
        p.panels[0].items.push(it);
    };
    for i in 0..9u8 {
        let (x, y) = (0.4 + (i % 3) as f32 * 2.7, 0.4 + (i / 3) as f32 * 1.25);
        add(&mut p, Frame::new(x, y, 2.3, 0.95), Content::Shape { outline: Outline::Rounded }, &move |it| {
            it.fill = Rgba::hex("#eef4fb");
            it.stroke = Rgba::hex("#2f6fb0");
            it.stroke_pt = 2.5;
            it.line_style = i;
        });
    }
    add(&mut p, Frame::new(0.6, 4.3, 7.3, 0.3), Content::Line, &|it| {
        it.stroke = Rgba::hex("#d1495b");
        it.stroke_pt = 3.0;
        it.line_style = 6;
    });
    let look = TextLook { color: Rgba::hex("#e8743b"), family: crate::fonts::SCRIPT_FAMILY.into(), align: Align::Center, ..Default::default() };
    add(&mut p, Frame::new(0.5, 4.7, 7.5, 1.5), Content::Headline { text: "Headline".into(), look, flair: Flair::BlendDown, warp: Warp::Wave, second: Rgba::hex("#d1495b"), outline: Rgba::hex("#111111"), stretch: false, bend: 1.0, bulge: 0.0 }, &|_| {});
    for i in 0..12u8 {
        let (x, y) = (0.4 + (i % 2) as f32 * 4.0, 6.4 + (i / 2) as f32 * 0.75);
        add(&mut p, Frame::new(x, y, 3.6, 0.6), Content::Ornament { style: i }, &|it| {
            it.stroke = Rgba::hex("#333333");
            it.fill = Rgba::hex("#c9a227");
            it.stroke_pt = 2.0;
        });
    }
    p
}

