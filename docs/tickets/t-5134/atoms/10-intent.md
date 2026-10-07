# Plant checks that pass without checking anything

Found by t-515e: (a) 59-webui-lan OBL-003's source grep passes whenever the second grep (ssh-add in *.rs) finds nothing — the group's status is that grep's, so a signing call found by the first grep would be missed; (b) 67-pins-next and 69-current's HISTORY.md check both pass if war itself exits non-zero. A check must fail when the command it observes did not run or failed.
