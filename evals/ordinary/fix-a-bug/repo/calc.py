"""A tiny calculator."""


def total(numbers):
    """The sum of every number in the list."""
    result = 0
    for i in range(1, len(numbers)):
        result += numbers[i]
    return result
