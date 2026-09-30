# Bounded file-input refusal

The reference app opened state/report inputs before checking file type. A FIFO
with no writer blocked that open indefinitely. A directory failed in the Python
file wrapper before the intended regular-file refusal and explicit cleanup.

The reader now opens with `O_NOFOLLOW | O_NONBLOCK`, checks the opened descriptor
with `fstat`, and closes it in `finally`. Regular-file byte limits remain intact.
This changes no authority, execution or completion rights.

The focused regression reproduced one failure and one error before the fix.
After the fix all three tests pass: FIFO refusal in a subprocess with a three-second
timeout, directory refusal, and exact/oversized regular-file reads. The subprocess
timeout bounds the test and kills/reaps a hung child; it is not the production fix.

The original offline browser observation remains pending. These checks do not
establish browser rendering, host isolation, independent verification or human
acceptance.

Full reference web suite also passed: Ran 195 tests in 65.248s  OK
Generated record check: 942 pass, 88 warn, 0 unknown, 0 error.
