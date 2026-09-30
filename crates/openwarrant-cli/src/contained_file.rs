// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs::{self, File, Metadata};
use std::io::{Read, Seek, SeekFrom};

#[cfg(any(test, windows))]
use std::fs::OpenOptions;

use camino::{Utf8Path, Utf8PathBuf};

use crate::repo_error::RepoError;

pub(crate) fn canonical_utf8(path: &Utf8Path, label: &str) -> Result<Utf8PathBuf, RepoError> {
    let canonical = fs::canonicalize(path).map_err(|source| RepoError::Io {
        context: format!("could not resolve {label} {path}"),
        source,
    })?;
    Utf8PathBuf::from_path_buf(canonical).map_err(|_| RepoError::NonUtf8Path)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct OpenedFileIdentity {
    device: u64,
    inode: u64,
}

/// Exact opened file plus directory handles needed to keep path resolution
/// stable on platforms without Linux `openat2(RESOLVE_NO_XDEV)`.
struct OpenedCandidate {
    file: File,
    _ancestor_guards: Vec<File>,
}

pub(crate) struct BoundedRegularRead {
    pub(crate) bytes: Vec<u8>,
    pub(crate) canonical: Utf8PathBuf,
    pub(crate) identity: OpenedFileIdentity,
}

pub(crate) fn read_contained_regular_bounded(
    path: &Utf8Path,
    repository_root: &Utf8Path,
    allowed_root: &Utf8Path,
    allowed_root_label: &str,
    label: &str,
    max_bytes: u64,
) -> Result<BoundedRegularRead, RepoError> {
    let read_limit = max_bytes.checked_add(1).ok_or_else(|| {
        RepoError::Message(format!(
            "cannot enforce a bounded read for {label}: {max_bytes} byte limit leaves no overflow sentinel"
        ))
    })?;
    reject_symlink_components(path, repository_root, label)?;
    let canonical = canonical_utf8(path, label)?;
    if !canonical.starts_with(repository_root) || !canonical.starts_with(allowed_root) {
        return Err(RepoError::Message(format!(
            "{label} path {path} resolves to {canonical}, outside {allowed_root_label} {allowed_root}"
        )));
    }

    // Inspect the canonical target before opening it. This prevents opening a
    // FIFO or device (which could block or stream forever) merely to discover
    // that it was not a document.
    let expected = fs::metadata(&canonical).map_err(|source| RepoError::Io {
        context: format!("could not inspect {label} {canonical}"),
        source,
    })?;
    validate_regular_size(&canonical, label, &expected, max_bytes)?;
    let containment = OpenedPathExpectation {
        requested: path,
        canonical: &canonical,
        repository_root,
        allowed_root,
        allowed_root_label,
        label,
        metadata: &expected,
    };

    // Content is opened exactly once. Metadata is checked again on that same
    // handle so a target replacement between canonicalization and open fails
    // closed on platforms that expose stable file identities.
    let mut candidate =
        open_contained_candidate(&canonical, repository_root, false).map_err(|source| {
        #[cfg(unix)]
        if source.raw_os_error() == Some(libc::EXDEV) {
            return RepoError::Message(format!(
                "{label} {canonical} crosses a mount boundary below repository root {repository_root}"
            ));
        }
        RepoError::Io {
            context: format!("could not open {label} {canonical}"),
            source,
        }
    })?;
    let file = &mut candidate.file;
    let opened = file.metadata().map_err(|source| RepoError::Io {
        context: format!("could not inspect opened {label} {canonical}"),
        source,
    })?;
    validate_regular_size(&canonical, label, &opened, max_bytes)?;
    if !same_snapshot(&expected, &opened) {
        return Err(RepoError::Message(format!(
            "{label} {canonical} changed while it was being opened"
        )));
    }
    let opened_identity = opened_file_identity(file, &opened, label)?;
    verify_opened_path(file, &containment, &opened)?;

    let capacity = usize::try_from(opened.len()).unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    (&mut *file)
        .take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|source| RepoError::Io {
            context: format!("could not read {label} {canonical}"),
            source,
        })?;
    if bytes.len() as u64 > max_bytes {
        return Err(RepoError::Message(format!(
            "{label} {canonical} exceeds {max_bytes} byte limit"
        )));
    }

    #[cfg(all(test, target_os = "linux"))]
    run_test_after_first_read_replacement();

    file.seek(SeekFrom::Start(0))
        .map_err(|source| RepoError::Io {
            context: format!("could not rewind {label} {canonical} for stability verification"),
            source,
        })?;
    let mut verification = (&mut *file).take(read_limit);
    let mut offset = 0_usize;
    let mut chunk = [0_u8; 8192];
    loop {
        let count = verification
            .read(&mut chunk)
            .map_err(|source| RepoError::Io {
                context: format!(
                    "could not re-read {label} {canonical} for stability verification"
                ),
                source,
            })?;
        if count == 0 {
            break;
        }
        let Some(end) = offset.checked_add(count) else {
            return Err(RepoError::Message(format!(
                "{label} {canonical} was not byte-stable while being read"
            )));
        };
        if end > bytes.len() || bytes[offset..end] != chunk[..count] {
            return Err(RepoError::Message(format!(
                "{label} {canonical} was not byte-stable while being read"
            )));
        }
        offset = end;
    }
    if offset != bytes.len() {
        return Err(RepoError::Message(format!(
            "{label} {canonical} was not byte-stable while being read"
        )));
    }

    let after = file.metadata().map_err(|source| RepoError::Io {
        context: format!("could not re-inspect {label} {canonical}"),
        source,
    })?;
    let after_identity = opened_file_identity(file, &after, label)?;
    if opened_identity != after_identity
        || !same_snapshot(&opened, &after)
        || after.len() != bytes.len() as u64
    {
        return Err(RepoError::Message(format!(
            "{label} {canonical} changed while it was being read"
        )));
    }
    verify_opened_path(file, &containment, &after)?;
    Ok(BoundedRegularRead {
        bytes,
        canonical,
        identity: after_identity,
    })
}

