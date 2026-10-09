// Paint personal board markings above the pieces, independently of engine arrows.
export function paintDrawings(context, drawings = [], cell = 128) {
  const colors = { G: '#4ac474', R: '#eb5852', Y: '#f5cd48', B: '#5397ef' };
  for (const drawing of drawings) {
    const color = colors[drawing.color];
    if (!color || ![drawing.from, drawing.to].every(point => Array.isArray(point) && point.length === 2 && point.every(Number.isFinite))) continue;
    const [x1, y1] = drawing.from, [x2, y2] = drawing.to;
    context.save();
    context.strokeStyle = context.fillStyle = color;
    context.lineWidth = Math.max(3, cell * .06);
    context.lineJoin = 'round';
    context.beginPath();
    if (x1 === x2 && y1 === y2) {
      const dotted = drawing.style?.startsWith('dotted');
      if (dotted) { context.lineCap = 'round'; context.setLineDash([0, context.lineWidth * 2.8]); }
      const points = drawing.outline;
      if (points?.length >= 4 && points.every(p => p.length === 2 && p.every(Number.isFinite))) {
        context.moveTo(...points[0]);
        for (const point of points.slice(1)) context.lineTo(...point);
        context.closePath(); context.stroke();
      } else if (drawing.style?.includes('circle')) {
        context.arc(x1, y1, cell * .43, 0, Math.PI * 2); context.stroke();
      } else context.strokeRect(x1 - cell * .43, y1 - cell * .43, cell * .86, cell * .86);
      if (dotted) context.setLineDash([]);
    } else {
      const length = Math.hypot(x2 - x1, y2 - y1);
      const dx = (x2 - x1) / length, dy = (y2 - y1) / length;
      const bend = drawing.style === 'curve-left' ? .35 : drawing.style === 'curve-right' ? -.35 : 0;
      const control = [(x1+x2)/2 - dy*length*bend, (y1+y2)/2 + dx*length*bend];
      const tangentLength = Math.hypot(x2-control[0], y2-control[1]);
      const tx = (x2-control[0])/tangentLength, ty = (y2-control[1])/tangentLength;
      const tip = [x2 - tx * cell * .12, y2 - ty * cell * .12];
      const base = [tip[0] - tx * cell * .28, tip[1] - ty * cell * .28];
      if (drawing.style === 'dashed') context.setLineDash([cell*.20,cell*.13]);
      context.moveTo(x1, y1);
      if (bend) context.quadraticCurveTo(...control,...base);
      else context.lineTo(...base);
      context.stroke();
      if (drawing.style === 'dashed') context.setLineDash([]);
      context.beginPath(); context.moveTo(...tip);
      context.lineTo(base[0] - ty * cell * .14, base[1] + tx * cell * .14);
      context.lineTo(base[0] + ty * cell * .14, base[1] - tx * cell * .14);
      context.closePath(); context.fill();
    }
    context.restore();
  }
}

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
  const palette = ({
    ClassicStaunton: ['#efe0c2','#8d6744','#31241b','#e8cb9b'],
    NeonGeometric: ['#233151','#18233b','#0c1221','#67e1f3'],
    ArtDecoFaceted: ['#d5dac5','#375b58','#162b2b','#e8c66f'],
  })[options.pieceSet] || ['#cdd6c1','#4c745c','#1c2c25','#d3ad62'];
  context.fillStyle = palette[2]; context.fillRect(0, 0, 1024, 1024);
  for (let row = 0; row < 8; row++) for (let col = 0; col < 8; col++) {
    context.fillStyle = (row + col) % 2 ? palette[1] : palette[0];
    context.fillRect(margin + col * cell, margin + row * cell, cell, cell);
  }
  const sets = { Cburnett: 'cburnett', Merida: 'merida', RoyalRascals: 'royal-rascals', UndeadCourt: 'undead-court', ClassicStaunton: 'classic-staunton', NeonGeometric: 'neon-geometric', ArtDecoFaceted: 'art-deco-faceted' };
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
        context.fillStyle = palette[3]; context.textAlign = 'center'; context.textBaseline = 'middle';
        context.fillText(file,margin+(i+.5)*cell,18); context.fillText(file,margin+(i+.5)*cell,1006);
        context.fillText(rank,18,margin+(i+.5)*cell); context.fillText(rank,1006,margin+(i+.5)*cell);
      } else {
        context.textAlign = 'left'; context.textBaseline = 'top'; context.fillStyle = i%2 ? palette[0] : palette[1];
        context.fillText(rank,5,i*cell+4);
        context.textAlign = 'right'; context.textBaseline = 'bottom'; context.fillStyle = i%2 ? palette[1] : palette[0];
        context.fillText(file,(i+1)*cell-5,1020);
      }
    }
  }
  const center = square => {
    if (typeof square !== 'string' || !/^[a-h][1-8]$/.test(square)) return null;
    const file = square.charCodeAt(0) - 97, rank = Number(square[1]) - 1;
    return [margin + ((options.flipped ? 7 - file : file) + .5) * cell,
      margin + ((options.flipped ? rank : 7 - rank) + .5) * cell];
  };
  const drawings = (options.boardMarks || []).map(mark => ({ color: mark.color, style: mark.style, from: center(mark.from), to: center(mark.to) }));
  if (drawings.length) paintDrawings(context, drawings, cell);
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
    const count = JSON.parse(settings).boardMarks?.length || 0;
    showStatus(count ? `Annotated board copied with ${count} drawing${count === 1 ? '' : 's'}` : 'Board image copied to clipboard');
  } catch (error) { showStatus(`Could not copy board: ${error.message}`); }
};

