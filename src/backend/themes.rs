use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub const THEME_KEYS: &[&str] = &[
    "indigo-purple",
    "ocean-teal",
    "industrial-orange",
    "slate-steel",
    "dark-mode",
    "aquatic-sci-fi",
];

pub const DEFAULT_THEME: &str = "indigo-purple";

pub fn resolve_theme(input: &Option<String>, hostname: Option<&str>) -> String {
    match input {
        Some(t) if t == "random" => random_theme(hostname),
        Some(t) if THEME_KEYS.contains(&t.as_str()) => t.clone(),
        Some(t) => {
            log::warn!("Unknown theme '{}', falling back to '{}'", t, DEFAULT_THEME);
            DEFAULT_THEME.to_string()
        }
        None => DEFAULT_THEME.to_string(),
    }
}

pub fn random_theme(hostname: Option<&str>) -> String {
    let index = match hostname {
        Some(name) if !name.is_empty() => {
            let mut hasher = DefaultHasher::new();
            name.hash(&mut hasher);
            (hasher.finish() as usize) % THEME_KEYS.len()
        }
        _ => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            (now.as_nanos() as usize) % THEME_KEYS.len()
        }
    };
    THEME_KEYS[index].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_theme_explicit_valid() {
        assert_eq!(
            resolve_theme(&Some("ocean-teal".to_string()), None),
            "ocean-teal"
        );
        assert_eq!(
            resolve_theme(&Some("dark-mode".to_string()), None),
            "dark-mode"
        );
        assert_eq!(
            resolve_theme(&Some("aquatic-sci-fi".to_string()), None),
            "aquatic-sci-fi"
        );
    }

    #[test]
    fn test_resolve_theme_default() {
        assert_eq!(resolve_theme(&None, None), DEFAULT_THEME);
    }

    #[test]
    fn test_resolve_theme_invalid_falls_back() {
        assert_eq!(
            resolve_theme(&Some("nonexistent".to_string()), None),
            DEFAULT_THEME
        );
    }

    #[test]
    fn test_resolve_theme_random_with_hostname() {
        let result = resolve_theme(&Some("random".to_string()), Some("killing-time"));
        assert!(THEME_KEYS.contains(&result.as_str()));
    }

    #[test]
    fn test_resolve_theme_random_deterministic() {
        let a = resolve_theme(&Some("random".to_string()), Some("killing-time"));
        let b = resolve_theme(&Some("random".to_string()), Some("killing-time"));
        assert_eq!(a, b);
    }

    #[test]
    fn test_resolve_theme_random_different_hostnames() {
        let a = resolve_theme(&Some("random".to_string()), Some("killing-time"));
        let b = resolve_theme(&Some("random".to_string()), Some("sleeper-service"));
        let c = resolve_theme(&Some("random".to_string()), Some("grey-area"));
        assert!(THEME_KEYS.contains(&a.as_str()));
        assert!(THEME_KEYS.contains(&b.as_str()));
        assert!(THEME_KEYS.contains(&c.as_str()));
    }

    #[test]
    fn test_random_theme_no_hostname() {
        let result = random_theme(None);
        assert!(THEME_KEYS.contains(&result.as_str()));
    }

    #[test]
    fn test_random_theme_empty_hostname() {
        let result = random_theme(Some(""));
        assert!(THEME_KEYS.contains(&result.as_str()));
    }

    #[test]
    fn test_all_theme_keys_are_valid() {
        for key in THEME_KEYS {
            assert!(!key.is_empty());
            assert!(
                key.contains('-'),
                "theme key '{}' should be kebab-case",
                key
            );
        }
    }
}
