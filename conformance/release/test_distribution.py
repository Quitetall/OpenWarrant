# SPDX-License-Identifier: Apache-2.0
"""The one-line installers name exactly the assets the release workflow publishes.

package.py's archive_name() is the one spelling. The cargo-binstall metadata in
crates/openwarrant-cli/Cargo.toml, install.sh, and packaging/homebrew/render.py
each restate it; these tests hold them to it, and show each check refusing a
planted drift. No network: render.py's fetch is injected.

    python3 conformance/release/test_distribution.py
"""
import importlib.util
from pathlib import Path
import re
import sys
import tempfile
import tomllib
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
from package import archive_name  # noqa: E402

spec = importlib.util.spec_from_file_location('render', ROOT / 'packaging/homebrew/render.py')
render = importlib.util.module_from_spec(spec)
spec.loader.exec_module(render)

# Rust target -> (platform.system(), platform.machine()) on the runner that
# builds it. check_preview.py requires exactly these two hosts.
HOSTS = {'x86_64-unknown-linux-gnu': ('Linux', 'x86_64'), 'aarch64-apple-darwin': ('Darwin', 'arm64')}
REPO = 'https://github.com/Quitetall/OpenWarrant'


def manifest():
    return tomllib.loads((ROOT / 'crates/openwarrant-cli/Cargo.toml').read_text())


def release_targets():
    text = (ROOT / '.github/workflows/release.yml').read_text()
    return set(re.findall(r'^\s+target: (\S+)\s*$', text, re.M))


def binstall_mismatches(cargo, repo=REPO):
    """Every way the binstall metadata fails to name the published assets."""
    problems = []
    package = cargo['package']
    version = package['version']
    meta = package.get('metadata', {}).get('binstall')
    if meta is None:
        return ['no [package.metadata.binstall]']
    bins = [b['name'] for b in cargo.get('bin', [])]
    if bins != ['war']:
        problems.append(f'binaries are {bins}, not [war]')
    if meta.get('pkg-fmt') != 'tgz':
        problems.append(f'pkg-fmt is {meta.get("pkg-fmt")!r}, not tgz (package.py writes .tar.gz)')
    # package.py writes the binary at bin/war in a flat archive.
    bin_dir = meta.get('bin-dir', '').replace('{ bin }', 'war').replace('{ binary-ext }', '')
    if bin_dir != 'bin/war':
        problems.append(f'bin-dir resolves to {bin_dir!r}, not bin/war')
    if 'pkg-url' in meta:
        problems.append('a top-level pkg-url would claim an asset for targets release.yml never builds')
    overrides = meta.get('overrides', {})
    if set(overrides) != set(HOSTS):
        problems.append(f'overrides cover {sorted(overrides)}, release.yml builds {sorted(HOSTS)}')
    for target, (system, machine) in HOSTS.items():
        url = overrides.get(target, {}).get('pkg-url', '')
        url = url.replace('{ repo }', repo).replace('{ version }', version)
        tag = 'v' + version
        want = f'{repo}/releases/download/{tag}/{archive_name(tag, system, machine)}'
        if url != want:
            problems.append(f'{target}: pkg-url is {url!r}, package.py publishes {want!r}')
    return problems


class Spelling(unittest.TestCase):
    def test_release_builds_exactly_the_mapped_targets(self):
        self.assertEqual(release_targets(), set(HOSTS))

    def test_install_sh_and_render_name_the_same_platforms(self):
        platforms = {f'{s.lower()}-{m}' for s, m in HOSTS.values()}
        script = (ROOT / 'install.sh').read_text()
        self.assertEqual(set(re.findall(r'PLATFORM="([^"]+)"', script)), platforms)
        self.assertEqual(set(render.PLATFORMS), platforms)
        for system, machine in HOSTS.values():
            p = f'{system.lower()}-{machine}'
            self.assertEqual(render.asset_name('v1.2.3-alpha.4', p), archive_name('v1.2.3-alpha.4', system, machine))


