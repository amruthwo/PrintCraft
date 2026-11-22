//! Address merge: fields like «Name» or «Address» in text are filled in
//! from the address book when printing, one copy for each chosen person:
//! labels take one person per label, envelopes and everything else one
//! person per set of sheets.

use crate::model::{Content, Item, Project, Span};
use crate::settings::Contact;

/// The fields that can go in text: what's typed, and what it means.
pub const FIELDS: [(&str, &str); 8] = [
    ("«Address»", "The whole address (name, street, city)"),
    ("«Name»", "Name"),
    ("«Street»", "Street"),
    ("«City»", "City"),
    ("«State»", "State or region"),
    ("«Zip»", "ZIP or postcode"),
    ("«Phone»", "Phone"),
    ("«Email»", "Email"),
];

fn value(c: &Contact, field: &str) -> String {
    match field {
        "«Address»" => c.mailing_block(),
        "«Name»" => c.name.clone(),
        "«Street»" => c.street.clone(),
        "«City»" => c.city.clone(),
        "«State»" => c.region.clone(),
        "«Zip»" => c.postcode.clone(),
        "«Phone»" => c.phone.clone(),
        "«Email»" => c.email.clone(),
        _ => String::new(),
    }
}

fn item_has_fields(it: &Item) -> bool {
    match &it.content {
        Content::Text { text, .. } | Content::Headline { text, .. } => FIELDS.iter().any(|(f, _)| text.contains(f)),
        Content::Group { items, .. } => items.iter().any(item_has_fields),
        _ => false,
    }
}

/// Whether any page has a merge field in it.
pub fn has_fields(p: &Project) -> bool {
    p.panels.iter().any(|pn| pn.items.iter().any(item_has_fields))
}

/// Replace the fields in `text` with one person's details, moving styled
/// stretches of letters along so they still cover the same words.
pub fn fill_text(text: &str, runs: &mut [Span], c: &Contact) -> String {
    let mut out = text.to_string();
    for (field, _) in FIELDS {
        while let Some(byte_at) = out.find(field) {
            let at = out[..byte_at].chars().count();
            let old_len = field.chars().count();
            let new = value(c, field);
            let delta = new.chars().count() as isize - old_len as isize;
            out.replace_range(byte_at..byte_at + field.len(), &new);
            let end = at + old_len;
            for r in runs.iter_mut() {
                let shift = |v: usize| if v >= end { (v as isize + delta).max(0) as usize } else if v > at { at + (v - at).min(new.chars().count()) } else { v };
                r.start = shift(r.start);
                r.end = shift(r.end);
            }
        }
    }
    out
}

fn fill_item(it: &mut Item, c: &Contact) {
    match &mut it.content {
        Content::Text { text, runs, .. } => *text = fill_text(text, runs, c),
        Content::Headline { text, .. } => *text = fill_text(text, &mut [], c),
        Content::Group { items, .. } => items.iter_mut().for_each(|i| fill_item(i, c)),
        _ => {}
    }
}

/// A copy of the project with one person's details filled in.
pub fn fill(p: &Project, c: &Contact) -> Project {
    let mut q = p.clone();
    for pn in &mut q.panels {
        pn.items.iter_mut().for_each(|i| fill_item(i, c));
    }
    q
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ann() -> Contact {
        Contact { name: "Ann Lee".into(), street: "12 Oak St".into(), city: "Mesa".into(), region: "AZ".into(), postcode: "85201".into(), ..Default::default() }
    }

    #[test]
    fn fields_fill_in_and_styles_follow() {
        // "Dear «Name»," with the name in bold (letters 5..11).
        let mut runs = vec![Span { start: 5, end: 11, bold: Some(true), ..Default::default() }, Span { start: 11, end: 12, italic: Some(true), ..Default::default() }];
        let t = fill_text("Dear «Name»,", &mut runs, &ann());
        assert_eq!(t, "Dear Ann Lee,");
        assert_eq!((runs[0].start, runs[0].end), (5, 12));
        assert_eq!((runs[1].start, runs[1].end), (12, 13));
    }

    #[test]
    fn whole_address() {
        assert_eq!(fill_text("«Address»", &mut [], &ann()), "Ann Lee\n12 Oak St\nMesa, AZ 85201");
    }
}
