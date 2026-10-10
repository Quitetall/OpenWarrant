# Checks

What must say "pass" before an item ticks as observed: `war done <item> --check`
runs the Warrant's tests and KPIs (and the item's own) and ticks only if they pass.

## Tests

- ladder: `cargo test -q -p openwarrant-cli --test tick_ladder_cli`
