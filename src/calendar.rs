//! Calendar maker: month grids that fill in their own dates, with holidays
//! and birthdays from the address book written in, one page per month.

use crate::model::*;

pub const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const DAYS: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];

/// Days since 1970-01-01 for a date (proleptic Gregorian).
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// The date `days` after 1970-01-01: (year, month 1–12, day).
pub fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    ((yoe + era * 400) as i32 + if m <= 2 { 1 } else { 0 }, m, d)
}

/// 0 = Sunday … 6 = Saturday.
pub fn weekday(y: i32, m: u32, d: u32) -> u32 {
    (days_from_civil(y, m, d) + 4).rem_euclid(7) as u32
}

pub fn days_in_month(y: i32, m: u32) -> u32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    (days_from_civil(ny, nm, 1) - days_from_civil(y, m, 1)) as u32
}

pub fn today() -> (i32, u32, u32) {
    civil_from_days((crate::platform::now_ms() / 86_400_000) as i64)
}

/// The `n`th given weekday of a month (n = 1…4), or the last one (n = 0).
fn nth_weekday(y: i32, m: u32, wd: u32, n: u32) -> u32 {
    if n == 0 {
        let last = days_in_month(y, m);
        last - (weekday(y, m, last) + 7 - wd) % 7
    } else {
        1 + (wd + 7 - weekday(y, m, 1)) % 7 + 7 * (n - 1)
    }
}

/// Easter Sunday (Gregorian): (month, day).
fn easter(y: i32) -> (u32, u32) {
    let (a, b, c) = (y % 19, y / 100, y % 100);
    let (d, e) = (b / 4, b % 4);
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let (i, k) = (c / 4, c % 4);
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    (((h + l - 7 * m + 114) / 31) as u32, ((h + l - 7 * m + 114) % 31 + 1) as u32)
}

/// Common US holidays and observances in a month: (day, name).
pub fn holidays(y: i32, m: u32) -> Vec<(u32, &'static str)> {
    let mut v = Vec::new();
    match m {
        1 => {
            v.push((1, "New Year's Day"));
            v.push((nth_weekday(y, 1, 1, 3), "Martin Luther King Jr. Day"));
        }
        2 => {
            v.push((14, "Valentine's Day"));
            v.push((nth_weekday(y, 2, 1, 3), "Presidents' Day"));
        }
        3 => v.push((17, "St. Patrick's Day")),
        5 => {
            v.push((nth_weekday(y, 5, 0, 2), "Mother's Day"));
            v.push((nth_weekday(y, 5, 1, 0), "Memorial Day"));
        }
        6 => {
            v.push((14, "Flag Day"));
            v.push((nth_weekday(y, 6, 0, 3), "Father's Day"));
            v.push((19, "Juneteenth"));
        }
        7 => v.push((4, "Independence Day")),
        9 => v.push((nth_weekday(y, 9, 1, 1), "Labor Day")),
        10 => {
            v.push((nth_weekday(y, 10, 1, 2), "Columbus Day"));
            v.push((31, "Halloween"));
        }
        11 => {
            v.push((11, "Veterans Day"));
            v.push((nth_weekday(y, 11, 4, 4), "Thanksgiving"));
        }
        12 => {
            v.push((24, "Christmas Eve"));
            v.push((25, "Christmas Day"));
            v.push((31, "New Year's Eve"));
        }
        _ => {}
    }
    let (em, ed) = easter(y);
    if em == m {
        v.push((ed, "Easter"));
    }
    v
}

/// Choices for the calendar maker.
#[derive(Clone, Debug, PartialEq)]
pub struct CalendarOptions {
    pub year: i32,
    /// First month, 1–12.
    pub month: u32,
    /// How many months (one page each).
    pub months: u32,
    pub monday_first: bool,
    pub holidays: bool,
    pub birthdays: bool,
    /// Leave the top half of each page for a picture (tall pages).
    pub picture_space: bool,
    /// Color scheme (see SCHEMES).
    pub scheme: usize,
}

impl Default for CalendarOptions {
    fn default() -> Self {
        let (y, m, _) = today();
        // Starting next month is the usual wish for a new calendar.
        let (year, month) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
        Self { year, month, months: 1, monday_first: false, holidays: true, birthdays: true, picture_space: false, scheme: 0 }
    }
}

