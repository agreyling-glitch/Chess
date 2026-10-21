#!/usr/bin/env python3
"""Offline integration comparison against python-chess 1.11.2, using real Syzygy files."""
import argparse
from collections import Counter
from contextlib import contextmanager
from datetime import datetime, timezone
import hashlib
import importlib.metadata
import json
from pathlib import Path
import platform
import queue
import random
import secrets
import shutil
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / 'server/validation/syzygy-fixtures.json'
CORPUS = ROOT / 'server/validation/tablebase-cases.json'
ORIGIN = 'http://localhost:8788'


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_dataset(directory):
    manifest = json.loads(MANIFEST.read_text(encoding='utf-8'))
    for item in manifest['files']:
        path = directory / item['name']
        if not path.is_file() or path.stat().st_size != item['bytes'] or sha256(path) != item['sha256']:
            raise ValueError(f"Missing or changed fixture: {item['name']}; run the explicit fixture preparation command")
    return manifest


@contextmanager
def companion(binary, directory):
    with tempfile.TemporaryDirectory(prefix='ironwood-real-tb-') as temporary:
        root = Path(temporary)
        token = secrets.token_hex(32)
        (root / 'owner-token').write_text(token, encoding='utf-8')
        config = dict(listen='127.0.0.1:0', allowed_origins=[ORIGIN], token_file='owner-token',
                      archive=None, tablebase_directories=[str(directory.resolve())],
                      limits=dict(requests_per_minute=10000))
        (root / 'config.json').write_text(json.dumps(config), encoding='utf-8')
        child = subprocess.Popen([str(binary), '--config', str(root / 'config.json')],
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                                 creationflags=subprocess.CREATE_NO_WINDOW if platform.system() == 'Windows' else 0)
        lines = queue.Queue()
        def reader():
            for line in child.stdout:
                lines.put(line)
        thread = threading.Thread(target=reader, daemon=True)
        thread.start()
        address = None
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        def request(path, method='GET'):
            req = urllib.request.Request(address + '/api/v1/' + path, method=method,
                  headers={'Origin': ORIGIN, 'Authorization': 'Bearer ' + token, 'X-Ironwood-Protocol': '1'})
            try:
                with opener.open(req, timeout=20) as response:
                    return response.status, json.load(response)
            except urllib.error.HTTPError as error:
                return error.code, json.load(error)
        try:
            deadline = time.monotonic() + 10
            while time.monotonic() < deadline:
                if child.poll() is not None:
                    raise RuntimeError('Companion failed to start')
                try:
                    event = json.loads(lines.get(timeout=0.1))
                except (queue.Empty, json.JSONDecodeError):
                    continue
                if event.get('event') == 'listening':
                    address = 'http://' + event['fields']['address']
                    break
            if address is None:
                raise RuntimeError('Companion startup timed out')
            yield request
        finally:
            if child.poll() is None:
                try:
                    if address:
                        request('shutdown', 'POST')
                    child.wait(timeout=10)
                except (OSError, subprocess.TimeoutExpired):
                    child.kill()
                    child.wait(timeout=5)
            child.stdout.close()
            thread.join(timeout=2)


def categories(wdl, dtz, clock, checkmate=False):
    """Independent five-valued result plus the documented rounded 100-ply boundary.

    Python's WDL assumes a zeroed clock; its signed DTZ50'' includes the 100-ply
    offset for natural cursed/blessed outcomes. Do not interpret DTZ as DTM.
    Python does not expose rounding provenance, so both outcomes at exactly the
    boundary are retained here; the caller separately requires rounded=true for
    any ambiguous response. No broad +/-1 DTZ tolerance is allowed.
    """
    names = {-2: 'loss', -1: 'blessed-loss', 0: 'draw', 1: 'cursed-win', 2: 'win'}
    if checkmate:
        return {'loss'}
    if not clock or not dtz:
        return {names[wdl]}
    distance = abs(dtz) + clock
    outcome = ('win' if dtz > 0 else 'loss') if distance <= 100 else ('cursed-win' if dtz > 0 else 'blessed-loss')
    return {outcome, 'maybe-win' if dtz > 0 else 'maybe-loss'} if distance == 100 else {outcome}


def compare_position(table, board, actual):
    wdl, dtz = table.probe_wdl(board), table.probe_dtz(board)
    assert actual.get('dtz') == dtz, f"DTZ: expected {dtz}, got {actual.get('dtz')}"
    assert isinstance(actual.get('dtzRounded'), bool), 'Missing DTZ precision flag'
    allowed = categories(wdl, dtz, board.halfmove_clock, board.is_checkmate())
    if dtz == 1:
        for move in board.legal_moves:
            child = board.copy()
            child.push(move)
            if child.is_checkmate():
                allowed = {'win'}
                assert not actual['dtzRounded'], 'A directly proven mate in one has exact DTZ'
                break
    assert actual.get('category') in allowed, f"Category: expected {sorted(allowed)}, got {actual.get('category')}"
    if actual.get('category', '').startswith('maybe-'):
        assert actual['dtzRounded'], 'Ambiguous result must identify rounding'
    if dtz == 0:
        assert not actual['dtzRounded'], 'A draw has exact DTZ zero'
    assert actual.get('dtm') is None, 'Syzygy does not supply DTM'


