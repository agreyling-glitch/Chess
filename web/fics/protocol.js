// FICS style 12 fields: https://www.freechess.org/Help/HelpFiles/style12.html
export function parseStyle12(line) {
  const fields = line.trim().split(/\s+/);
  if (fields[0] !== '<12>' || fields.length < 31 ||
      !fields.slice(1, 9).every(rank => /^[prnbqkPRNBQK-]{8}$/.test(rank)) ||
      !['W', 'B'].includes(fields[9])) return null;
  const relation = Number(fields[19]);
  if (!Number.isInteger(relation)) return null;
  return {
    ranks: fields.slice(1, 9), turn: fields[9], game: Number(fields[16]),
    white: fields[17], black: fields[18], relation,
    whiteTime: Number(fields[24]), blackTime: Number(fields[25]),
    moveNumber: Number(fields[26]), lastMove: fields[27], lastSan: fields[29],
    flipped: fields[30] === '1',
  };
}

export function parseFicsChunk(buffer, chunk) {
  const combined = (buffer + chunk)
    .replace(/\x1b\[[0-9;]*[A-Za-z]/g, '')
    .replace(/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/g, '')
    .replace(/\r/g, '');
  const lines = combined.split('\n');
  return { lines: lines.slice(0, -1), rest: lines.at(-1).slice(-8192) };
}

export function squareAt(rank, file) {
  return 'abcdefgh'[file] + String(8 - rank);
}

// Keep the legacy SAN-only API for live-history backfill.
export function parseFicsMoveRow(line) {
  return parseFicsTimedMoveRow(line)?.moves.map(move=>move.san) ?? null;
}

function elapsedSeconds(value) {
  const parts=value?.match(/^(?:(\d+):)?(\d+):([0-5]\d(?:\.\d+)?)$/);
  if(!parts || (parts[1]!==undefined && Number(parts[2])>=60))return null;
  const seconds=Number(parts[1]||0)*3600+Number(parts[2])*60+Number(parts[3]);
  return Number.isFinite(seconds)&&Number.isSafeInteger(Math.round(seconds*1000))?seconds:null;
}

export function parseFicsTimedMoveRow(line) {
  const row=line.match(/^\s*(\d+)\.\s+(.+)$/);
  if(!row)return null;
  const number=Number(row[1]);if(!Number.isSafeInteger(number)||number<1||number>100000)return null;
  const moves=[];
  const tokens=row[2].matchAll(/(\S+?)(?:\s+\(([^)]*)\))?(?=\s|$)/g);
  for(const token of tokens) {
    const san=token[1];
    if(/^(?:\{|\*|1-0|0-1|1\/2-1\/2)/.test(san))break;
    if(!/^(?:O-O(?:-O)?|0-0(?:-0)?|[KQRBN]?[a-h]?[1-8]?x?[a-h][1-8](?:=[QRBN])?)[+#]?[!?]*$/.test(san))return null;
    moves.push({san,elapsed:elapsedSeconds(token[2]),ply:(number-1)*2+moves.length});
    if(moves.length===2)break;
  }
  return moves.length?{number,moves}:null;
}

export function parseFicsTimeControl(line) {
  const match=line.match(/initial time:\s*(\d+(?:\.\d+)?)\s*(minutes?|seconds?)\s*,?\s*increment:\s*(\d+(?:\.\d+)?)\s*seconds?/i);
  if(!match)return null;
  const initial=Number(match[1])*(match[2].toLowerCase().startsWith('minute')?60:1),increment=Number(match[3]);
  return [initial,increment].every(value=>Number.isFinite(value)&&value>=0&&Number.isSafeInteger(Math.round(value*1000)))?{initial,increment}:null;
}

function pgnTime(seconds) {
  const ms=Math.round(seconds*1000);
  return `${Math.floor(ms/3600000)}:${String(Math.floor(ms/60000)%60).padStart(2,'0')}:${String(Math.floor(ms/1000)%60).padStart(2,'0')}.${String(ms%1000).padStart(3,'0')}`;
}

export function ficsHistoryPgn(request,result) {
  const control=request.timeControl;
  const timed=control&&(control.initial>0||control.increment>0);
  const remaining=timed?[control.initial,control.initial]:[null,null];
  let expectedPly=0;
  const notation=request.moves.map(move=>{
    if(move.ply!==expectedPly){remaining[0]=null;remaining[1]=null;}
    expectedPly=move.ply+1;
    const side=move.ply%2,comments=[];
    if(move.elapsed!==null) {
      comments.push(`[%emt ${pgnTime(move.elapsed)}]`);
      if(remaining[side]!==null) {
        remaining[side]=Math.max(0,remaining[side]-move.elapsed+control.increment);
        comments.push(`[%clk ${pgnTime(remaining[side])}]`);
      }
    } else remaining[side]=null;
    const number=side===0?`${Math.floor(move.ply/2)+1}. `:'';
    return `${number}${move.san}${comments.length?` { ${comments.join(' ')} }`:''}`;
  }).join(' ');
  const timeControl=timed?`[TimeControl "${control.initial}+${control.increment}"]\n`:'';
  return `[Event "FICS history game"]\n[Site "freechess.org"]\n[White "${request.white}"]\n[Black "${request.black}"]\n[Result "${result}"]\n${timeControl}\n${notation} ${result}`;
}


// Original parser for the documented FICS finger rating table.
export function parseFicsRating(line) {
  const match = line.match(/^\s*(Lightning|Blitz|Standard)\s+(\d+[PpEe*]?|----|\+\+\+\+)\s+(\d+(?:\.\d+)?)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)(?:\s+(\d+))?/i);
  if (!match) return null;
  return {category:match[1].toLowerCase(),rating:match[2],rd:Number(match[3]),wins:Number(match[4]),losses:Number(match[5]),draws:Number(match[6]),total:Number(match[7]),best:match[8] ? Number(match[8]) : null};
}
