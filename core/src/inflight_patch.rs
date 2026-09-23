// SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "patch-inflight")]
use std::sync::Arc;

#[cfg(feature = "patch-inflight")]
use nv_redfish_patch_inflight::patch_registry::InflightPatchRegistry;
#[cfg(feature = "patch-inflight")]
use nv_redfish_patch_inflight::with_registry;
#[cfg(not(feature = "patch-inflight"))]
use serde_json::Value;

/// Shared in-flight patches, or a zero-sized no-op without `patch-inflight`.
#[derive(Clone, Default)]
pub struct MaybeInflightPatchRegistry(
    #[cfg(feature = "patch-inflight")] Arc<InflightPatchRegistry>,
    #[cfg(not(feature = "patch-inflight"))] (),
);

#[cfg(feature = "patch-inflight")]
impl From<InflightPatchRegistry> for MaybeInflightPatchRegistry {
    fn from(registry: InflightPatchRegistry) -> Self {
        Self(Arc::new(registry))
    }
}

impl MaybeInflightPatchRegistry {
    /// Run synchronous response deserialization with these patches active.
    ///
    /// With `patch-inflight`, restores the previous context on return or panic.
    /// Without it, calls `f` directly. The context only applies during this call,
    /// not while polling a future returned by `f`.
    #[inline]
    pub fn with_context<T>(&self, f: impl FnOnce() -> T) -> T {
        #[cfg(feature = "patch-inflight")]
        {
            with_registry(self.0.clone(), f)
        }
        #[cfg(not(feature = "patch-inflight"))]
        {
            f()
        }
    }
}

#[cfg(feature = "patch-inflight")]
pub use nv_redfish_patch_inflight::patch_inflight;

/// Return the value unchanged when the `patch-inflight` feature is disabled.
#[cfg(not(feature = "patch-inflight"))]
#[inline]
#[must_use]
pub const fn patch_inflight(value: Value) -> Value {
    value
}

#[cfg(all(test, not(feature = "patch-inflight")))]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn disabled_context_is_zero_sized_and_preserves_response() {
        assert_eq!(std::mem::size_of::<MaybeInflightPatchRegistry>(), 0);
        let value = json!({
            "@odata.id": "/redfish/v1/Managers/1/NetworkProtocol",
            "NTP": { "NTPServers": [null] }
        });
        let result =
            MaybeInflightPatchRegistry::default().with_context(|| patch_inflight(value.clone()));
        assert_eq!(result, value);
    }
}
