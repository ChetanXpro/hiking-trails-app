use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub struct Caution {
    kind: &'static str,
    message: String,
    source_url: String,
}

pub fn derive_cautions<'a>(ways: impl IntoIterator<Item = (i64, &'a Value)>) -> Vec<Caution> {
    let mut notes = BTreeMap::new();
    for (id, tags) in ways {
        let value = |key: &str| {
            tags.get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
        };
        let mut add = |kind, message: String| {
            notes.entry((kind, message.clone())).or_insert(Caution {
                kind,
                message,
                source_url: format!("https://www.openstreetmap.org/way/{id}"),
            });
        };
        // Pedestrian-specific access overrides the general access tag.
        match value("foot").or_else(|| value("access")) {
            Some("no") => add("access", "A section is mapped as closed to pedestrians.".into()),
            Some("private") => add("access", "A section is mapped as private access.".into()),
            Some("permit") => add("access", "A section is mapped as requiring a permit.".into()),
            Some("customers") | Some("destination") => add("access", "A section has restricted access; check the source before using it as a through route.".into()),
            _ => {}
        }
        if let Some(condition) = value("foot:conditional").or_else(|| value("access:conditional")) {
            add(
                "conditional_access",
                format!(
                    "Mapped access condition: {}",
                    condition.chars().take(240).collect::<String>()
                ),
            );
        }
        if matches!(value("trail_visibility"), Some("bad" | "horrible" | "no")) {
            add(
                "navigation",
                "Some sections are mapped as difficult to follow on the ground.".into(),
            );
        }
        if value("seasonal").is_some_and(|s| s != "no") {
            add(
                "seasonal",
                "A section is marked seasonal; check local conditions.".into(),
            );
        }
        if value("ford").is_some_and(|s| s != "no") || value("highway") == Some("ford") {
            add(
                "water_crossing",
                "An unbridged water crossing is mapped on this route.".into(),
            );
        }
    }
    notes.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn respects_foot_overrides_and_never_guesses_access() {
        assert!(derive_cautions([(1, &json!({"access":"private","foot":"yes"}))]).is_empty());
        assert!(derive_cautions([(1, &json!({}))]).is_empty());
        let notes = derive_cautions([(
            2,
            &json!({"access":"yes","foot":"private","trail_visibility":"bad"}),
        )]);
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].kind, "access");
    }
    #[test]
    fn deduplicates_route_notes_but_retains_a_source() {
        let tags = json!({"seasonal":"yes","ford":"yes"});
        let notes = derive_cautions([(7, &tags), (8, &tags)]);
        assert_eq!(notes.len(), 2);
        assert!(notes.iter().all(|n| n.source_url.ends_with("/7")));
    }
}