/// Name, title color, header fill, header text, weekend fill.
pub const SCHEMES: [(&str, &str, &str, &str, &str); 5] = [
    ("Sky", "#2f6fb0", "#2f6fb0", "#ffffff", "#eef4fb"),
    ("Garden", "#3d7a46", "#5c9e5a", "#ffffff", "#eef6ec"),
    ("Berry", "#a33b62", "#c2577d", "#ffffff", "#fbeef3"),
    ("Sunshine", "#c77d12", "#f2b631", "#3a2a00", "#fdf6e3"),
    ("Classic", "#222222", "#444444", "#ffffff", "#f3f3f3"),
];

/// Fill the project's pages with month grids, one page per month.
/// `people`: (month, day, name) birthdays.
pub fn fill(p: &mut Project, o: &CalendarOptions, people: &[(u32, u32, String)]) {
    let (w, h) = (p.width, p.height);
    let (_, title_c, head_fill, head_text, weekend) = SCHEMES[o.scheme.min(SCHEMES.len() - 1)];
    p.panels.clear();
    for k in 0..o.months.clamp(1, 12) {
        let idx = (o.month as i32 - 1) + k as i32;
        let (y, m) = (o.year + idx.div_euclid(12), (idx.rem_euclid(12) + 1) as u32);
        let mut panel = Panel { name: format!("{} {y}", MONTHS[m as usize - 1]), background: Rgba::WHITE, items: Vec::new(), pattern: 0, pattern_color: Rgba::hex("#dddddd"), pattern_scale: 1.0, guides: Vec::new() };
        let mut add = |p: &mut Project, f: Frame, c: Content, style: &dyn Fn(&mut Item)| {
            let id = p.fresh_id();
            let mut it = Item::new(id, f, c);
            style(&mut it);
            panel.items.push(it);
        };
        let margin = 0.4;
        let mut top = margin;
        if o.picture_space && h > w {
            // Room for a photo above the month.
            add(p, Frame::new(margin, top, w - 2.0 * margin, h * 0.42), Content::Text { text: "Put a photo here:\nInsert > From My Art, or Pictures on the left.\n(Delete this box once your photo is in.)".into(), look: TextLook { size: 14.0, align: Align::Center, valign: VAlign::Middle, color: Rgba::hex("#8a8a8a"), ..Default::default() }, shape: Some(Outline::Rounded), fit: false, runs: Vec::new() }, &|i| {
                i.stroke = Rgba::hex("#bbbbbb");
                i.stroke_pt = 1.0;
                i.fill = Rgba::hex("#f7f7f7");
            });
            top += h * 0.42 + 0.15;
        }
        let title_h = 0.75;
        let title = format!("{} {y}", MONTHS[m as usize - 1]);
        add(p, Frame::new(margin, top, w - 2.0 * margin, title_h), Content::Headline { text: title, look: TextLook { color: Rgba::hex(title_c), bold: true, align: Align::Center, ..Default::default() }, flair: Flair::Plain, warp: Warp::Straight, second: Rgba::hex(title_c), outline: Rgba::hex(title_c), stretch: false, bend: 1.0, bulge: 0.0 }, &|_| {});
        top += title_h + 0.1;
        let cw = (w - 2.0 * margin) / 7.0;
        let head_h = 0.32;
        for col in 0..7u32 {
            let wd = if o.monday_first { (col + 1) % 7 } else { col };
            let name = if cw < 0.9 { &DAYS[wd as usize][..3] } else { DAYS[wd as usize] };
            add(p, Frame::new(margin + col as f32 * cw, top, cw, head_h), Content::Text { text: name.into(), look: TextLook { size: 11.0, bold: true, align: Align::Center, valign: VAlign::Middle, color: Rgba::hex(head_text), ..Default::default() }, shape: Some(Outline::Rectangle), fit: false, runs: Vec::new() }, &|i| {
                i.fill = Rgba::hex(head_fill);
                i.stroke = Rgba::hex(head_fill);
            });
        }
        top += head_h;
        let first = weekday(y, m, 1);
        let lead = if o.monday_first { (first + 6) % 7 } else { first };
        let n = days_in_month(y, m);
        let rows = (lead + n).div_ceil(7);
        let ch = (h - margin - top) / rows as f32;
        let notes: Vec<(u32, String)> = {
            let mut v: Vec<(u32, String)> = if o.holidays { holidays(y, m).into_iter().map(|(d, s)| (d, s.to_string())).collect() } else { Vec::new() };
            if o.birthdays {
                v.extend(people.iter().filter(|(pm, _, _)| *pm == m).map(|(_, d, name)| (*d, format!("{name}'s birthday"))));
            }
            v
        };
        for cell in 0..rows * 7 {
            let (row, col) = (cell / 7, cell % 7);
            let wd = if o.monday_first { (col + 1) % 7 } else { col };
            let (x, yy) = (margin + col as f32 * cw, top + row as f32 * ch);
            let day = cell as i64 - lead as i64 + 1;
            let in_month = day >= 1 && day <= n as i64;
            let weekend_day = wd == 0 || wd == 6;
            add(p, Frame::new(x, yy, cw, ch), Content::Shape { outline: Outline::Rectangle }, &|i| {
                i.fill = if !in_month { Rgba::hex("#fafafa") } else if weekend_day { Rgba::hex(weekend) } else { Rgba::WHITE };
                i.stroke = Rgba::hex("#9a9a9a");
                i.stroke_pt = 0.75;
            });
            if in_month {
                let d = day as u32;
                let text_c = if weekend_day { Rgba::hex(title_c) } else { Rgba::hex("#333333") };
                add(p, Frame::new(x + 0.04, yy + 0.02, cw - 0.08, 0.3), Content::Text { text: d.to_string(), look: TextLook { size: 12.0, bold: true, color: text_c, ..Default::default() }, shape: None, fit: false, runs: Vec::new() }, &|_| {});
                let here: Vec<&str> = notes.iter().filter(|(nd, _)| *nd == d).map(|(_, s)| s.as_str()).collect();
                if !here.is_empty() {
                    add(p, Frame::new(x + 0.04, yy + 0.3, cw - 0.08, (ch - 0.32).max(0.2)), Content::Text { text: here.join("\n"), look: TextLook { size: 9.0, color: Rgba::hex("#555555"), ..Default::default() }, shape: None, fit: false, runs: Vec::new() }, &|_| {});
                }
            }
        }
        p.panels.push(panel);
    }
}

