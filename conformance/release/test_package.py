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


if __name__=='__main__':unittest.main()

