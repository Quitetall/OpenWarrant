// SPDX-License-Identifier: Apache-2.0
//! Activated enrollment plus sealed executable. The trusted host supplies actual
//! repository/collector identities and protects native inputs. Checks around a
//! process are not atomic launch fencing, caller authentication or assurance.
use super::{collector_loading::LoadedEnrollment, protected_executable::ProtectedExecutable};
use openwarrant_core::{
    document::runtime::{ProviderFailure, ProviderInterface, ProviderKind},
    runtime_collector::{Fault, Provider, Use},
};
use std::{ffi::OsString, path::Path, time::Duration};

pub struct ActivatedVerifier {
    enrollment: LoadedEnrollment,
    executable: ProtectedExecutable,
    repository: String,
    collector: String,
    warrant: String,
    provider: Provider,
    digest: String,
    host: Option<super::collector_loading::HostObservation>,
}
pub(super) fn failure(fault: Fault) -> ProviderFailure {
    match fault {
        Fault::Rejected(message) => ProviderFailure::Rejected(message.into()),
        Fault::Unavailable(message) => ProviderFailure::Unavailable(message.into()),
    }
}
impl ActivatedVerifier {
    /// Independent host identities, exact recorded Warrant UUID and provider
    /// are checked against an operator-activated record; no wildcard scope.
    pub fn acquire(
        enrollment: LoadedEnrollment,
        repository: &str,
        collector: &str,
        warrant: &str,
        provider: &ProviderInterface,
        path: &Path,
    ) -> Result<Self, ProviderFailure> {
        let provider = Provider {
            kind: match provider.kind {
                ProviderKind::Katana => "katana",
                ProviderKind::Blut => "blut",
            }
            .into(),
            identity: provider.identity.clone(),
            version: provider.version.clone(),
        };
        let digest = enrollment.enrollment().verifier_digest.clone();
        enrollment
            .allows_active(Use {
                repository,
                collector,
                warrant,
                provider: &provider,
                verifier_digest: &digest,
            })
            .map_err(failure)?;
        let expected = digest.strip_prefix("sha256:").ok_or_else(|| {
            ProviderFailure::Rejected("SHA-256 enrollment digest required".into())
        })?;
        let executable = ProtectedExecutable::acquire(path, expected)?;
        let result = Self {
            enrollment,
            executable,
            repository: repository.into(),
            collector: collector.into(),
            warrant: warrant.into(),
            provider,
            digest,
            host: None,
        };
        result.check()?;
        Ok(result)
    }
    pub(super) fn with_host(
        mut self,
        host: super::collector_loading::HostObservation,
    ) -> Result<Self, ProviderFailure> {
        self.host = Some(host);
        self.check()?;
        Ok(self)
    }
    pub fn check(&self) -> Result<(), ProviderFailure> {
        if let Some(host) = &self.host {
            host.check().map_err(failure)?;
        }
        self.enrollment
            .allows_active(Use {
                repository: &self.repository,
                collector: &self.collector,
                warrant: &self.warrant,
                provider: &self.provider,
                verifier_digest: &self.digest,
            })
            .map_err(failure)
    }
    /// Always check the current activation/authority after execution, including
    /// failed executions. A revoked result is never returned as usable evidence.
    pub fn run(
        &self,
        args: &[OsString],
        timeout: Duration,
        limit: usize,
    ) -> Result<(bool, Vec<u8>), ProviderFailure> {
        self.check()?;
        let result = self.executable.run(args, timeout, limit);
        self.check()?;
        result
    }
}
