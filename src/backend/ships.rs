use std::collections::HashMap;

const KNOWN_PREFIXES: &[&str] = &[
    "VFP/(D)ROU ",
    "VFP/(D)LOU ",
    "FP/(D)ROU ",
    "FP/(D)GOU ",
    "FP/(D)LOU ",
    "(D)GOU ",
    "(D)ROU ",
    "(ex-)GCU ",
    "GOU/PS ",
    "GCU ",
    "GCV ",
    "GOU ",
    "GSV ",
    "LCU ",
    "LOU ",
    "LSV ",
    "MSV ",
    "ROU ",
    "OU/e ",
];

const MAX_HOSTNAME_LEN: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipName {
    pub display_name: String,
    pub hostname: String,
}

fn strip_prefix(name: &str) -> &str {
    for prefix in KNOWN_PREFIXES {
        if name.starts_with(prefix) {
            return &name[prefix.len()..];
        }
    }
    name
}

fn slugify(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            ' ' => result.push('-'),
            '-' => result.push('-'),
            '\'' | ',' | '.' | '!' | '?' | '(' | ')' | '\u{2026}' => {}
            _ => {
                let lower = ch.to_ascii_lowercase();
                if lower.is_ascii_lowercase() || lower.is_ascii_digit() {
                    result.push(lower);
                }
            }
        }
    }
    while result.contains("--") {
        result = result.replace("--", "-");
    }
    result.trim_start_matches('-').trim_end_matches('-').to_string()
}

fn truncate_at_word_boundary(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    if let Some(pos) = s[..=max_len].rfind('-') {
        s[..pos].to_string()
    } else {
        s[..max_len].to_string()
    }
}

pub fn derive_hostname(full_name: &str) -> String {
    let stripped = strip_prefix(full_name);
    let slug = slugify(stripped);
    truncate_at_word_boundary(&slug, MAX_HOSTNAME_LEN)
}

pub fn random_ship_name() -> ShipName {
    let display_name = gsv_culture_ships::random();
    let hostname = derive_hostname(&display_name);
    ShipName {
        display_name,
        hostname,
    }
}

pub fn all_ship_names() -> Vec<ShipName> {
    let all = gsv_culture_ships::ships_as_slice();
    let mut hostnames: Vec<String> = Vec::with_capacity(all.len());
    let mut collision_counts: HashMap<String, usize> = HashMap::new();

    for name in all {
        let base_hostname = derive_hostname(name);
        let count = collision_counts.entry(base_hostname.clone()).or_insert(0);
        *count += 1;
        hostnames.push(base_hostname);
    }

    let mut final_counts: HashMap<String, usize> = HashMap::new();
    all.iter()
        .enumerate()
        .map(|(i, name)| {
            let base = &hostnames[i];
            let total = collision_counts[base];
            let hostname = if total > 1 {
                let suffix = final_counts.entry(base.clone()).or_insert(1);
                let result = format!("{}-{}", base, suffix);
                *suffix += 1;
                result
            } else {
                base.clone()
            };
            ShipName {
                display_name: name.to_string(),
                hostname,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_prefix() {
        assert_eq!(strip_prefix("GSV Anticipation Of A New Lover's Arrival, The"), "Anticipation Of A New Lover's Arrival, The");
        assert_eq!(strip_prefix("GCU Arbitrary"), "Arbitrary");
        assert_eq!(strip_prefix("ROU Killing Time"), "Killing Time");
        assert_eq!(strip_prefix("(D)GOU Limiting Factor"), "Limiting Factor");
        assert_eq!(strip_prefix("FP/(D)ROU Refreshingly Unconcerned With The Vulgar Exigencies Of Veracity"), "Refreshingly Unconcerned With The Vulgar Exigencies Of Veracity");
        assert_eq!(strip_prefix("Beastly To The Animals"), "Beastly To The Animals");
        assert_eq!(strip_prefix("(ex-)GCU Smile Tolerantly"), "Smile Tolerantly");
        assert_eq!(strip_prefix("OU/e Mistake Not\u{2026}"), "Mistake Not\u{2026}");
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Anticipation Of A New Lover's Arrival, The"), "anticipation-of-a-new-lovers-arrival-the");
        assert_eq!(slugify("Arbitrary"), "arbitrary");
        assert_eq!(slugify("Killing Time"), "killing-time");
        assert_eq!(slugify("Grey Area"), "grey-area");
        assert_eq!(slugify("Boo!"), "boo");
        assert_eq!(slugify("Fixed Grin"), "fixed-grin");
        assert_eq!(slugify("Bodhisattva, OAQS"), "bodhisattva-oaqs");
    }

    #[test]
    fn test_derive_hostname_short() {
        assert_eq!(derive_hostname("GCU Arbitrary"), "arbitrary");
        assert_eq!(derive_hostname("ROU Killing Time"), "killing-time");
        assert_eq!(derive_hostname("GCU Grey Area"), "grey-area");
        assert_eq!(derive_hostname("GCU Boo!"), "boo");
        assert_eq!(derive_hostname("GSV Zero Gravitas"), "zero-gravitas");
    }

    #[test]
    fn test_derive_hostname_truncation() {
        let hostname = derive_hostname("GSV Anticipation Of A New Lover's Arrival, The");
        assert!(hostname.len() <= MAX_HOSTNAME_LEN);
        assert!(!hostname.ends_with('-'));
    }

    #[test]
    fn test_derive_hostname_very_long() {
        let hostname = derive_hostname("FP/(D)ROU Refreshingly Unconcerned With The Vulgar Exigencies Of Veracity");
        assert!(hostname.len() <= MAX_HOSTNAME_LEN);
        assert!(!hostname.ends_with('-'));
    }

    #[test]
    fn test_all_ship_names_no_duplicate_hostnames() {
        let names = all_ship_names();
        let mut seen = HashMap::new();
        for name in &names {
            if let Some(existing) = seen.insert(name.hostname.clone(), &name.display_name) {
                panic!(
                    "Duplicate hostname '{}' from '{}' and '{}'",
                    name.hostname, existing, name.display_name
                );
            }
        }
    }

    #[test]
    fn test_all_ship_names_hostnames_valid() {
        let names = all_ship_names();
        for name in &names {
            assert!(
                name.hostname.len() <= MAX_HOSTNAME_LEN,
                "hostname too long: '{}' ({} chars) from '{}'",
                name.hostname,
                name.hostname.len(),
                name.display_name
            );
            assert!(
                !name.hostname.is_empty(),
                "empty hostname from '{}'",
                name.display_name
            );
            assert!(
                name.hostname
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "invalid chars in hostname '{}' from '{}'",
                name.hostname,
                name.display_name
            );
            assert!(
                !name.hostname.starts_with('-'),
                "hostname starts with '-': '{}' from '{}'",
                name.hostname,
                name.display_name
            );
            assert!(
                !name.hostname.ends_with('-'),
                "hostname ends with '-': '{}' from '{}'",
                name.hostname,
                name.display_name
            );
        }
    }

    #[test]
    fn test_print_all_ship_names() {
        let names = all_ship_names();
        for name in &names {
            eprintln!("{:<50} -> {}", name.display_name, name.hostname);
        }
        eprintln!("\nTotal ships: {}", names.len());
    }
}