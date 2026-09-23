// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

use std::cell::RefCell;
use std::sync::Arc;

use serde_json::Value;

use crate::patch_registry::InflightPatchRegistry;

/// Run synchronous response deserialization with the given registry active.
///
/// Restores the previous context on return or panic, including for nested
/// calls. The context is thread-local: do not return a future expecting it
/// to remain active when the future is polled.
pub fn with_registry<T>(registry: Arc<InflightPatchRegistry>, f: impl FnOnce() -> T) -> T {
    let _guard = ContextGuard(CURRENT_REGISTRY.replace(Some(registry)));
    f()
}

thread_local! {
    static CURRENT_REGISTRY: RefCell<Option<Arc<InflightPatchRegistry>>> = const { RefCell::new(None) };
}

struct ContextGuard(Option<Arc<InflightPatchRegistry>>);

impl Drop for ContextGuard {
    fn drop(&mut self) {
        CURRENT_REGISTRY.set(self.0.take());
    }
}

/// Apply the active context's patches, or return the value unchanged if absent.
#[must_use]
pub fn patch_inflight(value: Value) -> Value {
    let registry = CURRENT_REGISTRY.with_borrow(Clone::clone);
    match registry {
        Some(registry) => registry.patch_inflight(value),
        None => value,
    }
}

#[cfg(test)]
mod tests {
    use std::panic::catch_unwind;

    use serde_json::json;

    use super::*;

    fn with_default_registry<T>(f: impl FnOnce() -> T) -> T {
        with_registry(Arc::new(InflightPatchRegistry::default()), f)
    }

    fn response() -> Value {
        json!({
            "@odata.id": "/redfish/v1/Managers/1/NetworkProtocol",
            "NTP": { "NTPServers": [null, "ntp.example.com"] }
        })
    }

    fn assert_patched() {
        assert_eq!(
            patch_inflight(response())["NTP"]["NTPServers"],
            json!(["", "ntp.example.com"])
        );
    }

    #[test]
    fn patches_only_within_context_and_restores_nested_context() {
        assert_eq!(patch_inflight(response()), response());
        with_default_registry(|| {
            assert_patched();
            let empty = Arc::new(InflightPatchRegistry::new(vec![]).unwrap());
            with_registry(empty, || assert_eq!(patch_inflight(response()), response()));
            assert_patched();
        });
        assert_eq!(patch_inflight(response()), response());
    }

    #[test]
    fn restores_context_after_deserialization_error() {
        let result = with_default_registry(|| {
            assert_patched();
            serde_json::from_value::<Vec<String>>(json!([null]))
        });
        assert!(result.is_err());
        assert_eq!(patch_inflight(response()), response());
    }

    #[test]
    fn restores_context_after_panic() {
        with_default_registry(|| {
            assert!(catch_unwind(|| {
                with_registry(
                    Arc::new(InflightPatchRegistry::new(vec![]).unwrap()),
                    || {
                        panic!("deserialization panicked");
                    },
                );
            })
            .is_err());
            assert_patched();
        });
        assert_eq!(patch_inflight(response()), response());
    }

    #[test]
    fn patch_callback_can_enter_another_context() {
        use crate::patch_registry::InflightPatch;

        let registry = InflightPatchRegistry::new(vec![InflightPatch {
            priority: 0,
            name: "nested".into(),
            oid_predicate: "*".into(),
            patch: Arc::new(|value| {
                with_default_registry(assert_patched);
                value
            }),
        }])
        .unwrap();
        with_registry(Arc::new(registry), || {
            assert_eq!(patch_inflight(response()), response());
        });
    }
}