/// Keep live and Git-snapshot semantics identical: a Git tree records a symlink
/// blob, while a live filesystem otherwise follows it to target bytes. Reject
/// any symlink component instead of letting the same evidence identity mean two
/// different byte sequences at record and pinned-resolution time.
pub(crate) fn reject_symlink_components(
    path: &Utf8Path,
    repository_root: &Utf8Path,
    label: &str,
) -> Result<(), RepoError> {
    const MAX_COMPONENTS: usize = 256;

    let relative = path.strip_prefix(repository_root).map_err(|_| {
        RepoError::Message(format!(
            "{label} path {path} is not lexically below repository root {repository_root}"
        ))
    })?;
    let parts: Vec<&str> = relative.as_str().split('/').collect();
    if parts.is_empty()
        || parts.len() > MAX_COMPONENTS
        || parts
            .iter()
            .any(|part| part.is_empty() || *part == "." || *part == ".." || part.contains(':'))
    {
        return Err(RepoError::Message(format!(
            "{label} path {path} is not a bounded portable repository path"
        )));
    }
    let mut current = repository_root.to_owned();
    for part in parts {
        current.push(part);
        let metadata = fs::symlink_metadata(&current).map_err(|source| RepoError::Io {
            context: format!("could not inspect {label} path component {current}"),
            source,
        })?;
        #[cfg(windows)]
        let is_link_like = {
            use std::os::windows::fs::MetadataExt;

            use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

            metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        };
        #[cfg(not(windows))]
        let is_link_like = metadata.file_type().is_symlink();
        if is_link_like {
            return Err(RepoError::Message(format!(
                "{label} path {path} contains symlink component {current}"
            )));
        }
    }
    Ok(())
}

/// Open one repository-contained directory as a stable capability root.
///
/// Publication callers perform every later create/link/remove relative to this
/// exact handle. A path replacement after this function returns therefore
/// cannot redirect evidence writes through a different directory entry.
pub(crate) struct OpenedRepositoryDirectory {
    file: File,
    requested: Utf8PathBuf,
    canonical: Utf8PathBuf,
    repository_root: Utf8PathBuf,
    identity: OpenedFileIdentity,
    label: String,
}

impl OpenedRepositoryDirectory {
    pub(crate) fn try_clone_file(&self) -> Result<File, RepoError> {
        self.file.try_clone().map_err(|source| RepoError::Io {
            context: format!("could not clone opened {} {}", self.label, self.canonical),
            source,
        })
    }

