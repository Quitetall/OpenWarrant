#!/usr/bin/env python3
"""Executable documentation examples; no SDK or authority qualification claim."""
import unittest
from check_draft import BASE, footer_document


class FooterExamples(unittest.TestCase):
    def setUp(self):
        self.raw = (BASE / 'examples-footer/minimal.md').read_bytes()

    def test_readable_first_exact_spans_lf_and_crlf(self):
        for raw in [self.raw, self.raw.replace(b'\n', b'\r\n')]:
            meta, spans = footer_document(raw)
            self.assertEqual(meta['title'], 'Make signup easier')
            self.assertEqual(list(spans), ['outcome', 'scope', 'context'])
            start, end = spans['context']
            self.assertTrue(raw[start:end].startswith(b'## Known context'))
            self.assertTrue(raw[end:].startswith(b'<!-- ow:metadata -->'))
            self.assertNotIn(b'```toml', raw[start:end])

    def test_all_examples_and_wrapped_candidate(self):
        for path in list((BASE / 'examples-footer').glob('*.md')) + [BASE / 'WAR_Software_Architecture_Specification.md', BASE / 'format-contract.md']:
            if path.name != 'README.md':
                with self.subTest(path=path.name):
                    footer_document(path.read_bytes())

    def test_fenced_markers_are_unit_text(self):
        example = b'~~~~markdown\n<!-- ow:metadata -->\n<!-- ow:unit false binding -->\n~~~~\n'
        raw = self.raw.replace(b'## Scope\n', b'## Scope\n'+example)
        _, spans = footer_document(raw)
        self.assertEqual(list(spans), ['outcome', 'scope', 'context'])
        self.assertIn(example, raw[slice(*spans['scope'])])

    def test_compact_dependency_pairs_preserve_order(self):
        repeated = b'[[dependencies]]\nunit = "outcome"\ntarget = "#scope"\n[[dependencies]]\nunit = "outcome"\ntarget = "#context"\n'
        compact = b'dependencies = [{unit="outcome",target="#scope"}, {unit="outcome",target="#context"}]\n'
        def with_meta(extra):
            return self.raw.replace(b'state = "draft"\n', b'state = "draft"\n'+extra)
        self.assertEqual(footer_document(with_meta(repeated))[0], footer_document(with_meta(compact))[0])

    def test_unicode_spans_and_inert_quote_comment(self):
        raw = self.raw.replace(b'Explore the signup form.', 'Inspect café signup 🛠.'.encode())
        raw = raw.replace(b'state = "draft"', b'state = "draft" # """ is a comment')
        _, spans = footer_document(raw)
        scope = raw[slice(*spans['scope'])]
        self.assertIn('café signup 🛠'.encode(), scope)
        self.assertTrue(scope.endswith(b'\n\n'))

    def test_each_malformed_form_refuses(self):
        cases = {
            'missing': self.raw[:self.raw.index(b'<!-- ow:metadata -->')],
            'truncated': self.raw.replace(b'<!-- /ow:metadata -->', b''),
            'duplicate': self.raw + self.raw[self.raw.index(b'<!-- ow:metadata -->'):],
            'trailing': self.raw + b'new instructions',
            'mixed': b'+++\n' + self.raw,
            'old-version': self.raw.replace(b'document/1.0.0-rc.3', b'document/1.0.0-rc.2'),
            'unknown-version': self.raw.replace(b'document/1.0.0-rc.3', b'document/9'),
            'duplicate-key': self.raw.replace(b'state = "draft"', b'state = "draft"\nstate = "proposed"'),
            'multiline': self.raw.replace(b'state = "draft"', b'state = """draft"""'),
            'title': self.raw.replace(b'# Make signup easier', b'# Wrong title'),
            'unit': self.raw.replace(b'ow:unit scope', b'ow:unit outcome'),
            'bad-marker': self.raw.replace(b'<!-- ow:metadata -->', b'<!-- ow:metadata-- >'),
            'fence': self.raw.replace(b'## Scope\n', b'## Scope\n```\n'),
            'nul': self.raw+b'\0',
            'utf8': self.raw+b'\xff',
            'bom': b'\xef\xbb\xbf'+self.raw,
        }
        for name, raw in cases.items():
            with self.subTest(case=name), self.assertRaises((ValueError, KeyError)):
                footer_document(raw)


if __name__ == '__main__':
    unittest.main()
