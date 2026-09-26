# Position editor and screenshot recognition

Open New Game > Local > Starting position: Custom FEN > Position editor / Import screenshot.
The HTML editor is independent of egui; it passes a FEN back to the New Game dialog, where Rust validates legality before starting.

Images stay on the device. Recognition runs in a dedicated worker, reuses one ONNX session and warms up when the editor opens. The model and approximately 14 MB WASM runtime load only on opening the editor. Images are limited to a 1200-pixel detection dimension. Crop to the board before importing for the most reliable scan. Manual edits, rotation and explicit game-state fields are required review tools, not proof of recognition accuracy. The editor uses Cburnett pieces for clarity.

## Build and checks

- `npm run build:editor` bundles the worker and copies the pinned installed package assets and licenses.
- `npm run prepare:wrangler` also builds the editor.
- `node --test scripts/position-editor.test.mjs` verifies FEN handling.
- `node scripts/generate-fenshot-training.mjs 200` creates labeled examples under ignored `output/fenshot-training`.
- `node scripts/check-fenshot-model.mjs` verifies real model inference after generating examples.

## Training extension

The exporter renders all four Ironwood piece sets with varied sizes, board colors and JPEG quality. It uses Fenshot's own tile extraction. It produces upstream-compatible `shard-000.bin` uint8 tiles and `shard-000.labels` in A1..H8 order (class alphabet `1KQRBNPkqrbnp`), plus sample images and a manifest. These synthetic boards include arbitrary placements to balance piece classes; they are not intended to represent legal games.

The deployed model is still the upstream model; no accuracy improvement for decorative sets is claimed. To retrain, mix this corpus with the upstream corpus (do not replace its existing themes), run upstream `tools/tile-classifier/train.py`, then evaluate on held-out real screenshots for every set and on upstream regression fixtures. On Windows use a relative CORPUS path because the upstream script splits paths at colons. Training needs Python/PyTorch and ONNX export dependencies. Do not ship a replacement until actual screenshot evaluation passes. Model selection can then be changed in the asset build and worker together.

Upstream: https://github.com/scoriiu/fenshot/tree/main/tools/tile-classifier

URL puzzle import and automated website screenshots are not implemented. Upload, paste or drop an image instead.