/// Birthdays in the address book as (month, day, name); "MM-DD" dates.
pub fn birthdays(contacts: &[crate::settings::Contact]) -> Vec<(u32, u32, String)> {
    contacts
        .iter()
        .filter_map(|c| {
            let (m, d) = c.birthday.trim().split_once(['-', '/'])?;
            let (m, d) = (m.trim().parse::<u32>().ok()?, d.trim().parse::<u32>().ok()?);
            ((1..=12).contains(&m) && (1..=31).contains(&d)).then(|| (m, d, c.name.split_whitespace().next().unwrap_or(&c.name).to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(civil_from_days(days_from_civil(2026, 11, 22)), (2026, 11, 22));
        assert_eq!(weekday(2026, 11, 22), 0); // a Sunday
        assert_eq!(days_in_month(2028, 2), 29);
        assert_eq!(days_in_month(2027, 2), 28);
    }

    #[test]
    fn holidays_land_on_the_right_days() {
        assert!(holidays(2026, 11).contains(&(26, "Thanksgiving")));
        assert!(holidays(2027, 5).contains(&(31, "Memorial Day")));
        assert!(holidays(2027, 3).contains(&(28, "Easter")));
        assert!(holidays(2026, 4).contains(&(5, "Easter")));
    }

    #[test]
    fn a_month_fills_in() {
        let mut p = Project::new(crate::kinds::Kind::Calendar, true);
        let o = CalendarOptions { year: 2027, month: 2, months: 3, ..Default::default() };
        fill(&mut p, &o, &[(2, 14, "Ann".into())]);
        assert_eq!(p.panels.len(), 3);
        assert_eq!(p.panels[0].name, "February 2027");
        let words: Vec<String> = p.panels[0].items.iter().filter_map(|i| if let Content::Text { text, .. } = &i.content { Some(text.clone()) } else { None }).collect();
        assert!(words.iter().any(|t| t == "28") && !words.iter().any(|t| t == "29"));
        assert!(words.iter().any(|t| t.contains("Ann's birthday") && t.contains("Valentine's Day")));
    }
}
