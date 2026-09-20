// SPDX-License-Identifier: Apache-2.0
//! F4 condition syntax only. Applicability evaluation belongs to a provider.
use super::{Diagnostic, MetadataValue, references};

/// Limits apply to the supplied condition, before any owned representation is made.
#[derive(Clone, Copy, Debug)]
pub struct ConditionLimits {
    pub entries: usize,
    pub string_bytes: usize,
}
impl Default for ConditionLimits {
    fn default() -> Self {
        Self {
            entries: 4096,
            string_bytes: 1024 * 1024,
        }
    }
}

/// A borrowed, syntax-checked condition; this is not an applicability decision.
#[derive(Clone, Copy, Debug)]
pub struct ValidatedCondition<'a> {
    metadata: &'a MetadataValue,
}
impl<'a> ValidatedCondition<'a> {
    pub fn as_metadata(&self) -> &'a MetadataValue {
        self.metadata
    }
}

pub fn validate_condition(
    value: &MetadataValue,
    limits: ConditionLimits,
) -> Result<ValidatedCondition<'_>, Diagnostic> {
    if limits.entries == 0 || limits.string_bytes == 0 {
        return Err(Diagnostic::error(
            "resource-limit",
            "Condition limits must be positive",
            0..0,
        ));
    }
    let invalid = || {
        Diagnostic::error(
            "condition-invalid",
            "Expected nonempty F4 condition table",
            0..0,
        )
    };
    let fields = value
        .as_table()
        .filter(|fields| !fields.is_empty() && fields.len() <= 3)
        .ok_or_else(invalid)?;
    let mut entries = limits.entries;
    let mut bytes = limits.string_bytes;
    for (key, field) in fields {
        if !matches!(key.as_str(), "stage" | "subsystem" | "path") {
            return Err(invalid());
        }
        let values = field
            .as_array()
            .filter(|values| !values.is_empty())
            .ok_or_else(invalid)?;
        entries = entries.checked_sub(values.len()).ok_or_else(|| {
            Diagnostic::error("resource-limit", "Condition entry budget exceeded", 0..0)
        })?;
        for item in values {
            let text = item
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(invalid)?;
            bytes = bytes.checked_sub(text.len()).ok_or_else(|| {
                Diagnostic::error(
                    "resource-limit",
                    "Condition string byte budget exceeded",
                    0..0,
                )
            })?;
        }
    }
    if !references::condition(value) {
        return Err(invalid());
    }
    Ok(ValidatedCondition { metadata: value })
}
