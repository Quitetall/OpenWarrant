# 69-current: 'OW-WAR-0073's signature stands' passes if the dry run fails

Found by t-5134's performer: the negative half of this check passes when `war sign --all --dry-run` itself fails, so it can pass without checking. Require the dry run's exit status and report, as t-5134 did for the HISTORY.md check.
