import unittest
from calc import add
class TestAdd(unittest.TestCase):
    def test_positive(self): self.assertEqual(add(3,4),7)
    def test_mixed(self): self.assertEqual(add(-2,1),-1)
