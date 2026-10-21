"""Checks that the independent validation runner rejects bad evidence."""
import copy
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import subprocess
import sys
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
(ROOT / 'tmp').mkdir(exist_ok=True)
def module(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / 'scripts' / filename)
    value = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(value)
    return value

validator = module('tablebase_validator', 'validate-companion-tablebases.py')
preparer = module('tablebase_preparer', 'prepare-companion-tablebase-fixtures.py')


class ValidationTests(unittest.TestCase):
    def test_clock_boundary_and_terminal_precedence(self):
        self.assertEqual(validator.categories(2, 11, 0), {'win'})
        self.assertEqual(validator.categories(-2, -14, 99), {'blessed-loss'})
        self.assertEqual(validator.categories(1, 103, 0), {'cursed-win'})
        self.assertEqual(validator.categories(-1, -116, 0), {'blessed-loss'})
        self.assertEqual(validator.categories(2, 11, 89), {'win', 'maybe-win'})
        self.assertEqual(validator.categories(-2, -1, 100, True), {'loss'})

    def test_wrong_distance_perspective_and_rounding_fail(self):
        import chess
        class Reference:
            def probe_wdl(self, board): return -2
            def probe_dtz(self, board): return -14
        board = chess.Board('8/8/8/8/8/2k5/8/KQ6 b - - 0 1')
        good = dict(category='loss', dtz=-14, dtzRounded=True, dtm=None)
        validator.compare_position(Reference(), board, good)
        for change in [dict(category='win'), dict(dtz=14), dict(dtz=-13), dict(dtm=8)]:
            with self.assertRaises(AssertionError):
                validator.compare_position(Reference(), board, dict(good, **change))
        board.halfmove_clock = 86
        with self.assertRaises(AssertionError):
            validator.compare_position(Reference(), board, dict(good, category='maybe-loss', dtzRounded=False))

    def test_changed_or_missing_file_fails_preflight(self):
        with tempfile.TemporaryDirectory(dir=ROOT / 'tmp') as temporary:
            root = Path(temporary)
            file = root / 'KQvK.rtbw'
            file.write_bytes(b'original')
            manifest = root / 'manifest.json'
            manifest.write_text(json.dumps(dict(files=[dict(name=file.name, bytes=8, sha256=validator.sha256(file))])))
            with patch.object(validator, 'MANIFEST', manifest):
                validator.verify_dataset(root)
                file.write_bytes(b'changed!')
                with self.assertRaises(ValueError): validator.verify_dataset(root)
                file.unlink()
                with self.assertRaises(ValueError): validator.verify_dataset(root)

    def test_failed_preflight_replaces_an_old_passing_report(self):
        with tempfile.TemporaryDirectory(dir=ROOT / 'tmp') as temporary:
            root = Path(temporary)
            report = root / 'report.json'
            report.write_text('{"status":"passed"}')
            result = subprocess.run([sys.executable, str(ROOT / 'scripts/validate-companion-tablebases.py'),
                '--tables', str(root), '--report', str(report)], capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 1)
            self.assertEqual(json.loads(report.read_text())['status'], 'failed')

    def test_downloader_does_not_accept_changed_bytes(self):
        with tempfile.TemporaryDirectory(dir=ROOT / 'tmp') as temporary:
            root = Path(temporary)
            manifest = copy.deepcopy(json.loads(preparer.MANIFEST.read_text()))
            manifest['files'] = manifest['files'][:1]
            lock = root / 'manifest.json'
            lock.write_text(json.dumps(manifest))
            with patch.object(preparer, 'MANIFEST', lock):
                with self.assertRaises(ValueError):
                    preparer.prepare(root / 'tables', fetch=lambda *a, **kw: io.BytesIO(b'untrusted'))
            self.assertEqual(list((root / 'tables').iterdir()), [])


if __name__ == '__main__':
    unittest.main()
