import sharp from 'sharp';
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

const root = join(import.meta.dirname, '..');
const web = join(root, 'web');
const logo = join(root, 'assets/branding/ironwood-logo.jpg');
const icon = join(root, 'assets/branding/ironwood-icon.png');
await mkdir(join(web, 'brand'), { recursive: true });
for (const size of [192, 256, 384, 768]) {
  const info = await sharp(logo).resize(size, size).webp({ quality: 82 })
    .toFile(join(web, `brand/ironwood-logo-${size}.webp`));
  console.log(`Logo ${size}: ${info.size} bytes`);
}
const sizes = new Map([
  [16, 'favicon-16x16.png'], [32, 'favicon-32x32.png'], [48, 'favicon-48x48.png'],
  [150, 'mstile-150x150.png'], [180, 'apple-touch-icon.png'],
  [192, 'android-chrome-192x192.png'], [512, 'android-chrome-512x512.png'],
]);
const images = new Map();
for (const [size, name] of sizes) {
  const png = await sharp(icon).resize(size, size).png({ palette: true, colours: 64 }).toBuffer();
  images.set(size, png);
  await writeFile(join(web, name), png);
}
// Separate maskable icon keeps the complete mark inside the central safe circle.
await sharp(icon).resize(410, 410).extend({ top: 51, bottom: 51, left: 51, right: 51,
  background: '#042313' }).png({ palette: true, colours: 64 })
  .toFile(join(web, 'android-chrome-maskable-512x512.png'));

const entries = [16, 32, 48];
const header = Buffer.alloc(6 + entries.length * 16);
header.writeUInt16LE(1, 2);
header.writeUInt16LE(entries.length, 4);
let offset = header.length;
for (const [index, size] of entries.entries()) {
  const pos = 6 + index * 16;
  header[pos] = header[pos + 1] = size;
  header.writeUInt16LE(1, pos + 4);
  header.writeUInt16LE(32, pos + 6);
  header.writeUInt32LE(images.get(size).length, pos + 8);
  header.writeUInt32LE(offset, pos + 12);
  offset += images.get(size).length;
}
await writeFile(join(web, 'favicon.ico'), Buffer.concat([header, ...entries.map(size => images.get(size))]));
// Keep legacy direct requests for the SVG icon consistent with the new identity.
await writeFile(join(web, 'favicon.svg'), `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 192 192"><image width="192" height="192" href="data:image/png;base64,${images.get(192).toString('base64')}"/></svg>\n`);
