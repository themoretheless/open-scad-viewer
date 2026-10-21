"""Exercise replacement/recovery using temporary bytes only, never product paths."""
from pathlib import Path
import importlib.util
import os
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
OUT = Path(__file__).resolve().parent


def load(name):
    spec = importlib.util.spec_from_file_location(name, OUT / f'{name}.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


publisher = load('publish-v11-guarded-20261018')
inventory = load('prepare-v11-delivery-20261018').inventory


class DeliveryGuards(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='sweep-delivery-guard-test-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.destination = self.root / 'primary' / 'target'
        self.incoming = self.root / 'incoming' / 'target'
        self.displaced = self.root / 'backup' / 'displaced'

    @staticmethod
    def file(path, value):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(value)

    def apply(self, before, after):
        publisher.apply_target(self.destination, self.incoming, self.displaced,
                               before, after, inventory)

    def test_existing_file_replaced_and_original_backup_preserved(self):
        self.file(self.destination, b'original peer bytes')
        self.file(self.incoming, b'qualified candidate')
        backup = self.root / 'backup' / 'original'
        self.file(backup, self.destination.read_bytes())
        before, after = inventory(self.destination), inventory(self.incoming)
        self.apply(before, after)
        self.assertEqual(self.destination.read_bytes(), b'qualified candidate')
        self.assertEqual(backup.read_bytes(), b'original peer bytes')

    def test_new_file_requires_absence(self):
        self.file(self.incoming, b'new metadata')
        self.apply(None, inventory(self.incoming))
        self.assertEqual(self.destination.read_bytes(), b'new metadata')

    def test_complete_directory_swap_preserves_removed_original_entries(self):
        self.file(self.destination / 'old.bin', b'original old artifact')
        self.file(self.destination / 'nested' / 'extra', b'untracked peer work')
        self.file(self.incoming / 'new.bin', b'candidate artifact')
        before, after = inventory(self.destination), inventory(self.incoming)
        self.apply(before, after)
        self.assertEqual(inventory(self.destination), after)
        self.assertEqual(inventory(self.displaced), before)
        self.assertEqual((self.displaced / 'nested' / 'extra').read_bytes(), b'untracked peer work')

    def test_failed_directory_install_restores_original_location(self):
        self.file(self.destination / 'old.bin', b'old')
        self.file(self.incoming / 'new.bin', b'new')
        before, after = inventory(self.destination), inventory(self.incoming)
        rename = os.rename

        def fail_incoming(source, target):
            if source == self.incoming:
                raise OSError('injected directory install failure')
            return rename(source, target)

        with patch.object(publisher.os, 'rename', side_effect=fail_incoming):
            with self.assertRaisesRegex(OSError, 'injected'):
                self.apply(before, after)
        self.assertEqual(inventory(self.destination), before)
        self.assertEqual(inventory(self.incoming), after)
        self.assertFalse(self.displaced.exists())

    def test_concurrent_writer_is_never_overwritten(self):
        self.file(self.destination, b'original')
        self.file(self.incoming, b'candidate')
        before, after = inventory(self.destination), inventory(self.incoming)
        self.destination.write_bytes(b'concurrent peer edit')
        with self.assertRaisesRegex(AssertionError, 'Concurrent write'):
            self.apply(before, after)
        self.assertEqual(self.destination.read_bytes(), b'concurrent peer edit')
        self.assertEqual(inventory(self.incoming), after)

    def test_changed_incoming_is_refused_before_displacing_primary(self):
        self.file(self.destination / 'original', b'original')
        self.file(self.incoming / 'candidate', b'candidate')
        before, after = inventory(self.destination), inventory(self.incoming)
        (self.incoming / 'candidate').write_bytes(b'changed candidate')
        with self.assertRaisesRegex(AssertionError, 'Incoming delivery changed'):
            self.apply(before, after)
        self.assertEqual(inventory(self.destination), before)
        self.assertFalse(self.displaced.exists())

    def test_symlink_target_cannot_alias_peer_file(self):
        peer = self.root / 'peer'
        self.file(peer, b'peer work')
        self.destination.parent.mkdir(parents=True)
        self.destination.symlink_to(peer)
        self.file(self.incoming, b'candidate')
        with self.assertRaisesRegex(ValueError, 'symlink delivery target'):
            self.apply(None, inventory(self.incoming))
        self.assertEqual(peer.read_bytes(), b'peer work')

    def test_symlink_in_tree_is_refused(self):
        peer = self.root / 'peer'
        self.file(peer, b'peer work')
        self.destination.mkdir(parents=True)
        (self.destination / 'alias').symlink_to(peer)
        with self.assertRaisesRegex(ValueError, 'symlink in artifact tree'):
            inventory(self.destination)
        self.assertEqual(peer.read_bytes(), b'peer work')


if __name__ == '__main__':
    unittest.main(verbosity=2)
