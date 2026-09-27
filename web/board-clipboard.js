// Render a selected position independently of the visible board and menus.
export async function boardImage(fen, options = {}) {
  const ranks = fen.split(' ')[0].split('/');
  const pieces = [];
  if (ranks.length !== 8) throw new Error('Invalid board position');
  for (const [rank, text] of ranks.entries()) {
    let file = 0;
    for (const piece of text) {
      if (/^[1-8]$/.test(piece)) file += Number(piece);
      else if (/^[prnbqkPRNBQK]$/.test(piece)) pieces.push({ piece, file: file++, rank });
      else throw new Error('Invalid board position');
    }
    if (file !== 8) throw new Error('Invalid board position');
  }
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = 1024;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('Image rendering is unavailable');
  const margin = options.frame ? 36 : 0;
  const cell = (1024 - margin * 2) / 8;
  context.fillStyle = '#1c2c25'; context.fillRect(0, 0, 1024, 1024);
  for (let row = 0; row < 8; row++) for (let col = 0; col < 8; col++) {
    context.fillStyle = (row + col) % 2 ? '#4c745c' : '#cdd6c1';
    context.fillRect(margin + col * cell, margin + row * cell, cell, cell);
  }
  const sets = { Cburnett: 'cburnett', Merida: 'merida', RoyalRascals: 'royal-rascals', UndeadCourt: 'undead-court' };
  const set = sets[options.pieceSet] || 'merida';
  const assets = new Map();
  if (options.pieceSet !== 'System') {
    await Promise.all([...new Set(pieces.map(item => item.piece))].map(piece => new Promise((resolve, reject) => {
      const image = new Image();
      image.onload = () => { assets.set(piece, image); resolve(); };
      image.onerror = () => reject(new Error('Could not load chess-piece artwork'));
      image.src = `/pieces/${set}/${piece === piece.toUpperCase() ? 'w' : 'b'}${piece.toUpperCase()}.svg`;
    })));
  }
  const symbols = {K:'♔',Q:'♕',R:'♖',B:'♗',N:'♘',P:'♙',k:'♚',q:'♛',r:'♜',b:'♝',n:'♞',p:'♟'};
  for (const {piece,file,rank} of pieces) {
    const col = options.flipped ? 7 - file : file;
    const row = options.flipped ? 7 - rank : rank;
    const x = margin + col * cell, y = margin + row * cell;
    context.save();
    if (options.shadows) { context.shadowColor = '#0007'; context.shadowBlur = cell * .04; context.shadowOffsetX = cell * .025; context.shadowOffsetY = cell * .04; }
    if (options.pieceSet === 'System') {
      context.font = `${cell * .82}px "Segoe UI Symbol", serif`;
      context.textAlign = 'center'; context.textBaseline = 'middle';
      context.fillStyle = piece === piece.toUpperCase() ? '#fff5d8' : '#171b20';
      context.fillText(symbols[piece],x+cell/2,y+cell/2);
    } else context.drawImage(assets.get(piece),x+cell*.06,y+cell*.06,cell*.88,cell*.88);
    context.restore();
  }
  if (options.coordinates) {
    context.font = `${options.frame ? 24 : 19}px system-ui, sans-serif`;
    for (let i = 0; i < 8; i++) {
      const file = 'abcdefgh'[options.flipped ? 7-i : i];
      const rank = String(options.flipped ? i+1 : 8-i);
      if (options.frame) {
        context.fillStyle = '#d3ad62'; context.textAlign = 'center'; context.textBaseline = 'middle';
        context.fillText(file,margin+(i+.5)*cell,18); context.fillText(file,margin+(i+.5)*cell,1006);
        context.fillText(rank,18,margin+(i+.5)*cell); context.fillText(rank,1006,margin+(i+.5)*cell);
      } else {
        context.textAlign = 'left'; context.textBaseline = 'top'; context.fillStyle = i%2 ? '#cdd6c1' : '#4c745c';
        context.fillText(rank,5,i*cell+4);
        context.textAlign = 'right'; context.textBaseline = 'bottom'; context.fillStyle = i%2 ? '#4c745c' : '#cdd6c1';
        context.fillText(file,(i+1)*cell-5,1020);
      }
    }
  }
  return new Promise((resolve,reject) => canvas.toBlob(blob => blob ? resolve(blob) : reject(new Error('Could not create the board image')), 'image/png'));
}

function showStatus(message) {
  document.getElementById('board-copy-status')?.remove();
  const status = document.createElement('div');
  status.id = 'board-copy-status'; status.setAttribute('role','status'); status.textContent = message;
  Object.assign(status.style, {position:'fixed',bottom:'48px',left:'50%',transform:'translateX(-50%)',zIndex:'9999',padding:'14px 20px',maxWidth:'80vw',background:'#282f35',color:'#ecebe4',border:'1px solid #d3ad62',borderRadius:'6px',font:'16px system-ui'});
  document.body.append(status); setTimeout(() => status.remove(),6000);
}

window.ironwoodCopyBoard = async (fen, settings) => {
  try {
    if (!navigator.clipboard?.write || typeof ClipboardItem === 'undefined') throw new Error('Image clipboard is unavailable in this browser');
    const image = boardImage(fen, JSON.parse(settings));
    // Supply a promise so the clipboard request starts before artwork loading finishes.
    await navigator.clipboard.write([new ClipboardItem({'image/png':image})]);
    showStatus('Board image copied to clipboard');
  } catch (error) { showStatus(`Could not copy board: ${error.message}`); }
};
