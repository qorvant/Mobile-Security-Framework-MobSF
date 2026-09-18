// Generates a 256x256 32-bit ICO for the Windows executable.
// Run with: node scripts/gen_icon.mjs
import { writeFileSync, mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const SIZE = 256;
const W = SIZE, H = SIZE;

const header = Buffer.alloc(6);
header.writeUInt16LE(0, 0); // reserved
header.writeUInt16LE(1, 2); // type = icon
header.writeUInt16LE(1, 4); // count

const dirEntry = Buffer.alloc(16);
dirEntry.writeUInt8(W === 256 ? 0 : W, 0); // width (0 means 256)
dirEntry.writeUInt8(H === 256 ? 0 : H, 1); // height
dirEntry.writeUInt8(0, 2); // colors
dirEntry.writeUInt8(0, 3); // reserved
dirEntry.writeUInt16LE(1, 4); // planes
dirEntry.writeUInt16LE(32, 6); // bit count

const bmpHeader = Buffer.alloc(40);
bmpHeader.writeUInt32LE(40, 0);
bmpHeader.writeInt32LE(W, 4);
bmpHeader.writeInt32LE(H * 2, 8); // height includes XOR + AND mask
bmpHeader.writeUInt16LE(1, 12); // planes
bmpHeader.writeUInt16LE(32, 14); // bit count
bmpHeader.writeUInt32LE(0, 16); // BI_RGB
bmpHeader.writeUInt32LE(W * H * 4, 20); // image size

// XOR pixel data (BGRA, bottom-up). A simple blue->teal gradient with a shield-ish look.
const pixels = Buffer.alloc(W * H * 4);
for (let y = 0; y < H; y++) {
  for (let x = 0; x < W; x++) {
    const t = (x + y) / (W + H);
    const r = Math.round(40 + t * 60);
    const g = Math.round(120 + t * 80);
    const b = Math.round(200 + t * 40);
    // distance from center for a rounded-square mask
    const dx = (x - W / 2) / (W / 2);
    const dy = (y - H / 2) / (H / 2);
    const d = Math.sqrt(dx * dx + dy * dy);
    const alpha = d <= 1 ? 255 : 0;
    const rowFromBottom = H - 1 - y;
    const o = (rowFromBottom * W + x) * 4;
    pixels[o] = b; pixels[o + 1] = g; pixels[o + 2] = r; pixels[o + 3] = alpha;
  }
}

// AND mask (1bpp), all zero (fully opaque where alpha set).
const andMaskSize = Math.ceil((W * H) / 8);
const andMask = Buffer.alloc(andMaskSize, 0);

const imageSize = bmpHeader.length + pixels.length + andMask.length;
dirEntry.writeUInt32LE(imageSize, 8);
dirEntry.writeUInt32LE(header.length + dirEntry.length, 12); // image offset

const icon = Buffer.concat([header, dirEntry, bmpHeader, pixels, andMask]);

const outDir = join(dirname(fileURLToPath(import.meta.url)), '..', 'app', 'icons');
mkdirSync(outDir, { recursive: true });
writeFileSync(join(outDir, 'icon.ico'), icon);
console.log('wrote', join(outDir, 'icon.ico'), icon.length, 'bytes');
