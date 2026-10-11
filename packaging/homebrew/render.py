#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Render the Homebrew formula for one published release.

Reads each archive's `.sha256` from the release itself and fills
packaging/homebrew/openwarrant.rb. It refuses, writing nothing, when the tag is
not a version tag, when a platform's `.sha256` is missing or unreadable, when a
checksum file is malformed or names another file, and when any placeholder is
left unfilled. It never pushes to a tap; what to do with the output is the
owner's step (docs/INSTALL.md).

    python3 packaging/homebrew/render.py --tag v1.0.0-alpha.3 [--output FILE]
"""
import argparse
from pathlib import Path
import re
import sys
import urllib.error
import urllib.request

TEMPLATE = Path(__file__).resolve().parent / 'openwarrant.rb'
REPO = 'Quitetall/OpenWarrant'
# The two targets release.yml builds, as package.py names them
# (platform.system().lower(), platform.machine()), and the template's
# placeholder for each checksum.
PLATFORMS = {'darwin-arm64': '@SHA256_DARWIN_ARM64@', 'linux-x86_64': '@SHA256_LINUX_X86_64@'}
TAG = re.compile(r'v(\d+\.\d+\.\d+(?:-[0-9A-Za-z]+(?:\.[0-9A-Za-z]+)*)?)')
REPO_NAME = re.compile(r'[A-Za-z0-9][A-Za-z0-9-]*/[A-Za-z0-9._-]+')
LIMIT = 4096


class RenderError(ValueError):
    pass


def asset_name(tag, platform):
    # package.py's archive_name(label=tag, system, machine).
    return f'openwarrant-{tag}-{platform}.tar.gz'


def asset_url(repo, tag, name):
    return f'https://github.com/{repo}/releases/download/{tag}/{name}'


def fetch_url(url):
    try:
        with urllib.request.urlopen(url, timeout=30) as response:
            data = response.read(LIMIT + 1)
    except (urllib.error.URLError, OSError) as error:
        raise RenderError(f'could not read {url}: {error}') from None
    if len(data) > LIMIT:
        raise RenderError(f'{url} is larger than a checksum file')
    return data


def checksum(data, name, url):
    # sha256sum's format, exactly as package.py writes it: "<hex>  <name>\n".
    try:
        text = data.decode('ascii')
    except UnicodeDecodeError:
        raise RenderError(f'{url} is not a checksum file') from None
    match = re.fullmatch(r'([0-9a-f]{64})  (\S+)\n?', text)
    if not match:
        raise RenderError(f'{url} is not "<sha256>  <file>"')
    if match.group(2) != name:
        raise RenderError(f'{url} names {match.group(2)}, not {name}')
    return match.group(1)


def render(tag, repo=REPO, fetch=fetch_url, template=None):
    match = TAG.fullmatch(tag)
    if not match:
        raise RenderError(f'{tag!r} is not a version tag (v<major>.<minor>.<patch>[-pre])')
    if not REPO_NAME.fullmatch(repo):
        raise RenderError(f'{repo!r} is not owner/name')
    text = TEMPLATE.read_text() if template is None else template
    values = {'@REPO@': repo, '@TAG@': tag, '@VERSION@': match.group(1)}
    for platform, placeholder in PLATFORMS.items():
        name = asset_name(tag, platform)
        url = asset_url(repo, tag, name + '.sha256')
        values[placeholder] = checksum(fetch(url), name, url)
    for placeholder, value in values.items():
        if placeholder not in text:
            raise RenderError(f'the template has no {placeholder}')
        text = text.replace(placeholder, value)
    left = sorted(set(re.findall(r'@[A-Z0-9_]+@', text)))
    if left:
        raise RenderError('unfilled placeholders: ' + ', '.join(left))
    # Every archive URL in the formula is one whose checksum was just read.
    expected = {asset_url(repo, tag, asset_name(tag, p)) for p in PLATFORMS}
    found = set(re.findall(r'url "([^"]+)"', text))
    if found != expected:
        raise RenderError(f'formula URLs {sorted(found)} are not the checked assets {sorted(expected)}')
    return text


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--tag', required=True, help='the published release tag, e.g. v1.0.0-alpha.3')
    parser.add_argument('--repo', default=REPO, help='owner/name on GitHub (default %(default)s)')
    parser.add_argument('--output', type=Path, help='write here instead of stdout; refuses an existing path')
    args = parser.parse_args(argv)
    try:
        text = render(args.tag, args.repo)
    except RenderError as error:
        print(f'render: {error}; nothing written', file=sys.stderr)
        return 1
    if args.output is None:
        sys.stdout.write(text)
    else:
        try:
            with open(args.output, 'x') as out:
                out.write(text)
        except FileExistsError:
            print(f'render: {args.output} exists; nothing written', file=sys.stderr)
            return 1
    return 0


if __name__ == '__main__':
    sys.exit(main())
