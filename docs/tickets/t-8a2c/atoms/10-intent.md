# Found by the verifier-fix agents: a whole-ticket claim hints a refused done; context_artifacts reads outside the repository; a one-line file (SECTIONS.json) travels in a bundle with no text

## Notes

- **2026-09-27 07:54 UTC, claude:** claim_cmd names the next open item (test claiming_a_whole_ticket_names_a_command_that_works runs the hinted command). context_select refuses an absolute path, a .. step, or a link resolving outside the root: dispatch.artifact-outside (unit test incl. symlink to /etc). bundle byte_windows: head, first hit per term, tail within the allowance, char-boundary safe (unit test, 60 KB one-line JSON and multibyte).
