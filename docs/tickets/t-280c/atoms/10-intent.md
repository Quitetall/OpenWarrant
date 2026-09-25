# war next is slow (~7.5 s)

Ticket commands answer in 5-13 ms; war next takes ~7.5 s on this corpus (it dry-runs every pending act and rebuilds status). Profile it the way compile was (run under gdb, SIGINT the child) and cut it.
