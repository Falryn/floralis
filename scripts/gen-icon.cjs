const sharp = require('sharp');
const fs = require('fs');
const path = require('path');

// Source image path - update this to your source image
const SOURCE_IMAGE = path.join(__dirname, 'icon-512.png');

async function main() {
  const iconsDir = path.join(__dirname, '..', 'src-tauri', 'icons');

  if (!fs.existsSync(SOURCE_IMAGE)) {
    console.error(`Source image not found: ${SOURCE_IMAGE}`);
    process.exit(1);
  }

  // Generate PNG files
  const sizes = [32, 128, 256, 512, 1024];
  for (const size of sizes) {
    await sharp(SOURCE_IMAGE)
      .resize(size, size, { fit: 'cover' })
      .png()
      .toFile(path.join(iconsDir, `${size}x${size}.png`));
    console.log(`Generated ${size}x${size}.png`);
  }

  await sharp(SOURCE_IMAGE)
    .resize(256, 256, { fit: 'cover' })
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
