// PGN generation for manually entered scoresheets.
export function normalizeMoves(text) {
  return text.normalize('NFKC').replace(/[–—−]/g, '-')
    .replace(/\b[0Oo]-[0Oo](?:-[0Oo])?\b/g, match => match.toUpperCase().replaceAll('0', 'O'))
    .replace(/\b(\d+)\s*[.)]\s*/g, '$1. ')
    .replace(/\b(\d+)\s+(?=[KQRBN]?[a-h][1-8])/g, '$1. ')
    .replace(/\s+/g, ' ').trim();
}

export function scoresheetPgn(moves, headers = {}) {
  const result = ['*', '1-0', '0-1', '1/2-1/2'].includes(headers.Result) ? headers.Result : '*';
  const tags = { Event: headers.Event || 'Scoresheet import', Site: '?', Date: headers.Date || '????.??.??', Round: '?', White: headers.White || '?', Black: headers.Black || '?', Result: result };
  const safe = value => String(value).replace(/[\r\n]/g, ' ').replaceAll('\\', '\\\\').replaceAll('"', '\\"');
  // A conflicting game result must be corrected explicitly.
  const cleaned = normalizeMoves(moves);
  const tokens = cleaned.split(/\s+/).filter(Boolean);
  let halfMoves = 0;
  for (const [index, token] of tokens.entries()) {
    if (/^\d+\.$/.test(token)) {
      if (halfMoves % 2 || Number(token.slice(0, -1)) !== Math.floor(halfMoves / 2) + 1) throw new Error(`Unexpected move number ${token} Check for missing or duplicated rows.`);
    } else if (/^(1-0|0-1|1\/2-1\/2|\*)$/.test(token)) {
      if (index !== tokens.length - 1) throw new Error('The game result must appear after all moves.');
    } else {
      if (!/^(?:[KQRBN]?[a-h]?[1-8]?x?[a-h][1-8](?:=[QRBN])?|O-O(?:-O)?)[+#]?[!?]*$/.test(token)) throw new Error(`Unrecognized text at half-move ${halfMoves + 1}: ${token}. Correct it using the photo.`);
      halfMoves++;
    }
  }
  const trailing = cleaned.match(/(?:^|\s)(1-0|0-1|1\/2-1\/2|\*)$/)?.[1];
  if (trailing && trailing !== result) throw new Error('The result in the moves differs from the selected game result. Correct one of them.');
  return Object.entries(tags).map(([key, value]) => `[${key} "${safe(value)}"]`).join('\n') + '\n\n' + cleaned + (trailing ? '' : ` ${result}`);
}
