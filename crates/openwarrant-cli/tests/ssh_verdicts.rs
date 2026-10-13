// SPDX-License-Identifier: Apache-2.0
//! The `ssh-keygen -Y verify` verdict memo and its persistent cache of good
//! verdicts (`openwarrant_cli::ssh_verdicts`): one ask per question across
//! threads; a pass recorded only for exactly the bytes `ssh-keygen` accepted;
//! a tampered signature refused with the cache warm; a changed
//! allowed_signers file, principal or namespace another key; a clock-bound
//! allowed_signers file never cached; the bound (count and age) pruning
//! deterministically, and a pruned pass only asked again.
//!
//! The key is a throwaway generated here, in a scratch directory, and the
//! signature is made with `ssh-keygen -Y sign -f <private key>` with no agent
//! in the environment.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_cli::ssh_verdicts::{self, Exact, Question, Slots};

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-ssh-verdicts-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn concurrent_askers_of_one_question_ask_once() {
    let slots: Arc<Slots<u32>> = Arc::new(Slots::default());
    let asked = Arc::new(AtomicUsize::new(0));
    let start = Arc::new(Barrier::new(8));
    let answers: Vec<u32> = (0..8)
        .map(|_| {
            let (slots, asked, start) = (slots.clone(), asked.clone(), start.clone());
            std::thread::spawn(move || {
                start.wait();
                slots.ask(
                    "one question".to_owned(),
                    || {
                        asked.fetch_add(1, Ordering::SeqCst);
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        7
                    },
                    |v| Some(*v),
                    |v| v,
                )
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect();
    assert_eq!(asked.load(Ordering::SeqCst), 1, "eight askers, one ask");
    assert!(answers.iter().all(|a| *a == 7));
}

#[test]
fn an_answer_not_kept_is_asked_again() {
    let slots: Slots<u32> = Slots::default();
    let asked = AtomicUsize::new(0);
    for _ in 0..3 {
        let got: Result<u32, String> = slots.ask(
            "q".to_owned(),
            || {
                asked.fetch_add(1, Ordering::SeqCst);
                Err("ssh-keygen could not be run".to_owned())
            },
            |r| r.as_ref().ok().copied(),
            Ok,
        );
        assert!(got.is_err());
    }
    assert_eq!(
        asked.load(Ordering::SeqCst),
        3,
        "an unavailable answer is never kept"
    );
}

/// A throwaway key, an allowed_signers line for it, a message and its
/// signature under `oh.war/response`.
#[cfg(unix)]
struct Signed {
    root: PathBuf,
    allowed: Vec<u8>,
    message: Vec<u8>,
    signature: Vec<u8>,
}

#[cfg(unix)]
fn ssh_keygen(dir: &Path, args: &[&str]) -> bool {
    Command::new("ssh-keygen")
        .args(args)
        .current_dir(dir)
        .env_remove("SSH_AUTH_SOCK")
        .env_remove("SSH_AGENT_PID")
        .stdin(std::process::Stdio::null())
        .output()
        .is_ok_and(|o| o.status.success())
}

#[cfg(unix)]
fn signed(tag: &str) -> Option<Signed> {
    let root = scratch(tag);
    if !ssh_keygen(
        &root,
        &[
            "-q",
            "-t",
            "ed25519",
            "-N",
            "",
            "-C",
            "throwaway",
            "-f",
            "key",
        ],
    ) {
        eprintln!("ssh-keygen is not available; skipped");
        std::fs::remove_dir_all(&root).unwrap();
        return None;
    }
    let public = std::fs::read_to_string(root.join("key.pub")).unwrap();
    let public: Vec<&str> = public.split_whitespace().take(2).collect();
    let allowed = format!(
        "tester namespaces=\"oh.war/response\" {}\n",
        public.join(" ")
    )
    .into_bytes();
    let message =
        b"schema = \"oh.war/authorization-response/v1\"\nwarrant = \"T-WAR-0001\"\n".to_vec();
    std::fs::write(root.join("message"), &message).unwrap();
    assert!(ssh_keygen(
        &root,
        &[
            "-Y",
            "sign",
            "-f",
            "key",
            "-n",
            "oh.war/response",
            "message"
        ]
    ));
    let signature = std::fs::read(root.join("message.sig")).unwrap();
    Some(Signed {
        root,
        allowed,
        message,
        signature,
    })
}

#[cfg(unix)]
fn question<'a>(s: &'a Signed, signature: &'a [u8], allowed: &'a [u8]) -> Question<'a> {
    Question {
        allowed_signers: allowed,
        principal: "tester",
        namespace: "oh.war/response",
        message: &s.message,
        signature,
    }
}

#[cfg(unix)]
fn entries(dir: &Utf8Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .map(|d| {
            d.map(|e| e.unwrap().file_name().into_string().unwrap())
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

#[cfg(unix)]
#[test]
fn a_pass_is_recorded_only_for_the_bytes_ssh_keygen_accepted() {
    let Some(s) = signed("pass") else { return };
    let tool = ssh_verdicts::ssh_keygen().expect("ssh-keygen on PATH");
    let dir = Utf8PathBuf::from_path_buf(s.root.join("cache")).unwrap();

    // Cold: not known; asked over exactly these bytes; recorded.
    let good = question(&s, &s.signature, &s.allowed);
    let key = good
        .key(tool)
        .expect("an untimed allowed_signers file is cacheable");
    assert!(!ssh_verdicts::known_good(&dir, &key));
    assert_eq!(
        ssh_verdicts::verify_exact(&dir, &key, &tool.0, &good),
        Exact::Good
    );
    assert!(ssh_verdicts::known_good(&dir, &key));
    assert_eq!(
        entries(&dir),
        vec![key.clone()],
        "one entry, no scratch left"
    );

    // Warm, and one byte of the signature flipped: another key, unknown, and
    // `ssh-keygen` refuses it; nothing is recorded.
    let mut tampered = s.signature.clone();
    let at = tampered.len() / 2;
    tampered[at] ^= 0x01;
    let bad = question(&s, &tampered, &s.allowed);
    let bad_key = bad.key(tool).unwrap();
    assert_ne!(bad_key, key);
    assert!(!ssh_verdicts::known_good(&dir, &bad_key));
    assert_eq!(
        ssh_verdicts::verify_exact(&dir, &bad_key, &tool.0, &bad),
        Exact::NotGood
    );
    assert!(!ssh_verdicts::known_good(&dir, &bad_key));
    assert_eq!(
        entries(&dir),
        vec![key.clone()],
        "a refusal is never recorded"
    );

    // One byte of the message changed: another key, refused.
    let mut message = s.message.clone();
    message[0] ^= 0x01;
    let moved = Question {
        message: &message,
        ..question(&s, &s.signature, &s.allowed)
    };
    let moved_key = moved.key(tool).unwrap();
    assert_ne!(moved_key, key);
    assert_eq!(
        ssh_verdicts::verify_exact(&dir, &moved_key, &tool.0, &moved),
        Exact::NotGood
    );

    // A changed allowed_signers file (the key now granted another
    // principal): another key, and refused for `tester`.
    let regranted = String::from_utf8(s.allowed.clone())
        .unwrap()
        .replacen("tester", "somebody-else", 1)
        .into_bytes();
    let changed = question(&s, &s.signature, &regranted);
    let changed_key = changed.key(tool).unwrap();
    assert_ne!(changed_key, key);
    assert!(!ssh_verdicts::known_good(&dir, &changed_key));
    assert_eq!(
        ssh_verdicts::verify_exact(&dir, &changed_key, &tool.0, &changed),
        Exact::NotGood
    );

    // Another principal or namespace over the same bytes: other keys.
    let other_principal = Question {
        principal: "somebody-else",
        ..question(&s, &s.signature, &s.allowed)
    };
    let other_namespace = Question {
        namespace: "oh.war/dsse",
        ..question(&s, &s.signature, &s.allowed)
    };
    assert_ne!(other_principal.key(tool).unwrap(), key);
    assert_ne!(other_namespace.key(tool).unwrap(), key);

    // An entry whose content is not exactly the format and its key is not
    // a pass.
    std::fs::write(dir.join(&bad_key), "good\n").unwrap();
    assert!(!ssh_verdicts::known_good(&dir, &bad_key));

    // Under a clock-bound line `ssh-keygen` answers for now only: no key.
    for option in [
        "valid-before=\"29990101\"",
        "valid-after=\"20000101\"",
        "cert-authority",
    ] {
        let timed = String::from_utf8(s.allowed.clone())
            .unwrap()
            .replacen("namespaces=", &format!("{option},namespaces="), 1)
            .into_bytes();
        assert!(
            question(&s, &s.signature, &timed).key(tool).is_none(),
            "{option}"
        );
    }
    std::fs::remove_dir_all(&s.root).unwrap();
}

#[cfg(unix)]
#[test]
fn a_different_ssh_keygen_is_another_key() {
    let Some(s) = signed("tool") else { return };
    let tool = ssh_verdicts::ssh_keygen().expect("ssh-keygen on PATH");
    let q = question(&s, &s.signature, &s.allowed);
    let other = (tool.0.clone(), "0".repeat(64));
    assert_ne!(q.key(tool), q.key(&other));
    std::fs::remove_dir_all(&s.root).unwrap();
}

// ---------------------------------------------------------------------------
// The bound: at most MAX_ENTRIES, none unused past MAX_UNUSED, and a pruned
// pass costs only a respawn.
// ---------------------------------------------------------------------------

const DAY: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

fn key_name(i: usize) -> String {
    format!("{i:064x}")
}

#[test]
fn victims_keep_the_bound_and_drop_the_unused() {
    use std::time::{Duration, UNIX_EPOCH};
    let now = UNIX_EPOCH + Duration::from_secs(1_000 * 24 * 60 * 60);
    let at = |days_ago: u64| now - DAY * u32::try_from(days_ago).unwrap();
    let entries = vec![
        (key_name(1), at(91)),         // past the age
        (key_name(2), at(89)),         // within it
        (key_name(3), at(5)),          // within it
        (key_name(4), at(5)),          // same mtime as 3: ties by name
        (key_name(5), now + DAY * 30), // a future mtime is just used
    ];
    let max = ssh_verdicts::MAX_UNUSED;

    // The age rule alone.
    assert_eq!(
        ssh_verdicts::victims(&entries, now, 100, max),
        vec![key_name(1)]
    );
    // The bound: the age victim, then the oldest of the rest.
    assert_eq!(
        ssh_verdicts::victims(&entries, now, 3, max),
        vec![key_name(1), key_name(2)]
    );
    // A tie in mtime goes by name: 3 before 4.
    assert_eq!(
        ssh_verdicts::victims(&entries, now, 2, max),
        vec![key_name(1), key_name(2), key_name(3)]
    );
    // Zero: everything.
    assert_eq!(ssh_verdicts::victims(&entries, now, 0, max).len(), 5);
    // Deterministic: the order of the listing does not matter.
    let mut reversed = entries.clone();
    reversed.reverse();
    for bound in 0..=6 {
        assert_eq!(
            ssh_verdicts::victims(&entries, now, bound, max),
            ssh_verdicts::victims(&reversed, now, bound, max),
            "bound {bound}"
        );
    }
}

#[cfg(unix)]
#[test]
fn prune_on_disk_removes_only_what_the_rules_name() {
    use std::fs::File;
    use std::time::SystemTime;
    let root = scratch("prune");
    let dir = Utf8PathBuf::from_path_buf(root.join("cache")).unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    let now = SystemTime::now();
    let age = |p: &Utf8Path, days: u32| {
        File::options()
            .write(true)
            .open(p)
            .unwrap()
            .set_modified(now - DAY * days)
            .unwrap();
    };
    // Ten entries, entry i last used i days ago; entry 9 is 95 days old.
    for i in 0..10 {
        let p = dir.join(key_name(i));
        std::fs::write(&p, "x").unwrap();
        age(
            &p,
            if i == 9 {
                95
            } else {
                u32::try_from(i).unwrap()
            },
        );
    }
    // Not an entry: left alone, however old.
    std::fs::write(dir.join("README"), "x").unwrap();
    age(&dir.join("README"), 400);
    // Scratch: old goes, fresh stays.
    std::fs::write(dir.join(".old.1.tmp"), "x").unwrap();
    age(&dir.join(".old.1.tmp"), 2);
    std::fs::write(dir.join(".new.1.tmp"), "x").unwrap();
    std::fs::create_dir(dir.join(".ask-1-0")).unwrap();
    std::fs::write(dir.join(".ask-1-0/message.sig"), "x").unwrap();
    std::fs::File::open(dir.join(".ask-1-0"))
        .unwrap()
        .set_modified(now - DAY * 2)
        .unwrap();
    std::fs::create_dir(dir.join(".ask-1-1")).unwrap();

    let removed = ssh_verdicts::prune(&dir, now, 4, ssh_verdicts::MAX_UNUSED);
    assert_eq!(removed, 6, "one aged, then five to reach the bound of four");
    let mut want: Vec<String> = (0..4).map(key_name).collect();
    want.extend([
        ".ask-1-1".to_owned(),
        ".new.1.tmp".to_owned(),
        "README".to_owned(),
    ]);
    want.sort();
    assert_eq!(entries(&dir), want);

    // Again, nothing over the bound: nothing removed.
    assert_eq!(
        ssh_verdicts::prune(&dir, now, 4, ssh_verdicts::MAX_UNUSED),
        0
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[cfg(unix)]
#[test]
fn pruning_everything_only_costs_a_respawn() {
    use std::time::SystemTime;
    let Some(s) = signed("pruned") else { return };
    let tool = ssh_verdicts::ssh_keygen().expect("ssh-keygen on PATH");
    let dir = Utf8PathBuf::from_path_buf(s.root.join("cache")).unwrap();

    let mut tampered = s.signature.clone();
    let at = tampered.len() / 2;
    tampered[at] ^= 0x01;
    let mut message = s.message.clone();
    message[0] ^= 0x01;
    let regranted = String::from_utf8(s.allowed.clone())
        .unwrap()
        .replacen("tester", "somebody-else", 1)
        .into_bytes();
    let questions = [
        question(&s, &s.signature, &s.allowed),
        question(&s, &tampered, &s.allowed),
        Question {
            message: &message,
            ..question(&s, &s.signature, &s.allowed)
        },
        question(&s, &s.signature, &regranted),
    ];
    let keys: Vec<String> = questions.iter().map(|q| q.key(tool).unwrap()).collect();
    // What a check answers: a pass from the cache, or ssh-keygen asked.
    let check = || -> Vec<Exact> {
        questions
            .iter()
            .zip(&keys)
            .map(|(q, k)| {
                if ssh_verdicts::known_good(&dir, k) {
                    Exact::Good
                } else {
                    ssh_verdicts::verify_exact(&dir, k, &tool.0, q)
                }
            })
            .collect()
    };
    let cold = check();
    assert_eq!(
        cold,
        vec![Exact::Good, Exact::NotGood, Exact::NotGood, Exact::NotGood]
    );
    assert_eq!(entries(&dir), vec![keys[0].clone()]);
    assert_eq!(check(), cold, "warm");

    // Prune everything: no entry left, no pass known.
    let removed = ssh_verdicts::prune(&dir, SystemTime::now(), 0, ssh_verdicts::MAX_UNUSED);
    assert_eq!(removed, 1);
    assert!(entries(&dir).is_empty());
    assert!(!ssh_verdicts::known_good(&dir, &keys[0]));

    // The same check again: the same verdicts, the pass asked and recorded
    // again, every refusal still a refusal and still unrecorded.
    assert_eq!(check(), cold, "after pruning everything");
    assert_eq!(entries(&dir), vec![keys[0].clone()]);
    std::fs::remove_dir_all(&s.root).unwrap();
}

#[cfg(unix)]
#[test]
fn a_hit_marks_its_entry_used() {
    use std::time::SystemTime;
    let Some(s) = signed("refresh") else { return };
    let tool = ssh_verdicts::ssh_keygen().expect("ssh-keygen on PATH");
    let dir = Utf8PathBuf::from_path_buf(s.root.join("cache")).unwrap();
    let q = question(&s, &s.signature, &s.allowed);
    let key = q.key(tool).unwrap();
    assert_eq!(
        ssh_verdicts::verify_exact(&dir, &key, &tool.0, &q),
        Exact::Good
    );
    let path = dir.join(&key);
    let old = SystemTime::now() - DAY * 2;
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(old)
        .unwrap();
    assert!(ssh_verdicts::known_good(&dir, &key));
    let mtime = std::fs::metadata(&path).unwrap().modified().unwrap();
    assert!(
        mtime > old + DAY,
        "a hit on a two-day-old entry refreshes it"
    );
    std::fs::remove_dir_all(&s.root).unwrap();
}
