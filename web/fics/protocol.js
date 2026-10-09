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

// FICS "moves" lists SAN in two columns, optionally followed by time spent.
export function parseFicsMoveRow(line) {
  const match = line.match(/^\s*\d+\.\s+(.+)$/);
  if (!match) return null;
  const tokens = match[1].replace(/\s+\(\d+(?::\d{2}){1,2}\)/g, '').trim().split(/\s+/);
  if (!tokens[0] || /^(?:\{|\*|1-0|0-1|1\/2-1\/2)/.test(tokens[0])) return null;
  return tokens.slice(0, 2).filter(token => token && !/^(?:\{|\*|1-0|0-1|1\/2-1\/2)/.test(token));
}


// Original parser for the documented FICS finger rating table.
export function parseFicsRating(line) {
  const match = line.match(/^\s*(Lightning|Blitz|Standard)\s+(\d+[PpEe*]?|----|\+\+\+\+)\s+(\d+(?:\.\d+)?)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)(?:\s+(\d+))?/i);
  if (!match) return null;
  return {category:match[1].toLowerCase(),rating:match[2],rd:Number(match[3]),wins:Number(match[4]),losses:Number(match[5]),draws:Number(match[6]),total:Number(match[7]),best:match[8] ? Number(match[8]) : null};
}
