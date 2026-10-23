# Preserve HTTP request deadlines across interrupted and late reads

Protected CI40011093425 failed the original silent-request408 test with400. The original syscall kind was not logged. Separate deterministic external byte-reader controls reproduce400 on Interrupted and on EOF returned after the deadline; late complete reads could also pass. Retry Interrupted and check the same absolute deadline after reads. Do not enlarge deadlines or alter the original test expectation. Preserve the original CI failure and report its exact underlying syscall cause as UNKNOWN.
