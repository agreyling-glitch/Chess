export const STANDARD = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
export function parseFen(fen) {
 const fields=fen.trim().split(/\s+/), ranks=fields[0].split('/');
 if(ranks.length!==8) throw Error('FEN must have eight ranks.');
 const board=ranks.flatMap(rank=>{const row=[];for(const c of rank){if(/[1-8]/.test(c)) row.push(...Array(Number(c)).fill(''));else if(/[prnbqkPRNBQK]/.test(c)) row.push(c);else throw Error('Invalid FEN piece.');}if(row.length!==8)throw Error('Each rank must have eight squares.');return row;});
 if(fields.length!==6 || !/^[wb]$/.test(fields[1]) || !/^(-|[KQkqA-Ha-h]+)$/.test(fields[2]) || !/^(-|[a-h][36])$/.test(fields[3]) || !/^\d+$/.test(fields[4]) || !/^[1-9]\d*$/.test(fields[5])) throw Error('Enter a complete six-field FEN.');
 return {board,turn:fields[1],rights:fields[2],ep:fields[3],half:fields[4],full:fields[5]};
}
export function placement(board){return Array.from({length:8},(_,r)=>{let s='',empty=0;for(const p of board.slice(r*8,r*8+8)){if(!p)empty++;else{if(empty)s+=empty;empty=0;s+=p;}}return s+(empty||'');}).join('/');}
export function basicError(board){if(board.filter(p=>p==='K').length!==1 || board.filter(p=>p==='k').length!==1)return 'Place exactly one king of each color.';if([...board.slice(0,8),...board.slice(56)].some(p=>p.toLowerCase()==='p'))return 'Pawns cannot occupy the first or eighth rank.';return '';}
