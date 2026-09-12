use crate::config::Provider;
use std::collections::HashMap;

/// Merges local and remote provider lists based on the provider `name`.
/// - Providers unique to either list are preserved.
/// - When a provider exists in both lists:
///   - If remote has non-empty fields (like remark or claude_models) and local doesn't, remote values are adopted.
///   - Connection parameters (base_url, api_key, model, wire_api) from remote are preferred if they differ,
///     as remote represents the synced cloud state.
///   - The latest test metrics (latency_ms, last_ok, last_test) are preserved from whichever has the more recent test.
pub fn merge_providers(local: &[Provider], remote: &[Provider]) -> Vec<Provider> {
    let mut map: HashMap<String, Provider> = HashMap::new();

    // Insert all local providers first
    for p in local {
        map.insert(p.name.clone(), p.clone());
    }

    // Merge in remote providers
    for r in remote {
        if let Some(l) = map.get_mut(&r.name) {
            // Update connection parameters from remote
            l.base_url = r.base_url.clone();
            l.api_key = r.api_key.clone();
            l.model = r.model.clone();
            l.wire_api = r.wire_api.clone();

            // Remarks: if remote has a remark, prefer it; otherwise keep local
            if r.remark.is_some() {
                l.remark = r.remark.clone();
            }

            // Claude models configuration
            if r.claude_models.is_some() {
                l.claude_models = r.claude_models.clone();
            }

            // Test results: pick the more recent test
            match (&l.last_test, &r.last_test) {
                (Some(lt), Some(rt)) => {
                    if rt > lt {
                        l.last_test = r.last_test;
                        l.latency_ms = r.latency_ms;
                        l.last_ok = r.last_ok;
                    }
                }
                (None, Some(_)) => {
                    l.last_test = r.last_test;
                    l.latency_ms = r.latency_ms;
                    l.last_ok = r.last_ok;
                }
                _ => {}
            }
        } else {
            // Provider is only present on remote, add it
            map.insert(r.name.clone(), r.clone());
        }
    }

    // Maintain a stable order: local providers in original order, then newly added remote providers
    let mut result = Vec::with_capacity(map.len());
    let mut added = std::collections::HashSet::new();

    for p in local {
        if let Some(item) = map.remove(&p.name) {
            result.push(item);
            added.insert(p.name.clone());
        }
    }

    for r in remote {
        if !added.contains(&r.name) {
            if let Some(item) = map.remove(&r.name) {
                result.push(item);
                added.insert(r.name.clone());
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_disjoint_providers() {
        let local = vec![Provider {
            name: "p1".to_string(),
            base_url: "https://api1.com".to_string(),
            api_key: "k1".to_string(),
            model: "m1".to_string(),
            wire_api: "responses".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }];

        let remote = vec![Provider {
            name: "p2".to_string(),
            base_url: "https://api2.com".to_string(),
            api_key: "k2".to_string(),
            model: "m2".to_string(),
            wire_api: "responses".to_string(),
            remark: Some("backup".to_string()),
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }];

        let merged = merge_providers(&local, &remote);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].name, "p1");
        assert_eq!(merged[1].name, "p2");
        assert_eq!(merged[1].remark, Some("backup".to_string()));
    }

    #[test]
    fn test_merge_overlapping_providers() {
        let local = vec![Provider {
            name: "common".to_string(),
            base_url: "https://old-url.com".to_string(),
            api_key: "old-key".to_string(),
            model: "old-model".to_string(),
            wire_api: "responses".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }];

        let remote = vec![Provider {
            name: "common".to_string(),
            base_url: "https://new-url.com".to_string(),
            api_key: "new-key".to_string(),
            model: "new-model".to_string(),
            wire_api: "chat".to_string(),
            remark: Some("updated remark".to_string()),
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
        }];

        let merged = merge_providers(&local, &remote);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "common");
        assert_eq!(merged[0].base_url, "https://new-url.com");
        assert_eq!(merged[0].api_key, "new-key");
        assert_eq!(merged[0].wire_api, "chat");
        assert_eq!(merged[0].remark, Some("updated remark".to_string()));
    }
}