    pub(crate) fn verify_current_location(&self) -> Result<(), RepoError> {
        let opened = self.file.metadata().map_err(|source| RepoError::Io {
            context: format!("could not inspect opened {} {}", self.label, self.canonical),
            source,
        })?;
        if !opened.is_dir() {
            return Err(RepoError::Message(format!(
                "opened {} {} is no longer a directory",
                self.label, self.canonical
            )));
        }
        if opened_file_identity(&self.file, &opened, &self.label)? != self.identity {
            return Err(RepoError::Message(format!(
                "opened {} {} changed stable identity",
                self.label, self.canonical
            )));
        }
        verify_opened_directory_location(
            &self.file,
            &self.requested,
            &self.canonical,
            &self.repository_root,
            &self.label,
        )
    }
}

pub(crate) fn open_repository_directory(
    path: &Utf8Path,
    repository_root: &Utf8Path,
    label: &str,
) -> Result<OpenedRepositoryDirectory, RepoError> {
    let repository_root = canonical_utf8(repository_root, "repository root")?;
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        repository_root.join(path)
    };
    reject_symlink_components(&path, &repository_root, label)?;
    let canonical = canonical_utf8(&path, label)?;
    if !canonical.starts_with(&repository_root) {
        return Err(RepoError::Message(format!(
            "{label} path {path} resolves to {canonical}, outside repository root {repository_root}"
        )));
    }
    let expected = fs::metadata(&canonical).map_err(|source| RepoError::Io {
        context: format!("could not inspect {label} {canonical}"),
        source,
    })?;
    if !expected.is_dir() {
        return Err(RepoError::Message(format!(
            "{label} {canonical} is not a directory"
        )));
    }
    let expectation = OpenedPathExpectation {
        requested: &path,
        canonical: &canonical,
        repository_root: &repository_root,
        allowed_root: &repository_root,
        allowed_root_label: "repository root",
        label,
        metadata: &expected,
    };
    let candidate =
        open_contained_candidate(&canonical, &repository_root, true).map_err(|source| {
            #[cfg(unix)]
            if source.raw_os_error() == Some(libc::EXDEV) {
                return RepoError::Message(format!(
                    "{label} {canonical} crosses a mount boundary below repository root {repository_root}"
                ));
            }
            RepoError::Io {
                context: format!("could not open {label} {canonical}"),
                source,
            }
        })?;
    let opened = candidate.file.metadata().map_err(|source| RepoError::Io {
        context: format!("could not inspect opened {label} {canonical}"),
        source,
    })?;
    if !opened.is_dir() {
        return Err(RepoError::Message(format!(
            "opened {label} {canonical} is not a directory"
        )));
    }
    verify_opened_path(&candidate.file, &expectation, &opened)?;
    let identity = opened_file_identity(&candidate.file, &opened, label)?;
    Ok(OpenedRepositoryDirectory {
        file: candidate.file,
        requested: path,
        canonical,
        repository_root,
        identity,
        label: label.to_owned(),
    })
}

