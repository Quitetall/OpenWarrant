import unittest
from calc import add
class AddTests(unittest.TestCase):
    def test_positive(self): self.assertEqual(add(3, 4), 7)
    def test_negative(self): self.assertEqual(add(-2, 1), -1)