class Binstall(unittest.TestCase):
    def test_metadata_names_the_published_assets(self):
        self.assertEqual(binstall_mismatches(manifest()), [])

    def test_drift_is_refused(self):
        def planted(edit):
            cargo = manifest()
            edit(cargo['package']['metadata']['binstall'])
            return binstall_mismatches(cargo)

        def misspell(m):
            o = m['overrides']['aarch64-apple-darwin']
            o['pkg-url'] = o['pkg-url'].replace('darwin-arm64', 'darwin-aarch64')

        def unprefixed(m):
            o = m['overrides']['x86_64-unknown-linux-gnu']
            o['pkg-url'] = o['pkg-url'].replace('openwarrant-v{ version }', 'openwarrant-{ version }')

        plants = {
            'misspelled platform': misspell,
            'tag without v': unprefixed,
            'missing target': lambda m: m['overrides'].pop('aarch64-apple-darwin'),
            'extra target': lambda m: m['overrides'].update({'aarch64-unknown-linux-gnu': {'pkg-url': 'x'}}),
            'top-level pkg-url': lambda m: m.update({'pkg-url': '{ repo }/x'}),
            'nested bin-dir': lambda m: m.update({'bin-dir': 'openwarrant/bin/{ bin }{ binary-ext }'}),
            'zip format': lambda m: m.update({'pkg-fmt': 'zip'}),
        }
        for name, edit in plants.items():
            with self.subTest(plant=name):
                self.assertNotEqual(planted(edit), [])

    def test_absent_metadata_is_refused(self):
        cargo = manifest()
        del cargo['package']['metadata']
        self.assertEqual(binstall_mismatches(cargo), ['no [package.metadata.binstall]'])


TAG = 'v1.0.0-alpha.3'
SUMS = {'darwin-arm64': '3d47c977cb345438d780951c52eac21cc18a7023d4654cf2f05c4b53271abeec',
        'linux-x86_64': '32e181bf9ee1360dc331900cdca449ab434dc07ab2160aea862a8bb8c3b086c9'}


def published(tag=TAG, sums=SUMS, body=None):
    """A fetch over a fake release: URL -> bytes, or the error a 404 gives."""
    files = {}
    for platform, digest in sums.items():
        name = render.asset_name(tag, platform)
        text = f'{digest}  {name}\n' if body is None else body(digest, name)
        files[render.asset_url(render.REPO, tag, name + '.sha256')] = text.encode()

    def fetch(url):
        if url not in files:
            raise render.RenderError(f'could not read {url}: HTTP Error 404: Not Found')
        return files[url]
    return fetch


class Homebrew(unittest.TestCase):
    def test_renders_the_published_checksums(self):
        text = render.render(TAG, fetch=published())
        self.assertIn('version "1.0.0-alpha.3"', text)
        for platform, digest in SUMS.items():
            url = render.asset_url(render.REPO, TAG, render.asset_name(TAG, platform))
            self.assertIn(f'url "{url}"\n      sha256 "{digest}"', text)
        self.assertNotRegex(text, r'@[A-Z0-9_]+@')

    def test_refusals(self):
        only_linux = {'linux-x86_64': SUMS['linux-x86_64']}
        cases = {
            'missing .sha256': dict(fetch=published(sums=only_linux)),
            'malformed checksum': dict(fetch=published(body=lambda d, n: f'{d[:-1]}  {n}\n')),
            'uppercase checksum': dict(fetch=published(body=lambda d, n: f'{d.upper()}  {n}\n')),
            'checksum for another file': dict(fetch=published(body=lambda d, n: f'{d}  other.tar.gz\n')),
            'two lines': dict(fetch=published(body=lambda d, n: f'{d}  {n}\n{d}  {n}\n')),
            'not a version tag': dict(tag='latest', fetch=published()),
            'bad repo': dict(repo='../x', fetch=published()),
            'template lost a placeholder': dict(
                fetch=published(), template=render.TEMPLATE.read_text().replace('@SHA256_LINUX_X86_64@', '0' * 64)),
            'template grew a placeholder': dict(
                fetch=published(), template=render.TEMPLATE.read_text() + '# @UNKNOWN@\n'),
            'template URL drifted': dict(
                fetch=published(), template=render.TEMPLATE.read_text().replace('-darwin-arm64.tar.gz"', '-darwin-aarch64.tar.gz"')),
        }
        for name, kwargs in cases.items():
            with self.subTest(case=name):
                tag = kwargs.pop('tag', TAG)
                with self.assertRaises(render.RenderError):
                    render.render(tag, **kwargs)

    def test_output_refuses_an_existing_path(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / 'openwarrant.rb'
            out.write_text('kept')
            original = render.render
            render.render = lambda tag, repo: 'new'
            try:
                self.assertEqual(render.main(['--tag', TAG, '--output', str(out)]), 1)
            finally:
                render.render = original
            self.assertEqual(out.read_text(), 'kept')


if __name__ == '__main__':
    unittest.main()
