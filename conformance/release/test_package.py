# SPDX-License-Identifier: Apache-2.0
import copy
import hashlib
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from install_check import verify
from package import publish_new


def archive(files,extra=None):
    out=io.BytesIO()
    with tarfile.open(fileobj=out,mode='w:gz') as tar:
        for name,data in files.items():
            info=tarfile.TarInfo(name);info.size=len(data);tar.addfile(info,io.BytesIO(data))
        if extra is not None:tar.addfile(extra)
    return out.getvalue()


def fixture():
    paths=['bin/war','sdk-source.tar','LICENSE','CANDIDATE.md','standard/source-set.json','skills/openwarrant/LICENSE.mattpocock']
    files={p:b'fixed verification-only fixture' for p in paths}
    manifest={'schema':'oh.war/candidate-bundle/v1','qualified':False,'promotable':False,
              'files':{p:{'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data),'executable':p=='bin/war'} for p,data in files.items()}}
    files['MANIFEST.json']=json.dumps(manifest).encode()
    return files


class PackageTests(unittest.TestCase):
    def check(self,data):
        with tempfile.TemporaryDirectory() as t:
            path=Path(t)/'candidate.tar.gz';path.write_bytes(data)
            return verify(path,hashlib.sha256(data).hexdigest())

    def test_exact_inventory_accepted_without_executing_fixture(self):
        files=fixture();checked,manifest=self.check(archive(files));self.assertEqual(checked,files)
        self.assertFalse(manifest['qualified'])

    def test_tamper_missing_duplicate_and_false_mark_refuse(self):
        for kind in ['tamper','missing','duplicate','mark','symlink','escape']:
            with self.subTest(kind=kind):
                files=fixture();extra=None
                if kind=='tamper':files['bin/war']+=b'changed'
                elif kind=='missing':del files['LICENSE']
                elif kind=='duplicate':extra=tarfile.TarInfo('LICENSE');extra.size=0
                elif kind=='mark':
                    m=json.loads(files['MANIFEST.json']);m['qualified']=True;files['MANIFEST.json']=json.dumps(m).encode()
                elif kind=='symlink':extra=tarfile.TarInfo('link');extra.type=tarfile.SYMTYPE;extra.linkname='/tmp/outside'
                else:files['../outside']=b'escape'
                with self.assertRaises(ValueError):self.check(archive(files,extra))

    def test_duplicate_manifest_fields_and_outer_checksum_refuse(self):
        files=fixture();files['MANIFEST.json']=files['MANIFEST.json'].replace(b'"qualified": false',b'"qualified": true,"qualified": false')
        with self.assertRaisesRegex(ValueError,'Duplicate'):self.check(archive(files))
        with tempfile.TemporaryDirectory() as t:
            p=Path(t)/'archive';p.write_bytes(archive(fixture()))
            with self.assertRaisesRegex(ValueError,'checksum'):verify(p,'0'*64)

    def test_new_publication_refuses_existing_file_and_symlink(self):
        with tempfile.TemporaryDirectory() as t:
            p=Path(t)/'result';publish_new(p,b'original')
            with self.assertRaises(FileExistsError):publish_new(p,b'changed')
            self.assertEqual(p.read_bytes(),b'original')
            link=Path(t)/'link';link.symlink_to(p)
            with self.assertRaises(FileExistsError):publish_new(link,b'changed')
            self.assertEqual(p.read_bytes(),b'original')
            self.assertEqual(sorted(x.name for x in Path(t).iterdir()),['link','result'])


class ExpandedMetadataTests(unittest.TestCase):
    def test_hidden_pax_metadata_counts_toward_expanded_limit(self):
        from unittest.mock import patch
        out=io.BytesIO()
        with tarfile.open(fileobj=out,mode='w:gz',format=tarfile.PAX_FORMAT) as tar:
            entry=tarfile.TarInfo('small');entry.size=1;entry.pax_headers={'comment':'x'*(2*1024*1024)}
            tar.addfile(entry,io.BytesIO(b'x'))
        with tempfile.TemporaryDirectory() as tmp,patch('install_check.LIMIT',1024*1024):
            path=Path(tmp)/'pax.tgz';path.write_bytes(out.getvalue())
            with self.assertRaisesRegex(ValueError,'Expanded archive size limit'):
                verify(path,hashlib.sha256(path.read_bytes()).hexdigest())


class SourceExpansionTests(unittest.TestCase):
    def test_sparse_source_refuses_before_any_write(self):
        from install_check import extract_source
        out=io.BytesIO()
        with tarfile.open(fileobj=out,mode='w',format=tarfile.PAX_FORMAT) as tar:
            entry=tarfile.TarInfo('large');entry.size=1
            entry.pax_headers={'GNU.sparse.map':'0,1','GNU.sparse.size':str(32*1024*1024)}
            tar.addfile(entry,io.BytesIO(b'x'))
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            with self.assertRaisesRegex(ValueError,'Sparse or over-limit'):
                extract_source(out.getvalue(),root)
            self.assertEqual(list(root.iterdir()),[])


class SourceCapsuleTests(unittest.TestCase):
    def test_compressed_source_limits_corruption_and_profile_confusion_refuse(self):
        import gzip
        from unittest.mock import patch
        from install_check import source_tar
        self.assertEqual(source_tar(fixture(),{'schema':'oh.war/candidate-bundle/v1'}),fixture()['sdk-source.tar'])
        for kind,message in [('expanded','Expanded SDK source size limit'),('truncated','Malformed compressed SDK source'),('garbage','Malformed compressed SDK source'),('ambiguous','Ambiguous'),('future','Unsupported candidate')]:
            with self.subTest(kind=kind),tempfile.TemporaryDirectory() as tmp,patch('install_check.LIMIT',32768):
                files=fixture();files.pop('MANIFEST.json');old=files.pop('sdk-source.tar')
                source=gzip.compress(b'x'*65536 if kind=='expanded' else old,mtime=0)
                if kind=='truncated':source=source[:-4]
                if kind=='garbage':source=b'not gzip'
                files['sdk-source.tar.gz']=source
                if kind=='ambiguous':files['sdk-source.tar']=old
                manifest={'schema':'oh.war/candidate-bundle/v3' if kind=='future' else 'oh.war/candidate-bundle/v2','qualified':False,'promotable':False,
                          'files':{p:{'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data),'executable':p=='bin/war'} for p,data in files.items()}}
                files['MANIFEST.json']=json.dumps(manifest).encode();data=archive(files)
                path=Path(tmp)/'capsule.tgz';path.write_bytes(data)
                with self.assertRaisesRegex(ValueError,message):verify(path,hashlib.sha256(data).hexdigest())
                self.assertEqual([p.name for p in Path(tmp).iterdir()],['capsule.tgz'])

    def test_full_source_fits_without_raising_existing_limits(self):
        import os
        import subprocess
        from unittest.mock import patch
        import package
        import install_check
        self.assertEqual(package.LIMIT,256*1024*1024)
        self.assertEqual(install_check.LIMIT,256*1024*1024)
        with tempfile.TemporaryDirectory() as temporary:
            base=Path(temporary);root=base/'repository';root.mkdir()
            contents={
                'LICENSE':b'MIT test fixture notice\n',
                'Cargo.toml':b'[package]\nname="package-fixture"\nversion="0.1.0"\nedition="2021"\nlicense="MIT"\n',
                'src/lib.rs':b'pub fn fixture() {}\n',
                'tool.sh':b'#!/bin/sh\nexit 0\n',
                'padding.txt':b'x'*(70*1024),
            }
            for path in [
                'crates/openwarrant-core/Cargo.toml',
                'conformance/sdk/document/minimal-rc3.md',
                '.claude/skills/openwarrant/LICENSE.mattpocock',
                '.claude/skills/openwarrant/references/sdk-artifacts.md',
                'docs/sas/drafts/1.0.0-rc.3/format-contract.md',
                'docs/sas/drafts/1.0.0-rc.3/source-set.json',
                'docs/sas/drafts/1.0.0-rc.2/format-contract.md',
                'conformance/sdk/cli/README.md',
                'conformance/skills/check_skills.py',
            ]:contents[path]=b'synthetic source packaging fixture\n'
            contents['.claude/skills/openwarrant/references/sdk-artifacts.md']=b'See ../../../../conformance/sdk/cli/README.md\n'
            contents['docs/sas/drafts/1.0.0-rc.3/format-contract.md']=b'See ../1.0.0-rc.2/format-contract.md\n'
            contents['docs/sas/drafts/1.0.0-rc.3/sdk-contract.md']=b'See ../../../../conformance/sdk/cli/README.md\n'
            for name,data in contents.items():
                path=root/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)
                if name=='tool.sh':path.chmod(0o755)
            (root/'context-link').symlink_to('src/lib.rs')
            subprocess.run(['cargo','generate-lockfile','--offline'],cwd=root,check=True,capture_output=True,timeout=30)
            for args in [
                ['init','-q','--template=','--initial-branch=fixture'],['add','.'],
                ['-c','user.name=fixture','-c','user.email=fixture@invalid','-c','commit.gpgSign=false','-c','core.hooksPath=/dev/null','commit','-qm','fixture'],
            ]:subprocess.run(['git',*args],cwd=root,check=True,capture_output=True,timeout=30)
            binary=base/'war';binary.write_bytes(b'#!/bin/sh\nprintf "war fixture\\n"\n#'+b'x'*(140*1024)+b'\n');binary.chmod(0o700)
            expected={p.decode(): (root/os.fsdecode(p)).read_bytes() for p in subprocess.check_output(['git','ls-files','-z'],cwd=root,timeout=10).split(b'\0') if p}
            with patch('package.ROOT',root),patch('package.LIMIT',200*1024),patch('install_check.LIMIT',200*1024):
                result=package.bundle(binary,base/'output','fixture')
                data=result.read_bytes();files,manifest=install_check.verify(result,hashlib.sha256(data).hexdigest())
                self.assertEqual(manifest['schema'],'oh.war/candidate-bundle/v2')
                self.assertFalse(manifest['qualified']);self.assertFalse(manifest['promotable'])
                extracted=base/'source';extracted.mkdir()
                source=install_check.source_tar(files,manifest)
                self.assertEqual(source,subprocess.check_output(['git','archive','--format=tar','HEAD'],cwd=root,timeout=10))
                install_check.extract_source(source,extracted)
                actual={p.relative_to(extracted).as_posix():p.read_bytes() for p in extracted.rglob('*') if p.is_file()}
                self.assertEqual(actual,expected)
                self.assertTrue((extracted/'context-link').is_symlink())
                self.assertEqual(os.readlink(extracted/'context-link'),'src/lib.rs')
                self.assertTrue((extracted/'tool.sh').stat().st_mode & 0o111)


class SourceSelectionTests(unittest.TestCase):
    """The source archive leaves the repository's evidence out and keeps
    everything the installed bundle is checked against."""

    def test_evidence_is_out_and_required_components_stay(self):
        from package import in_source
        for path in ['archive/legacy-20260914/history.bundle','artifacts/x.json',
                     'docs/evidence/structural-wave-20261014/a.json',
                     'docs/warrants/OW-WAR-0086/implementation/x.json',
                     'docs/warrants/OW-WAR-0001/gate-runs/r.json',
                     'docs/warrants/OW-WAR-0001/verifications/v.json']:
            with self.subTest(path=path):self.assertFalse(in_source(path))
        for path in ['crates/openwarrant-core/Cargo.toml','conformance/sdk/cli/README.md',
                     '.claude/skills/openwarrant/LICENSE.mattpocock','docs/sas/drafts/1.0.0-rc.3/format-contract.md',
                     'docs/format/FORMAT-1.0.md','docs/warrants/OW-WAR-0001/manifest.toml',
                     'docs/warrants/OW-WAR-0001/atoms/10-intent.md','docs/evidence.md']:
            with self.subTest(path=path):self.assertTrue(in_source(path))

    def test_committed_source_fits_the_limit(self):
        from package import LIMIT,git,source_inventory
        inventory=source_inventory(git('rev-parse','HEAD').decode().strip())
        self.assertLess(sum(e['bytes'] for e in inventory.values()),LIMIT//2)
        self.assertLess(len(inventory),16384)
        self.assertFalse(any(p.startswith('archive/') for p in inventory))


if __name__=='__main__':unittest.main()
