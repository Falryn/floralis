const sharp = require('sharp');
const fs = require('fs');
const path = require('path');

const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#fce4f0"/>
      <stop offset="50%" stop-color="#e8e0f7"/>
      <stop offset="100%" stop-color="#dce8f8"/>
    </linearGradient>
    <linearGradient id="petal" x1="0" y1="0" x2="0.3" y2="1">
      <stop offset="0%" stop-color="#ffb7d5"/>
      <stop offset="100%" stop-color="#ff85c0"/>
    </linearGradient>
    <linearGradient id="petal2" x1="0" y1="0" x2="0.3" y2="1">
      <stop offset="0%" stop-color="#ffc8dd"/>
      <stop offset="100%" stop-color="#ffadd2"/>
    </linearGradient>
    <radialGradient id="center" cx="0.5" cy="0.4" r="0.5">
      <stop offset="0%" stop-color="#fff5e0"/>
      <stop offset="100%" stop-color="#ffe0b2"/>
    </radialGradient>
    <filter id="shadow" x="-10%" y="-10%" width="120%" height="120%">
      <feDropShadow dx="0" dy="2" stdDeviation="4" flood-color="#c77dba" flood-opacity="0.3"/>
    </filter>
  </defs>
  <rect width="512" height="512" rx="96" fill="url(#bg)"/>
  <g fill="#c7a8ff" opacity="0.5">
    <circle cx="90" cy="100" r="4"/><circle cx="420" cy="80" r="3"/>
    <circle cx="70" cy="400" r="3.5"/><circle cx="440" cy="420" r="4"/>
    <circle cx="130" cy="440" r="2.5"/><circle cx="400" cy="130" r="2.5"/>
  </g>
  <g fill="#ffb7d5" opacity="0.4">
    <circle cx="110" cy="160" r="3"/><circle cx="400" cy="370" r="3"/>
    <circle cx="150" cy="380" r="2"/><circle cx="370" cy="110" r="2"/>
  </g>
  <g filter="url(#shadow)">
    <g transform="translate(256,240)">
      <ellipse cx="0" cy="-80" rx="48" ry="72" fill="url(#petal)" transform="rotate(0)"/>
      <ellipse cx="0" cy="-80" rx="48" ry="72" fill="url(#petal2)" transform="rotate(72)"/>
      <ellipse cx="0" cy="-80" rx="48" ry="72" fill="url(#petal)" transform="rotate(144)"/>
      <ellipse cx="0" cy="-80" rx="48" ry="72" fill="url(#petal2)" transform="rotate(216)"/>
      <ellipse cx="0" cy="-80" rx="48" ry="72" fill="url(#petal)" transform="rotate(288)"/>
    </g>
    <circle cx="256" cy="240" r="44" fill="url(#center)"/>
    <ellipse cx="240" cy="234" rx="6" ry="7.5" fill="#4a3060"/>
    <ellipse cx="272" cy="234" rx="6" ry="7.5" fill="#4a3060"/>
    <circle cx="243" cy="231" r="2.5" fill="white"/>
    <circle cx="275" cy="231" r="2.5" fill="white"/>
    <circle cx="238" cy="236" r="1.2" fill="white" opacity="0.7"/>
    <circle cx="270" cy="236" r="1.2" fill="white" opacity="0.7"/>
    <path d="M248 248 Q256 256 264 248" stroke="#4a3060" stroke-width="2.5" fill="none" stroke-linecap="round"/>
    <ellipse cx="230" cy="248" rx="10" ry="6" fill="#ff9999" opacity="0.35"/>
    <ellipse cx="282" cy="248" rx="10" ry="6" fill="#ff9999" opacity="0.35"/>
  </g>
  <g transform="translate(340, 140) scale(0.7)" fill="#ff85c0" opacity="0.7">
    <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/>
  </g>
  <g transform="translate(120, 340) scale(0.5)" fill="#c7a8ff" opacity="0.5">
    <path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z"/>
  </g>
</svg>`;

async function main() {
  const iconsDir = path.join(__dirname, '..', 'src-tauri', 'icons');
  
  // Generate PNG files
  const sizes = [32, 128, 256, 512, 1024];
  for (const size of sizes) {
    await sharp(Buffer.from(svg))
      .resize(size, size)
      .png()
      .toFile(path.join(iconsDir, `${size}x${size}.png`));
    console.log(`Generated ${size}x${size}.png`);
  }
  
  await sharp(Buffer.from(svg))
    .resize(256, 256)
    .png()
    .toFile(path.join(iconsDir, '128x128@2x.png'));
  console.log('Generated 128x128@2x.png');
  
  // Use dynamic import for ESM module
  const pngToIco = (await import('png-to-ico')).default;
  const icoBuf = await pngToIco(path.join(iconsDir, '256x256.png'));
  fs.writeFileSync(path.join(iconsDir, 'icon.ico'), icoBuf);
  console.log('Generated icon.ico');
  
  console.log('Done!');
}

main().catch(console.error);
