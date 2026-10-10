// SPDX-License-Identifier: Apache-2.0
//! Bounded, sealed data input for a native verifier. The expected SHA-256 must
//! come from independently approved host configuration, never the receipt.
//! This neither authenticates that configuration nor executes retained data.
use openwarrant_core::document::runtime::ProviderFailure;
use std::path::{Path, PathBuf};

const MAX_BYTES: usize = 128 * 1024 * 1024;

pub struct ProtectedInput {
    #[cfg(target_os = "linux")]
    _image: std::fs::File,
    argument: PathBuf,
    digest: String,
    bytes: usize,
    source_mode: u32,
}
impl ProtectedInput {
    /// Capture an operator-owned regular file into a non-executable sealed
    /// image. No growth beyond the caller's explicit finite byte budget.
    pub fn acquire(
        path: &Path,
        expected_digest: &str,
        max_bytes: usize,
    ) -> Result<Self, ProviderFailure> {
        if !path.is_absolute()
            || path
                .components()
                .any(|p| matches!(p, std::path::Component::ParentDir))
            || expected_digest.len() != 64
            || !expected_digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || max_bytes == 0
            || max_bytes > MAX_BYTES
        {
            return Err(rejected(
                "absolute input path, canonical SHA256 and finite byte budget required",
            ));
        }
        #[cfg(target_os = "linux")]
        {
            acquire_linux(path, expected_digest, max_bytes)
        }
        #[cfg(not(target_os = "linux"))]
        {
            Err(ProviderFailure::Unavailable(
                "sealed native input unsupported on this platform".into(),
            ))
        }
    }
    /// Keep this object alive through native verification. The child must be
    /// able to read the parent's procfs descriptor in the same process domain.
    /// CLOEXEC keeps the descriptor out of unrelated child processes; this
    /// explicit path provides access to the verifier's selected input only.
    pub fn argument(&self) -> &Path {
        &self.argument
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
    /// Original opened regular-file mode, including its type bits. The sealed
    /// image remains data-only; this preserves native job metadata, not execute permission.
    pub fn source_mode(&self) -> u32 {
        self.source_mode
    }
}
fn rejected(message: &str) -> ProviderFailure {
    ProviderFailure::Rejected(message.into())
}

#[cfg(target_os = "linux")]
fn acquire_linux(
    path: &Path,
    expected: &str,
    limit: usize,
) -> Result<ProtectedInput, ProviderFailure> {
    use rustix::fs::{self as rfs, Access, AtFlags, MemfdFlags, Mode, OFlags, SealFlags};
    use std::{
        fs,
        io::{Read, Write},
        os::{fd::AsRawFd, unix::fs::MetadataExt},
    };
    let system_error = |e: rustix::io::Errno| ProviderFailure::Unavailable(e.to_string());
    let io_error = |e: std::io::Error| ProviderFailure::Unavailable(e.to_string());
    crate::authority_cmd::store::unprivileged_reader().map_err(|_| {
        ProviderFailure::Unavailable(
            "execution privileges cannot establish protected native inputs".into(),
        )
    })?;
    let uid = rustix::process::geteuid().as_raw();
    for component in path.ancestors() {
        let metadata = fs::symlink_metadata(component).map_err(io_error)?;
        let writable = match rfs::accessat(rfs::CWD, component, Access::WRITE_OK, AtFlags::EACCESS)
        {
            Ok(()) => true,
            Err(rustix::io::Errno::ACCESS | rustix::io::Errno::ROFS) => false,
            Err(e) => return Err(system_error(e)),
        };
        if metadata.file_type().is_symlink()
            || metadata.uid() == uid
            || metadata.mode() & 0o022 != 0
            || writable
            || (component != path && !metadata.is_dir())
        {
            return Err(rejected(
                "native input path is writable, self-owned, linked or not a directory",
            ));
        }
    }
    let fd = rfs::open(
        path,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map_err(system_error)?;
    let mut source = fs::File::from(fd);
    let metadata = source.metadata().map_err(io_error)?;
    if !metadata.is_file()
        || metadata.uid() == uid
        || metadata.mode() & 0o022 != 0
        || metadata.len() > limit as u64
    {
        return Err(rejected(
            "protected regular native input within byte budget required",
        ));
    }
    let mut bytes = Vec::new();
    (&mut source)
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > limit {
        return Err(rejected("native input byte budget exceeded"));
    }
    let digest = openwarrant_compiler::sha256_hex(&bytes);
    if digest != expected {
        return Err(rejected(
            "native input digest differs from approved configuration",
        ));
    }
    let fd = rfs::memfd_create(
        "openwarrant-native-input",
        MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING | MemfdFlags::NOEXEC_SEAL,
    )
    .map_err(system_error)?;
    let mut image = fs::File::from(fd);
    image.write_all(&bytes).map_err(io_error)?;
    rfs::fchmod(&image, Mode::from_bits_truncate(0o400)).map_err(system_error)?;
    rfs::fcntl_add_seals(
        &image,
        SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::EXEC | SealFlags::SEAL,
    )
    .map_err(system_error)?;
    let argument = PathBuf::from(format!(
        "/proc/{}/fd/{}",
        std::process::id(),
        image.as_raw_fd()
    ));
    Ok(ProtectedInput {
        _image: image,
        argument,
        digest,
        bytes: bytes.len(),
        source_mode: metadata.mode(),
    })
}