#[cfg(target_os = "linux")]
fn verify_opened_directory_location(
    file: &File,
    requested: &Utf8Path,
    canonical: &Utf8Path,
    repository_root: &Utf8Path,
    label: &str,
) -> Result<(), RepoError> {
    let descriptor = linux_proc_fd_path(file);
    let opened = match canonical_utf8(&descriptor, &format!("opened {label} descriptor")) {
        Ok(opened) => opened,
        Err(error) if procfs_descriptor_lookup_unavailable(&error) => {
            use std::os::unix::fs::MetadataExt;

            let current = canonical_utf8(requested, label)?;
            let current_metadata = fs::metadata(&current).map_err(|source| RepoError::Io {
                context: format!("could not inspect current {label} {current}"),
                source,
            })?;
            let opened_metadata = file.metadata().map_err(|source| RepoError::Io {
                context: format!("could not inspect opened {label} {canonical}"),
                source,
            })?;
            if current != canonical
                || current_metadata.dev() != opened_metadata.dev()
                || current_metadata.ino() != opened_metadata.ino()
            {
                return Err(RepoError::Message(format!(
                    "opened {label} no longer occupies contained path {canonical}"
                )));
            }
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    if opened != canonical || !opened.starts_with(repository_root) {
        return Err(RepoError::Message(format!(
            "opened {label} resolves to {opened}, not contained expected path {canonical} under repository root {repository_root}"
        )));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn verify_opened_directory_location(
    file: &File,
    requested: &Utf8Path,
    canonical: &Utf8Path,
    repository_root: &Utf8Path,
    label: &str,
) -> Result<(), RepoError> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let raw_path = rustix::fs::getpath(file).map_err(|source| RepoError::Io {
        context: format!("could not resolve exact opened {label} handle"),
        source: source.into(),
    })?;
    let opened = Utf8PathBuf::from_path_buf(std::path::PathBuf::from(OsStr::from_bytes(
        raw_path.to_bytes(),
    )))
    .map_err(|_| RepoError::NonUtf8Path)?;
    let current = canonical_utf8(requested, label)?;
    if opened != canonical || current != canonical || !opened.starts_with(repository_root) {
        return Err(RepoError::Message(format!(
            "opened {label} resolves to {opened}, not contained expected path {canonical} under repository root {repository_root}"
        )));
    }
    Ok(())
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn verify_opened_directory_location(
    _file: &File,
    _requested: &Utf8Path,
    _canonical: &Utf8Path,
    _repository_root: &Utf8Path,
    label: &str,
) -> Result<(), RepoError> {
    Err(RepoError::Message(format!(
        "cannot prove current opened-directory containment for {label} on this Unix target"
    )))
}

#[cfg(windows)]
fn verify_opened_directory_location(
    file: &File,
    requested: &Utf8Path,
    canonical: &Utf8Path,
    repository_root: &Utf8Path,
    label: &str,
) -> Result<(), RepoError> {
    let opened = Utf8PathBuf::from_path_buf(winx::file::get_file_path(file).map_err(|source| {
        RepoError::Io {
            context: format!("could not resolve exact opened {label} handle"),
            source,
        }
    })?)
    .map_err(|_| RepoError::NonUtf8Path)?;
    let current = canonical_utf8(requested, label)?;
    if opened != canonical || current != canonical || !opened.starts_with(repository_root) {
        return Err(RepoError::Message(format!(
            "opened {label} resolves to {opened}, not contained expected path {canonical} under repository root {repository_root}"
        )));
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn verify_opened_directory_location(
    _file: &File,
    _requested: &Utf8Path,
    _canonical: &Utf8Path,
    _repository_root: &Utf8Path,
    label: &str,
) -> Result<(), RepoError> {
    Err(RepoError::Message(format!(
        "cannot prove current opened-directory containment for {label} on this target"
    )))
}

/// Read one regular file contained by a repository root with the same
/// opened-handle, race, and byte-limit checks used for Warrant sources.
///
/// This is intentionally narrower than a general filesystem reader: callers
/// must supply the repository root whose authority bounds the artifact.
#[allow(
    dead_code,
    reason = "ADR 0186 wires this shared reader into legacy disposition loading in the next owned patch"
)]
pub(crate) fn read_repository_regular_bounded(
    path: &Utf8Path,
    repository_root: &Utf8Path,
    label: &str,
    max_bytes: u64,
) -> Result<Vec<u8>, RepoError> {
    let repository_root = canonical_utf8(repository_root, "repository root")?;
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        repository_root.join(path)
    };
    read_contained_regular_bounded(
        &path,
        &repository_root,
        &repository_root,
        "repository root",
        label,
        max_bytes,
    )
    .map(|read| read.bytes)
}

pub(crate) struct OpenedPathExpectation<'a> {
    pub(crate) requested: &'a Utf8Path,
    pub(crate) canonical: &'a Utf8Path,
    pub(crate) repository_root: &'a Utf8Path,
    pub(crate) allowed_root: &'a Utf8Path,
    pub(crate) allowed_root_label: &'a str,
    pub(crate) label: &'a str,
    pub(crate) metadata: &'a Metadata,
}

#[cfg(target_os = "linux")]
fn open_contained_candidate(
    path: &Utf8Path,
    repository_root: &Utf8Path,
    final_directory: bool,
) -> std::io::Result<OpenedCandidate> {
    use rustix::fs::{Mode, OFlags, ResolveFlags, openat2};

    let relative = path.strip_prefix(repository_root).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{path} is not below repository root {repository_root}"),
        )
    })?;
    if relative.as_str().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "capability root itself is not a readable artifact",
        ));
    }

    let root = File::open(repository_root)?;
    let mut flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK;
    if final_directory {
        flags |= OFlags::DIRECTORY;
    }
    let descriptor = openat2(
        &root,
        relative.as_str(),
        flags,
        Mode::empty(),
        ResolveFlags::BENEATH
            | ResolveFlags::NO_MAGICLINKS
            | ResolveFlags::NO_SYMLINKS
            | ResolveFlags::NO_XDEV,
    )?;
    Ok(OpenedCandidate {
        file: File::from(descriptor),
        _ancestor_guards: Vec::new(),
    })
}