def cases(chess, samples):
    corpus = json.loads(CORPUS.read_text(encoding='utf-8'))['cases']
    result = []
    for case in corpus:
        board = chess.Board(case['fen'])
        assert board.is_valid(), case['id']
        result.append((case['id'], board, case['tags']))
        result.append((case['id'] + '-mirrored', board.mirror(), case['tags'] + ['mirror']))
    # Exercise both sides of the fifty-move threshold, retaining identical placement.
    queen = chess.Board(corpus[0]['fen'])
    for turn in [chess.WHITE, chess.BLACK]:
        for clock in [1, 85, 86, 87, 88, 89, 90, 97, 98, 99, 100]:
            board = queen.copy()
            board.turn, board.halfmove_clock = turn, clock
            result.append((f'queen-clock-{turn}-{clock}', board, ['halfmove']))
    rng = random.Random(7811)
    for material in ['KQvK', 'KRvK', 'KPvK', 'KPvKP', 'KBBvK', 'KBvKN', 'KBBvKN']:
        white, black = material.split('v')
        pieces = white + black.lower()
        accepted = 0
        while accepted < samples:
            board = chess.Board(None)
            for square, piece in zip(rng.sample(range(64), len(pieces)), pieces):
                board.set_piece_at(square, chess.Piece.from_symbol(piece))
            board.turn = bool(rng.randrange(2))
            if board.is_valid():
                result.append((f'sample-{material}-{accepted}', board, ['sample']))
                accepted += 1
    return result


def failure_cases(binary, directory, chess):
    """Only modify disposable copies, never the input dataset."""
    checked = []
    with tempfile.TemporaryDirectory(prefix='ironwood-tb-replacement-') as temporary:
        scratch = Path(temporary)
        fen = '8/8/8/8/8/2k5/8/KQ6 w - - 0 1'
        path = 'tablebases/probe?' + urllib.parse.urlencode(dict(fen=fen))
        wdl, dtz = scratch / 'KQvK.rtbw', scratch / 'KQvK.rtbz'
        with companion(binary, scratch) as request:
            assert request(path)[1]['code'] == 'missing_table'
            shutil.copyfile(directory / wdl.name, wdl)
            assert request(path)[1]['code'] == 'missing_table'
            checked.append('wdl-only-install')
            shutil.copyfile(directory / dtz.name, dtz)
            assert request(path)[0] == 200
            for file in [wdl, dtz]:
                file.write_bytes(bytes(80))
                assert request(path)[1]['code'] == 'corrupt_table'
                shutil.copyfile(directory / file.name, file)
                assert request(path)[0] == 200
            checked.append('corrupt-wdl-and-dtz-restored')
            dtz.unlink()
            assert request(path)[1]['code'] == 'missing_table'
            shutil.copyfile(directory / dtz.name, dtz)
            assert request(path)[0] == 200
            checked.append('removed-and-reinstalled-dtz')
            for invalid, code in [('bad', 'invalid_position'),
                ('4k3/8/8/8/8/8/8/4K2R w K - 0 1', 'unsupported_position'),
                (chess.STARTING_FEN, 'unsupported_position')]:
                assert request('tablebases/probe?' + urllib.parse.urlencode(dict(fen=invalid)))[1]['code'] == code
            checked.append('invalid-castling-and-over-seven-pieces')
        wdl.unlink()
        dtz.unlink()
        # Black can capture a bishop; that successor requires KBvKN, not merely
        # the five-piece root tables. A partial installation must fail honestly.
        for suffix in ['rtbw', 'rtbz']:
            shutil.copyfile(directory / ('KBBvKN.' + suffix), scratch / ('KBBvKN.' + suffix))
        capture = '6K1/8/8/B7/8/B7/k7/5n2 b - - 0 1'
        with companion(binary, scratch) as request:
            status, error = request('tablebases/probe?' + urllib.parse.urlencode(dict(fen=capture)))
            assert status == 422 and error['code'] == 'missing_table'
            assert 'KBvKN' in error['message'], 'Missing capture successor must be identified'
        checked.append('missing-capture-successor')
    return checked


