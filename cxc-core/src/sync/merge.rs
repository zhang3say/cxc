use crate::config::Provider;
use std::collections::HashMap;

/// Merges local and remote provider lists based on the provider `name`.
/// - Providers unique to either list are preserved.
/// - When a provider exists in both lists:
///   - Remarks, claude_models, grok_web_search, reasoning_effort, plan_reasoning_effort,
///     context_length and compact_ratio: if one side has None and the other has Some, the non-empty
///     configuration is adopted.
///   - Connection parameters (base_url, api_key, model, wire_api) and test metrics (last_test, latency_ms, last_ok)
///     are kept strictly coupled:
///     - If both sets of connection parameters are identical, the more recent test metrics are adopted.
///     - If connection parameters differ:
///       - If remote has a more recent test timestamp than local, remote's parameters and test metrics are adopted.
///       - If local has a more recent test timestamp (or equal), local's parameters and test metrics are preserved.
///       - If neither side has been tested, remote's parameters are adopted as the synced cloud baseline.
pub fn merge_providers(local: &[Provider], remote: &[Provider]) -> Vec<Provider> {
    let mut map: HashMap<String, Provider> = HashMap::new();

    // Insert all local providers first
    for p in local {
        map.insert(p.name.clone(), p.clone());
    }

    // Merge in remote providers
    for r in remote {
        if let Some(l) = map.get_mut(&r.name) {
            let connection_identical = l.base_url == r.base_url
                && l.api_key == r.api_key
                && l.model == r.model
                && l.wire_api == r.wire_api;

            if connection_identical {
                // Adopt newer test metrics if available
                match (&l.last_test, &r.last_test) {
                    (Some(lt), Some(rt)) if rt > lt => {
                        l.last_test = r.last_test;
                        l.latency_ms = r.latency_ms;
                        l.last_ok = r.last_ok;
                    }
                    (None, Some(_)) => {
                        l.last_test = r.last_test;
                        l.latency_ms = r.latency_ms;
                        l.last_ok = r.last_ok;
                    }
                    _ => {}
                }
            } else {
                // Connection parameters differ: decide which one is newer, keeping parameters and test metrics coupled
                let prefer_remote = match (&l.last_test, &r.last_test) {
                    (Some(lt), Some(rt)) => rt > lt,
                    (None, Some(_)) => true,
                    (Some(_), None) => false,
                    (None, None) => true, // default to remote if neither has test timestamp
                };

                if prefer_remote {
                    l.base_url = r.base_url.clone();
                    l.api_key = r.api_key.clone();
                    l.model = r.model.clone();
                    l.wire_api = r.wire_api.clone();
                    l.last_test = r.last_test;
                    l.latency_ms = r.latency_ms;
                    l.last_ok = r.last_ok;
                }
                // Else: preserve local connection parameters and test metrics
            }

            // Adopt remarks if local is missing them
            if l.remark.is_none() && r.remark.is_some() {
                l.remark = r.remark.clone();
            }

            // Adopt Claude models if local is missing them
            if l.claude_models.is_none() && r.claude_models.is_some() {
                l.claude_models = r.claude_models.clone();
            }

            // Adopt Grok web_search policy if local is missing it
            if l.grok_web_search.is_none() && r.grok_web_search.is_some() {
                l.grok_web_search = r.grok_web_search.clone();
            }

            if l.reasoning_effort.is_none() && r.reasoning_effort.is_some() {
                l.reasoning_effort = r.reasoning_effort.clone();
            }
            if l.plan_reasoning_effort.is_none() && r.plan_reasoning_effort.is_some() {
                l.plan_reasoning_effort = r.plan_reasoning_effort.clone();
            }
            if l.context_length.is_none() && r.context_length.is_some() {
                l.context_length = r.context_length.clone();
            }
            if l.compact_ratio.is_none() && r.compact_ratio.is_some() {
                l.compact_ratio = r.compact_ratio;
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
    use chrono::{Duration, Local};

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
            grok_web_search: None,
            reasoning_effort: None,
            plan_reasoning_effort: None,
            context_length: None,
            compact_ratio: None,
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
            grok_web_search: None,
            reasoning_effort: None,
            plan_reasoning_effort: None,
            context_length: None,
            compact_ratio: None,
        }];

        let merged = merge_providers(&local, &remote);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].name, "p1");
        assert_eq!(merged[1].name, "p2");
        assert_eq!(merged[1].remark, Some("backup".to_string()));
    }

    #[test]
    fn test_merge_overlapping_providers_preserves_newer_local() {
        let now = Local::now();
        let local = vec![Provider {
            name: "common".to_string(),
            base_url: "https://local-updated.com".to_string(),
            api_key: "local-key".to_string(),
            model: "gpt-4o".to_string(),
            wire_api: "responses".to_string(),
            remark: Some("local remark".to_string()),
            last_test: Some(now),
            latency_ms: Some(42),
            last_ok: Some(true),
            claude_models: None,
            grok_web_search: None,
            reasoning_effort: None,
            plan_reasoning_effort: None,
            context_length: None,
            compact_ratio: None,
        }];

        let remote = vec![Provider {
            name: "common".to_string(),
            base_url: "https://remote-older.com".to_string(),
            api_key: "remote-key".to_string(),
            model: "gpt-4o".to_string(),
            wire_api: "responses".to_string(),
            remark: Some("remote remark".to_string()),
            last_test: Some(now - Duration::hours(1)),
            latency_ms: Some(150),
            last_ok: Some(true),
            claude_models: None,
            grok_web_search: None,
            reasoning_effort: None,
            plan_reasoning_effort: None,
            context_length: None,
            compact_ratio: None,
        }];

        let merged = merge_providers(&local, &remote);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].base_url, "https://local-updated.com");
        assert_eq!(merged[0].latency_ms, Some(42));
    }

    #[test]
    fn test_merge_overlapping_providers_adopts_newer_remote() {
        let now = Local::now();
        let local = vec![Provider {
            name: "common".to_string(),
            base_url: "https://local-older.com".to_string(),
            api_key: "local-key".to_string(),
            model: "gpt-4o".to_string(),
            wire_api: "responses".to_string(),
            remark: None,
            last_test: Some(now - Duration::hours(2)),
            latency_ms: Some(200),
            last_ok: Some(true),
            claude_models: None,
            grok_web_search: None,
            reasoning_effort: None,
            plan_reasoning_effort: None,
            context_length: None,
            compact_ratio: None,
        }];

        let remote = vec![Provider {
            name: "common".to_string(),
            base_url: "https://remote-newer.com".to_string(),
            api_key: "remote-key".to_string(),
            model: "gpt-4o-mini".to_string(),
            wire_api: "chat".to_string(),
            remark: Some("updated remark".to_string()),
            last_test: Some(now),
            latency_ms: Some(60),
            last_ok: Some(true),
            claude_models: None,
            grok_web_search: None,
            reasoning_effort: None,
            plan_reasoning_effort: None,
            context_length: None,
            compact_ratio: None,
        }];

        let merged = merge_providers(&local, &remote);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].base_url, "https://remote-newer.com");
        assert_eq!(merged[0].model, "gpt-4o-mini");
        assert_eq!(merged[0].latency_ms, Some(60));
        assert_eq!(merged[0].remark, Some("updated remark".to_string()));
    }

    #[test]
    fn test_merge_reasoning_effort_adopts_remote_only_when_local_missing() {
        let bare = |name: &str, effort: Option<&str>, plan: Option<&str>| Provider {
            name: name.to_string(),
            base_url: "https://api.example.com".to_string(),
            api_key: "k".to_string(),
            model: "m".to_string(),
            wire_api: "responses".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
            grok_web_search: None,
            reasoning_effort: effort.map(str::to_string),
            plan_reasoning_effort: plan.map(str::to_string),
            context_length: None,
            compact_ratio: None,
        };
        let merged = merge_providers(
            &[bare("empty", None, None), bare("set", Some("max"), Some("high"))],
            &[
                bare("empty", Some("high"), Some("xhigh")),
                bare("set", Some("low"), Some("low")),
            ],
        );
        assert_eq!(merged[0].reasoning_effort.as_deref(), Some("high"));
        assert_eq!(merged[0].plan_reasoning_effort.as_deref(), Some("xhigh"));
        assert_eq!(merged[1].reasoning_effort.as_deref(), Some("max"));
        assert_eq!(merged[1].plan_reasoning_effort.as_deref(), Some("high"));
    }

    #[test]
    fn test_merge_context_settings_keep_local_when_both_set() {
        let local_empty = Provider {
            name: "empty".to_string(),
            base_url: "https://api.example.com".to_string(),
            api_key: "k".to_string(),
            model: "m".to_string(),
            wire_api: "responses".to_string(),
            remark: None,
            last_test: None,
            latency_ms: None,
            last_ok: None,
            claude_models: None,
            grok_web_search: None,
            reasoning_effort: None,
            plan_reasoning_effort: None,
            context_length: None,
            compact_ratio: None,
        };
        let mut local_set = local_empty.clone();
        local_set.name = "set".to_string();
        local_set.context_length = Some("200k".to_string());
        local_set.compact_ratio = Some(80);

        let mut remote_empty = local_empty.clone();
        remote_empty.context_length = Some("1m".to_string());
        remote_empty.compact_ratio = Some(75);
        let mut remote_set = local_set.clone();
        remote_set.context_length = Some("500k".to_string());
        remote_set.compact_ratio = Some(90);

        let merged = merge_providers(&[local_empty, local_set], &[remote_empty, remote_set]);
        assert_eq!(merged[0].context_length.as_deref(), Some("1m"));
        assert_eq!(merged[0].compact_ratio, Some(75));
        assert_eq!(merged[1].context_length.as_deref(), Some("200k"));
        assert_eq!(merged[1].compact_ratio, Some(80));
    }
}
