"""Reports computed from a loaded journal.

Each report is split into a ``compute_*`` function returning plain rows and
a ``render_*`` function turning rows into text, so the rows can be tested
and reused without parsing output.
"""
