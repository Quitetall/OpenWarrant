// SPDX-License-Identifier: Apache-2.0
//! Bind an approved ELF image to its observed bytes and sealed execution.
//! This does not authenticate the caller, authorize work, protect dynamic
//! libraries, or fence authority revocation. Those remain host responsibilities.
use openwarrant_core::document::runtime::ProviderFailure;
use std::{ffi::OsString, path::Path, time::Duration};

pub struct ProtectedExecutable {
    #[cfg(target_os = "linux")]
    image: std::fs::File,
    digest: String,
}

impl ProtectedExecutable {
    /// The expected digest must come from independently approved configuration.
    pub fn acquire(path: &Path, expected_digest: &str) -> Result<Self, ProviderFailure> {
        if !path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
            || expected_digest.len() != 64
            || !expected_digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(ProviderFailure::Rejected(
                "absolute executable path and canonical SHA256 required".into(),
            ));
        }
        #[cfg(target_os = "linux")]
        {
            acquire_linux(path, expected_digest)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(ProviderFailure::Unavailable(
                "sealed executable observation unsupported on this platform".into(),
            ))
        }
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Keep the sealed descriptor alive through spawn and bounded collection.
    /// Environment is cleared so callers cannot inject loader settings.
    pub fn run(
        &self,
        args: &[OsString],
        timeout: Duration,
        limit: usize,
    ) -> Result<(bool, Vec<u8>), ProviderFailure> {
        if timeout.is_zero()
            || timeout > Duration::from_secs(300)
            || limit == 0
            || limit > 16 * 1024 * 1024
        {
            return Err(ProviderFailure::Rejected(
                "executable deadline or response budget outside bounds".into(),
            ));
        }
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd;
            let mut command =
                std::process::Command::new(format!("/proc/self/fd/{}", self.image.as_raw_fd()));
            command.args(args).env_clear().env("LC_ALL", "C");
            super::process::run_with_stdin(command, timeout, limit, std::process::Stdio::null())
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = args;
            Err(ProviderFailure::Unavailable(
                "sealed execution unsupported on this platform".into(),
            ))
        }
    }
}

#[cfg(target_os = "linux")]
fn acquire_linux(path: &Path, expected: &str) -> Result<ProtectedExecutable, ProviderFailure> {
    use rustix::fs::{self as rfs, Access, AtFlags, MemfdFlags, OFlags, SealFlags};
    use std::{
        fs,
        io::{Read, Write},
        os::unix::fs::MetadataExt,
    };
    let unavailable = |e: std::io::Error| ProviderFailure::Unavailable(e.to_string());
    let system_error = |e: rustix::io::Errno| ProviderFailure::Unavailable(e.to_string());
    let uid = rustix::process::geteuid().as_raw();
    // Inspect every component. No canonicalization that silently follows links.
    for component in path.ancestors() {
        let metadata = fs::symlink_metadata(component).map_err(unavailable)?;
        let writable = match rfs::accessat(rfs::CWD, component, Access::WRITE_OK, AtFlags::EACCESS)
        {
            Ok(()) => true,
            Err(rustix::io::Errno::ACCESS | rustix::io::Errno::ROFS) => false,
            Err(error) => return Err(system_error(error)),
        };
        if metadata.file_type().is_symlink()
            || metadata.uid() == uid
            || metadata.mode() & 0o022 != 0
            || writable
            || (component != path && !metadata.is_dir())
        {
            return Err(ProviderFailure::Rejected(
                "executable path is writable, self-owned, linked or not traversable as a directory"
                    .into(),
            ));
        }
    }
    let fd = rfs::open(
        path,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        rfs::Mode::empty(),
    )
    .map_err(system_error)?;
    let mut source = fs::File::from(fd);
    let metadata = source.metadata().map_err(unavailable)?;
    const LIMIT: u64 = 128 * 1024 * 1024;
    if !metadata.is_file()
        || metadata.uid() == uid
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o111 == 0
        || metadata.len() > LIMIT
    {
        return Err(ProviderFailure::Rejected(
            "executable descriptor is not a protected bounded executable file".into(),
        ));
    }
    let mut bytes = Vec::new();
    (&mut source)
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(unavailable)?;
    if bytes.len() as u64 > LIMIT || !bytes.starts_with(b"\x7fELF") {
        return Err(ProviderFailure::Rejected(
            "bounded ELF executable required; interpreters are not protected by this interface"
                .into(),
        ));
    }
    let digest = openwarrant_compiler::sha256_hex(&bytes);
    if digest != expected {
        return Err(ProviderFailure::Rejected(
            "observed executable digest differs from approved configuration".into(),
        ));
    }
    let fd = rfs::memfd_create(
        "openwarrant-verifier",
        MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING | MemfdFlags::EXEC,
    )
    .map_err(system_error)?;
    let mut image = fs::File::from(fd);
    image.write_all(&bytes).map_err(unavailable)?;
    rfs::fchmod(&image, rfs::Mode::from_bits_truncate(0o500)).map_err(system_error)?;
    rfs::fcntl_add_seals(
        &image,
        SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::EXEC | SealFlags::SEAL,
    )
    .map_err(system_error)?;
    Ok(ProtectedExecutable { image, digest })
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[test]
    fn approved_image_executes_from_a_sealed_descriptor() {
        let path = Path::new("/usr/bin/true");
        let digest = openwarrant_compiler::sha256_hex(&std::fs::read(path).unwrap());
        let image = ProtectedExecutable::acquire(path, &digest).unwrap();
        assert_eq!(image.digest(), digest);
        let (success, output) = image.run(&[], Duration::from_secs(3), 1024).unwrap();
        assert!(success);
        assert!(output.is_empty());
        use std::io::Write;
        assert!(
            image
                .image
                .try_clone()
                .unwrap()
                .write_all(b"changed")
                .is_err()
        );
    }
    #[test]
    fn mismatch_and_unbounded_execution_are_refused() {
        let path = Path::new("/usr/bin/true");
        assert!(matches!(
            ProtectedExecutable::acquire(path, &"0".repeat(64)),
            Err(ProviderFailure::Rejected(_))
        ));
        assert!(matches!(
            ProtectedExecutable::acquire(Path::new("true"), &"0".repeat(64)),
            Err(ProviderFailure::Rejected(_))
        ));
        let digest = openwarrant_compiler::sha256_hex(&std::fs::read(path).unwrap());
        let image = ProtectedExecutable::acquire(path, &digest).unwrap();
        assert!(matches!(
            image.run(&[], Duration::ZERO, 1024),
            Err(ProviderFailure::Rejected(_))
        ));
        assert!(matches!(
            image.run(&[], Duration::from_secs(1), 0),
            Err(ProviderFailure::Rejected(_))
        ));
    }
}