#[cfg(target_os = "macos")]
fn open_contained_candidate(
    path: &Utf8Path,
    repository_root: &Utf8Path,
    final_directory: bool,
) -> std::io::Result<OpenedCandidate> {
    use std::os::unix::fs::MetadataExt;
    use std::path::Component;

    use rustix::fs::{Mode, OFlags, openat};

    const MAX_CONTAINMENT_DEPTH: usize = 256;

    let relative = path.strip_prefix(repository_root).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{path} is not below repository root {repository_root}"),
        )
    })?;
    let components: Vec<_> = relative.as_std_path().components().collect();
    if components.is_empty() || components.len() > MAX_CONTAINMENT_DEPTH {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "artifact path is empty or exceeds containment depth limit",
        ));
    }

    let mut current = File::open(repository_root)?;
    let root_device = current.metadata()?.dev();
    let root_mount = darwin_mount_name(&current)?;
    let mut guards = Vec::with_capacity(components.len());

    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "artifact path contains a non-normal component",
            ));
        };
        let final_component = index + 1 == components.len();
        let mut flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK;
        if !final_component || final_directory {
            flags |= OFlags::DIRECTORY;
        }
        let next = File::from(openat(&current, name, flags, Mode::empty())?);
        let metadata = next.metadata()?;
        if (!final_component && !metadata.is_dir())
            || metadata.dev() != root_device
            || darwin_mount_name(&next)? != root_mount
        {
            return Err(std::io::Error::from_raw_os_error(libc::EXDEV));
        }
        guards.push(current);
        current = next;
    }

    Ok(OpenedCandidate {
        file: current,
        _ancestor_guards: guards,
    })
}

#[cfg(target_os = "macos")]
fn darwin_mount_name(file: &File) -> std::io::Result<Vec<u8>> {
    let statistics = rustix::fs::fstatfs(file)?;
    Ok(statistics
        .f_mntonname
        .iter()
        .copied()
        .take_while(|byte| *byte != 0)
        .map(|byte| byte as u8)
        .collect())
}

#[cfg(windows)]
fn open_contained_candidate(
    path: &Utf8Path,
    repository_root: &Utf8Path,
    final_directory: bool,
) -> std::io::Result<OpenedCandidate> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use std::path::Component;

    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    const MAX_CONTAINMENT_DEPTH: usize = 256;

    let relative = path.strip_prefix(repository_root).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{path} is not below repository root {repository_root}"),
        )
    })?;
    let components: Vec<_> = relative.as_std_path().components().collect();
    if components.is_empty() || components.len() > MAX_CONTAINMENT_DEPTH {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "artifact path is empty or exceeds containment depth limit",
        ));
    }

    let open_component = |component_path: &Utf8Path, directory: bool, writable: bool| {
        let mut options = OpenOptions::new();
        options.read(true).write(writable).share_mode(if directory {
            FILE_SHARE_READ | FILE_SHARE_WRITE
        } else {
            FILE_SHARE_READ
        });
        let mut flags = FILE_FLAG_OPEN_REPARSE_POINT;
        if directory {
            flags |= FILE_FLAG_BACKUP_SEMANTICS;
        }
        options.custom_flags(flags).open(component_path)
    };

    let root = open_component(repository_root, true, false)?;
    let root_metadata = root.metadata()?;
    if !root_metadata.is_dir()
        || root_metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        return Err(std::io::Error::other(
            "canonical repository root is not a non-reparse directory",
        ));
    }
    let root_volume = winapi_util::file::information(&root)?.volume_serial_number();
    let mut guards = vec![root];
    let mut accumulated = repository_root.to_owned();

    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "artifact path contains a non-normal component",
            ));
        };
        let name = name.to_str().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "canonical artifact path is not UTF-8",
            )
        })?;
        accumulated.push(name);
        let final_component = index + 1 == components.len();
        let next = open_component(
            &accumulated,
            !final_component || final_directory,
            final_component && final_directory,
        )?;
        let metadata = next.metadata()?;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(std::io::Error::other(format!(
                "artifact path crosses reparse component {accumulated}"
            )));
        }
        if (!final_component && !metadata.is_dir())
            || winapi_util::file::information(&next)?.volume_serial_number() != root_volume
        {
            return Err(std::io::Error::other(format!(
                "artifact path crosses a mount boundary at {accumulated}"
            )));
        }
        if final_component {
            return Ok(OpenedCandidate {
                file: next,
                _ancestor_guards: guards,
            });
        }
        guards.push(next);
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        "capability root itself is not a readable artifact",
    ))
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn open_contained_candidate(
    _path: &Utf8Path,
    _repository_root: &Utf8Path,
    _final_directory: bool,
) -> std::io::Result<OpenedCandidate> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "mount-safe repository reads are unsupported on this target",
    ))
}