let pendingPngCopy;
window.ironwoodBeginBoardPngCopy = () => {
  if (!navigator.clipboard?.write || typeof ClipboardItem === 'undefined') {
    showStatus('Image clipboard is unavailable in this browser');
    return false;
  }
  if (pendingPngCopy) {
    showStatus('A board image is already being copied');
    return false;
  }
  const details = { drawings: 0 };
  const image = new Promise((resolve, reject) => {
    pendingPngCopy = { resolve, reject, details };
  });
  // Start the clipboard write during the click, before 3D rendering takes time.
  try {
    navigator.clipboard.write([new ClipboardItem({ 'image/png': image })])
      .then(() => showStatus(details.drawings ? `Annotated 3D board copied with ${details.drawings} drawing${details.drawings === 1 ? '' : 's'}` : '3D board image copied to clipboard'))
      .catch(error => showStatus(`Could not copy board: ${error.message}`));
    return true;
  } catch (error) {
    pendingPngCopy = undefined;
    showStatus(`Could not copy board: ${error.message}`);
    return false;
  }
};
window.ironwoodFinishBoardPngCopy = async (png, coordinates) => {
  const copy = pendingPngCopy;
  pendingPngCopy = undefined;
  if (!copy) return;
  const image = new Blob([png], { type: 'image/png' });
  if (!coordinates) {
    copy.resolve(image);
    return;
  }
  try {
    const { fontSize = 16, labels = [], drawings = [], cell = 128 } = JSON.parse(coordinates);
    copy.details.drawings = drawings.length;
    const bitmap = await createImageBitmap(image);
    const canvas = document.createElement('canvas');
    canvas.width = bitmap.width;
    canvas.height = bitmap.height;
    const context = canvas.getContext('2d');
    if (!context) throw new Error('Image rendering is unavailable');
    context.drawImage(bitmap, 0, 0);
    bitmap.close();
    context.font = `${fontSize}px system-ui, sans-serif`;
    context.textAlign = 'center';
    context.textBaseline = 'middle';
    for (const { text, x, y } of labels) {
      context.fillStyle = '#141617';
      context.fillText(text, x + 1, y + 1);
      context.fillStyle = '#dcbd80';
      context.fillText(text, x, y);
    }
    if (drawings.length) paintDrawings(context, drawings, cell);
    canvas.toBlob(blob => blob
      ? copy.resolve(blob)
      : copy.reject(new Error('Could not create the board image')), 'image/png');
  } catch (error) {
    copy.reject(error);
  }
};
window.ironwoodFailBoardPngCopy = message => {
  const copy = pendingPngCopy;
  pendingPngCopy = undefined;
  copy?.reject(new Error(message));
};
