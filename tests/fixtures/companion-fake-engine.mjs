// Original deterministic UCI fixture. Not a playing engine or third-party asset.
import readline from 'node:readline';
import { writeFileSync, appendFileSync } from 'node:fs';
const mode = process.argv[2] || 'normal';
const log = process.argv[3];
const pid = process.argv[4];
if (pid) writeFileSync(pid, mode === 'cwd' ? process.cwd() : String(process.pid));
let timer, position = '', threads = '1';
const emit = line => process.stdout.write(line + '\n');
readline.createInterface({ input: process.stdin }).on('line', line => {
  if (log) appendFileSync(log, line + '\n');
  if (line === 'uci' && mode !== 'no-handshake') {
    emit('id name Ironwood Test Engine'); emit('id author Ironwood tests');
    emit('option name Threads type spin default 1 min 1 max 4');
    emit('option name Hash type spin default 16 min 1 max 128');
    emit('option name MultiPV type spin default 1 min 1 max 4');
    emit('option name Ponder type check default false');
    emit('option name Playing Style type combo default Normal var Normal var Very Aggressive');
    emit('option name EvalFile type string default default.nnue');
    emit('option name BackendOptions type string default <empty>');
    emit('option name Clear Hash type button');
    emit('option name SyzygyPath type string default <empty>');
    if (mode === 'partial') { process.stdout.write('uci'); setTimeout(() => process.stdout.write('ok\n'), 10); }
    else emit('uciok');
  } else if (line === 'isready') emit('readyok');
  else if (line.startsWith('setoption name Threads value ')) threads = line.split(' ').at(-1);
  else if (line.startsWith('position ')) position = line;
  else if (line.startsWith('go ')) {
    if (mode === 'crash') process.exit(7);
    if (mode === 'long-line') { process.stdout.write('x'.repeat(20000)); return; }
    if (mode === 'stderr') process.stderr.write('private diagnostic'.repeat(20000));
    const black = / b /.test(position) || /moves e2e4$/.test(position);
    const move = /4p3\/4P3/.test(position) ? (black ? 'b8c6' : 'g1f3') : (black ? 'e7e5' : 'e2e4');
    emit(`info depth 7 nodes 123 score cp ${black ? '-23' : '23'} multipv 1 pv ${move}`);
    emit('info string ordinary commentary must not become a score');
    emit('malformed junk');
    if (!line.includes('infinite')) timer = setTimeout(() => emit(`bestmove ${mode === 'illegal' ? 'e2e5' : move}`), 80);
  } else if (line === 'stop') {
    if (mode === 'ignore-stop') return;
    clearTimeout(timer);
    emit('info depth 8 score cp 999 pv e2e4');
    setTimeout(() => emit('bestmove e2e4'), 20);
  } else if (line === 'quit' && mode !== 'ignore-stop') process.exit(0);
});
