# Combined main candidate

Candidate `d8c2ba12b183ef050be6a25bad15204c5e4ff375` preserves the original commit ancestry of PR192, PR196–199 and PR200. Only generated projections conflicted for the SDK runner merge; the CLI rebuilt them. The evidence-only preservation merge also conflicted in the unsigned OW111 progress report; historical evidence pointers were combined and its pending work was replaced by current observed requirements. No signed atom, current verdict, authority record or published commit was rewritten.

Both native hosts passed [38073779068](https://github.com/Quitetall/OpenWarrant/actions/runs/38073779068). The post-merge public SDK CLI suite passed all 11 tests on Rust 1.97.1. Regenerated-document checking passed 2,028 checks with 625 warnings, zero errors and zero unknowns. These are observations of the named candidate, not independent assurance or phase acceptance.

The real OW11 corpus additionally passed the source-shutdown, isolated KF restore and exact byte recovery drill, and the OW consumer reconstructed the KF-restored bytes without access to the producer repository. See the adjacent real-corpus-kf-20261010 evidence. Its temporary KF worktree, local branch, own test databases and own object storage were cleaned up; retained raw inputs and outputs live in the project runtime-evidence directory.

Retargeting the PR to main did not launch the branch-filtered full CI gate. This final authored checkpoint is published after retargeting so the required full gate runs on the final source; native source inventory must be observed again. No prior green job is substituted for that gate. Independent verification, human acceptance and archive-format adoption remain separate.