#[cfg(all(test, unix))]
pub(crate) fn open_capability_candidate(
    path: &Utf8Path,
    repository_root: &Utf8Path,
) -> std::io::Result<File> {
    use cap_std::fs::{Dir, OpenOptions as CapOpenOptions, OpenOptionsExt};

    let relative = path.strip_prefix(repository_root).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{path} is not below repository root {repository_root}"),
        )
    })?;
    if relative.as_str().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "capability root itself is not a readable artifact",
        ));
    }

    let dir = Dir::open_ambient_dir(repository_root, cap_std::ambient_authority())?;
    let mut options = CapOpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    #[cfg(windows)]
    {
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
        };

        options
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }

    let file = dir.open_with(relative.as_std_path(), &options)?.into_std();
    let root = dir.into_std_file();

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        if root.metadata()?.dev() != file.metadata()?.dev() {
            return Err(std::io::Error::from_raw_os_error(libc::EXDEV));
        }
    }
    #[cfg(windows)]
    {
        let root_information = winapi_util::file::information(&root)?;
        let file_information = winapi_util::file::information(&file)?;
        if root_information.volume_serial_number() != file_information.volume_serial_number() {
            return Err(std::io::Error::other(
                "opened artifact crosses a mount boundary below capability root",
            ));
        }
    }

    Ok(file)
}

#[cfg(all(test, unix))]
pub(crate) fn open_regular_candidate(path: &Utf8Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    // A pathname can be replaced after metadata inspection. O_NONBLOCK keeps
    // a swapped FIFO or device from hanging the verifier; O_NOFOLLOW rejects a
    // swapped symlink. Handle metadata is still authoritative after open.
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
}

#[cfg(all(test, windows))]
pub(crate) fn open_regular_candidate(path: &Utf8Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ};

    // A canonical target should not itself be a reparse point. Opening a
    // replacement reparse point rather than following it lets the regular-file
    // metadata check fail closed if the final component is swapped.
    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

#[cfg(all(test, not(any(unix, windows))))]
pub(crate) fn open_regular_candidate(_path: &Utf8Path) -> std::io::Result<File> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "safe opened-handle verification is unsupported on this target",
    ))
}