def validate(args):
    import chess
    import chess.syzygy
    version = importlib.metadata.version('chess')
    if version != '1.11.2':
        raise ValueError('Use the pinned development reference: chess==1.11.2')
    manifest = verify_dataset(args.tables)
    report = dict(format='ironwood-tablebase-validation', version=1,
        timestamp=datetime.now(timezone.utc).isoformat(), status='running',
        reference=dict(implementation='python-chess', version=version, license='GPL-3.0-or-later', offline=True, sourceSha256=sha256(Path(chess.syzygy.__file__))),
        validatorSha256=sha256(Path(__file__)),
        platform=dict(system=platform.system(), machine=platform.machine()),
        binarySha256=sha256(args.binary), fixtureManifestSha256=sha256(MANIFEST),
        corpusSha256=sha256(CORPUS), fixtureFiles=len(manifest['files']),
        fixtureBytes=sum(f['bytes'] for f in manifest['files']), cases=[], failures=[], assertions={})
    counts, features, timings = Counter(), Counter(), []
    with chess.syzygy.open_tablebase(str(args.tables)) as table, companion(args.binary, args.tables) as request:
        status, coverage = request('tablebases')
        assert status == 200 and not coverage['rejectedFiles'] and not coverage['unavailableDirectories']
        for name, board, tags in cases(chess, args.samples):
            fen = board.fen(en_passant='fen')
            started = time.perf_counter()
            status, actual = request('tablebases/probe?' + urllib.parse.urlencode(dict(fen=fen)))
            elapsed = (time.perf_counter() - started) * 1000
            timings.append(elapsed)
            try:
                assert status == 200, f"HTTP {status}: {actual}"
                assert actual['source'] == 'companion'
                compare_position(table, board, actual)
                legal = {move.uci(): move for move in board.legal_moves}
                rows = actual['moves']
                assert len(rows) == len(legal) and {row['uci'] for row in rows} == set(legal), 'Legal move set differs'
                for row in rows:
                    move = legal[row['uci']]
                    assert row['san'] == board.san(move), 'SAN differs'
                    child = board.copy()
                    child.push(move)
                    compare_position(table, child, row)
                    assert row['zeroing'] == (child.halfmove_clock == 0), 'Zeroing flag differs'
                    counts['continuations'] += 1
                counts[actual['category']] += 1
                counts['roots'] += 1
                if any(board.is_en_passant(move) for move in legal.values()):
                    features['legalEnPassant'] += 1
                if {chess.QUEEN, chess.ROOK, chess.BISHOP, chess.KNIGHT}.issubset({m.promotion for m in legal.values()}):
                    features['allFourPromotions'] += 1
                if board.is_checkmate() and board.halfmove_clock >= 100:
                    features['mateAtClock100'] += 1
                if board.halfmove_clock == 0 and actual['category'] in {'cursed-win', 'blessed-loss'}:
                    features['natural-' + actual['category']] += 1
                if 'mirror' in tags:
                    features['mirroredRoots'] += 1
                report['cases'].append(dict(id=name, fen=fen, category=actual['category'], dtz=actual['dtz'],
                                             dtzRounded=actual['dtzRounded'], legalMoves=len(rows), tags=tags))
            except (AssertionError, KeyError, ValueError) as error:
                report['failures'].append(dict(id=name, fen=fen, error=str(error)))
    try:
        report['failureChecks'] = failure_cases(args.binary, args.tables, chess)
        verify_dataset(args.tables)
    except (AssertionError, ValueError, KeyError) as error:
        report['failures'].append(dict(id='failure-checks-or-dataset-changed', error=str(error)))
    required = {'win', 'draw', 'loss', 'cursed-win', 'blessed-loss', 'maybe-win', 'maybe-loss'}
    if not required.issubset(counts):
        report['failures'].append(dict(id='missing-category-coverage', error=str(sorted(required - counts.keys()))))
    required_features = {'legalEnPassant', 'allFourPromotions', 'mateAtClock100', 'natural-cursed-win', 'natural-blessed-loss', 'mirroredRoots'}
    if not required_features.issubset(features):
        report['failures'].append(dict(id='missing-rule-coverage', error=str(sorted(required_features - features.keys()))))
    report['featureCoverage'] = dict(features)
    report['assertions'] = dict(counts)
    report['latencyMs'] = dict(min=round(min(timings), 3), median=round(sorted(timings)[len(timings)//2], 3), max=round(max(timings), 3))
    report['status'] = 'passed' if not report['failures'] else 'failed'
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(dict(status=report['status'], assertions=report['assertions'], failures=report['failures'], report=str(args.report))))
    return 0 if report['status'] == 'passed' else 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tables', type=Path, default=ROOT / 'tmp/companion-tablebase-validation/tables')
    parser.add_argument('--binary', type=Path, default=ROOT / ('target/debug/ironwood-server.exe' if platform.system() == 'Windows' else 'target/debug/ironwood-server'))
    parser.add_argument('--report', type=Path, default=ROOT / 'tmp/companion-tablebase-validation/report.json')
    parser.add_argument('--samples', type=int, default=12, choices=range(0, 101), metavar='0..100')
    args = parser.parse_args()
    try:
        if not __debug__:
            raise RuntimeError('Run this validator without Python -O; assertions are required')
        exit_code = validate(args)
    except Exception as error:
        # Never leave a previous passing report behind after a failed preflight.
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(dict(format='ironwood-tablebase-validation', version=1,
            status='failed', failures=[dict(id='validation-preflight-or-run', error=str(error))]), indent=2) + '\n', encoding='utf-8')
        print('Validation failed: ' + str(error))
        exit_code = 1
    raise SystemExit(exit_code)
