"""Run: python3 test_calc.py"""
import sys

from calc import total

failures = 0
for numbers, want in [([1, 2, 3], 6), ([], 0), ([5], 5)]:
    got = total(numbers)
    if got != want:
        print(f"total({numbers}) = {got}, want {want}")
        failures += 1
print("ok" if not failures else f"{failures} failure(s)")
sys.exit(1 if failures else 0)