pub(crate) fn validate_regular_size(
    path: &Utf8Path,
    label: &str,
    metadata: &Metadata,
    max_bytes: u64,
) -> Result<(), RepoError> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;

        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(RepoError::Message(format!(
                "{label} {path} is a reparse point, not a regular file"
            )));
        }
    }

    if !metadata.is_file() {
        return Err(RepoError::Message(format!(
            "{label} {path} is not a regular file"
        )));
    }
    if metadata.len() > max_bytes {
        return Err(RepoError::Message(format!(
            "{label} {path} is {} bytes, exceeding {max_bytes} byte limit",
            metadata.len()
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn opened_file_identity(
    _file: &File,
    metadata: &Metadata,
    _label: &str,
) -> Result<OpenedFileIdentity, RepoError> {
    use std::os::unix::fs::MetadataExt;

    Ok(OpenedFileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(windows)]
fn opened_file_identity(
    file: &File,
    _metadata: &Metadata,
    label: &str,
) -> Result<OpenedFileIdentity, RepoError> {
    let information = winapi_util::file::information(file).map_err(|source| RepoError::Io {
        context: format!("could not obtain stable identity for opened {label}"),
        source,
    })?;

    Ok(OpenedFileIdentity {
        device: information.volume_serial_number(),
        inode: information.file_index(),
    })
}

#[cfg(not(any(unix, windows)))]
fn opened_file_identity(
    _file: &File,
    _metadata: &Metadata,
    label: &str,
) -> Result<OpenedFileIdentity, RepoError> {
    Err(RepoError::Message(format!(
        "cannot bind opened {label} to a stable file identity on this target"
    )))
}

#[cfg(target_os = "linux")]
fn linux_proc_fd_path(file: &File) -> Utf8PathBuf {
    use std::os::fd::AsRawFd;

    let descriptor_name = file.as_raw_fd().to_string();
    #[cfg(test)]
    if let Some(root) = TEST_LINUX_PROC_FD_ROOT.with(|slot| slot.borrow().clone()) {
        return root.join(descriptor_name);
    }

    Utf8PathBuf::from("/proc/self/fd").join(descriptor_name)
}

#[cfg(all(test, target_os = "linux"))]
std::thread_local! {
    static TEST_LINUX_PROC_FD_ROOT: std::cell::RefCell<Option<Utf8PathBuf>> = const {
        std::cell::RefCell::new(None)
    };
}

#[cfg(all(test, target_os = "linux"))]
pub(crate) fn with_test_linux_proc_fd_root<T>(root: &Utf8Path, operation: impl FnOnce() -> T) -> T {
    struct ResetProcFdRoot(Option<Utf8PathBuf>);

    impl Drop for ResetProcFdRoot {
        fn drop(&mut self) {
            TEST_LINUX_PROC_FD_ROOT.with(|slot| {
                *slot.borrow_mut() = self.0.take();
            });
        }
    }

    let previous = TEST_LINUX_PROC_FD_ROOT.with(|slot| slot.replace(Some(root.to_owned())));
    let _reset = ResetProcFdRoot(previous);
    operation()
}

#[cfg(all(test, target_os = "linux"))]
std::thread_local! {
    static TEST_AFTER_FIRST_READ_REPLACEMENT: std::cell::RefCell<Option<(Utf8PathBuf, Vec<u8>)>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(all(test, target_os = "linux"))]
fn run_test_after_first_read_replacement() {
    if let Some((path, bytes)) =
        TEST_AFTER_FIRST_READ_REPLACEMENT.with(|slot| slot.borrow_mut().take())
    {
        fs::write(path, bytes).expect("replace bytes after first bounded read");
    }
}

#[cfg(all(test, target_os = "linux"))]
pub(crate) fn with_test_after_first_read_replacement<T>(
    path: &Utf8Path,
    bytes: &[u8],
    operation: impl FnOnce() -> T,
) -> T {
    struct ResetReplacement(Option<(Utf8PathBuf, Vec<u8>)>);

    impl Drop for ResetReplacement {
        fn drop(&mut self) {
            TEST_AFTER_FIRST_READ_REPLACEMENT.with(|slot| {
                *slot.borrow_mut() = self.0.take();
            });
        }
    }

    let replacement = Some((path.to_owned(), bytes.to_vec()));
    let previous = TEST_AFTER_FIRST_READ_REPLACEMENT.with(|slot| slot.replace(replacement));
    let _reset = ResetReplacement(previous);
    operation()
}

#[cfg(target_os = "linux")]
pub(crate) fn verify_opened_path(
    file: &File,
    expectation: &OpenedPathExpectation<'_>,
    opened_metadata: &Metadata,
) -> Result<(), RepoError> {
    // `/proc/self/fd` resolves the object held by this exact descriptor, not
    // the pathname that was checked before `open`. This closes the rename or
    // symlink-swap window between canonicalization and opening on Linux.
    let descriptor = linux_proc_fd_path(file);
    let opened = match canonical_utf8(
        &descriptor,
        &format!("opened {} descriptor", expectation.label),
    ) {
        Ok(opened) => opened,
        Err(error) if procfs_descriptor_lookup_unavailable(&error) => {
            return verify_requested_path_snapshot(expectation, opened_metadata);
        }
        Err(error) => return Err(error),
    };
    if !same_snapshot(expectation.metadata, opened_metadata)
        || opened != expectation.canonical
        || !opened.starts_with(expectation.repository_root)
        || !opened.starts_with(expectation.allowed_root)
    {
        return Err(RepoError::Message(format!(
            "opened {} resolves to {opened}, not contained expected path {} under {} {}",
            expectation.label,
            expectation.canonical,
            expectation.allowed_root_label,
            expectation.allowed_root
        )));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn procfs_descriptor_lookup_unavailable(error: &RepoError) -> bool {
    matches!(
        error,
        RepoError::Io { source, .. }
            if matches!(
                source.kind(),
                std::io::ErrorKind::NotFound
                    | std::io::ErrorKind::NotADirectory
                    | std::io::ErrorKind::PermissionDenied
                    | std::io::ErrorKind::Unsupported
            )
    )
}

#[cfg(unix)]
fn verify_requested_path_snapshot(
    expectation: &OpenedPathExpectation<'_>,
    opened_metadata: &Metadata,
) -> Result<(), RepoError> {
    // Stable device/inode and the complete snapshot bind this handle to the
    // contained object inspected before open. Re-resolving the pathname also
    // catches a mapping that remains changed.
    let current = canonical_utf8(expectation.requested, expectation.label)?;
    let current_metadata = fs::metadata(expectation.requested).map_err(|source| RepoError::Io {
        context: format!(
            "could not inspect re-resolved {} {}",
            expectation.label, expectation.requested
        ),
        source,
    })?;
    if !same_snapshot(expectation.metadata, opened_metadata)
        || !same_snapshot(opened_metadata, &current_metadata)
        || current != expectation.canonical
        || !current.starts_with(expectation.repository_root)
        || !current.starts_with(expectation.allowed_root)
    {
        return Err(RepoError::Message(format!(
            "{} path or stable file identity changed from {} to {current} while open",
            expectation.label, expectation.canonical
        )));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn verify_opened_path(
    file: &File,
    expectation: &OpenedPathExpectation<'_>,
    opened_metadata: &Metadata,
) -> Result<(), RepoError> {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let raw_path = rustix::fs::getpath(file).map_err(|source| RepoError::Io {
        context: format!(
            "could not resolve exact opened {} handle",
            expectation.label
        ),
        source: source.into(),
    })?;
    let opened = Utf8PathBuf::from_path_buf(std::path::PathBuf::from(OsStr::from_bytes(
        raw_path.to_bytes(),
    )))
    .map_err(|_| RepoError::NonUtf8Path)?;
    if opened != expectation.canonical
        || !opened.starts_with(expectation.repository_root)
        || !opened.starts_with(expectation.allowed_root)
    {
        return Err(RepoError::Message(format!(
            "opened {} handle resolves to {opened}, not contained expected path {} under {} {}",
            expectation.label,
            expectation.canonical,
            expectation.allowed_root_label,
            expectation.allowed_root
        )));
    }
    verify_requested_path_snapshot(expectation, opened_metadata)
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
pub(crate) fn verify_opened_path(
    _file: &File,
    expectation: &OpenedPathExpectation<'_>,
    _opened_metadata: &Metadata,
) -> Result<(), RepoError> {
    Err(RepoError::Message(format!(
        "cannot prove mount-safe opened-handle containment for {} on this Unix target",
        expectation.label
    )))
}

#[cfg(windows)]
pub(crate) fn verify_opened_path(
    file: &File,
    expectation: &OpenedPathExpectation<'_>,
    opened_metadata: &Metadata,
) -> Result<(), RepoError> {
    let opened = Utf8PathBuf::from_path_buf(winx::file::get_file_path(file).map_err(|source| {
        RepoError::Io {
            context: format!(
                "could not resolve exact opened {} handle",
                expectation.label
            ),
            source,
        }
    })?)
    .map_err(|_| RepoError::NonUtf8Path)?;
    let current = canonical_utf8(expectation.requested, expectation.label)?;
    if !same_snapshot(expectation.metadata, opened_metadata)
        || opened != expectation.canonical
        || current != expectation.canonical
        || !opened.starts_with(expectation.repository_root)
        || !opened.starts_with(expectation.allowed_root)
        || !current.starts_with(expectation.repository_root)
        || !current.starts_with(expectation.allowed_root)
    {
        return Err(RepoError::Message(format!(
            "opened {} handle resolves to {opened}, not contained at expected path {} under {} {}",
            expectation.label,
            expectation.canonical,
            expectation.allowed_root_label,
            expectation.allowed_root
        )));
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
pub(crate) fn verify_opened_path(
    _file: &File,
    expectation: &OpenedPathExpectation<'_>,
    _opened_metadata: &Metadata,
) -> Result<(), RepoError> {
    Err(RepoError::Message(format!(
        "cannot prove opened-handle containment for {} on this target",
        expectation.label
    )))
}

#[cfg(unix)]
fn same_snapshot(left: &Metadata, right: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    left.dev() == right.dev()
        && left.ino() == right.ino()
        && left.len() == right.len()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

#[cfg(windows)]
fn same_snapshot(left: &Metadata, right: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    left.file_attributes() == right.file_attributes()
        && left.creation_time() == right.creation_time()
        && left.last_write_time() == right.last_write_time()
        && left.file_size() == right.file_size()
}

#[cfg(not(any(unix, windows)))]
fn same_snapshot(left: &Metadata, right: &Metadata) -> bool {
    left.len() == right.len()
        && left.modified().ok() == right.modified().ok()
        && left.created().ok() == right.created().ok()
}
