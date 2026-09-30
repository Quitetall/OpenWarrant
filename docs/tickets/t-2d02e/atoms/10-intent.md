# Make contractor frozen-module copy work across filesystems

Full gate at 91f912aa stopped in 56-contractor with exit 9. Actual clone reproduces fatal Invalid cross-device link from /mnt/4tb into /tmp, exit 128. The same source cloned with --no-hardlinks succeeds. Keep all frozen-module checks and refusal assertions intact.
