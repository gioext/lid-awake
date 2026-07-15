# Icon sources

## App icon

3案をimagegenで生成し、各案を32×32pxへ縮小比較した。顔、交差した腕、爪、丸まった寝姿が最も分離して見えた2案目を`imagegen-sloth-source.png`として採用した。

最終生成prompt:

```text
Create a square macOS app icon source illustration at 1024×1024. Center one large curled-up sleeping sloth with a clearly readable head, crossed arms and sleeping pose. Use a compact, strong silhouette made from broad flat shapes, not a photo. Use a dark graphite background and a warm brown sloth. Keep generous outer safe space and do not pre-mask or round the square canvas. No text, branch, logo, watermark, thin lines, or Apple product imagery. The sleeping animal must remain recognizable at 32px.
```

`scripts/build_icons.py`が1024×1024のsRGB RGBAへ正規化し、均一な赤い円と斜線を決定論的に合成する。

## Menu bar icon

`imagegen-tray-source.png`の生成prompt:

```text
Create a source glyph for a macOS menu bar template icon. Use a bold curled-up sleeping sloth silhouette with a readable rounded head, simple face cutout, curled body and crossed arms. Add a simple prohibition ring and an upper-left to lower-right slash. Use pure black flat shapes only on a perfectly uniform bright chroma green background. No gray, gradient, shadow, texture, fur detail, thin line, text, logo, branch, watermark, Apple product, border, or rounded-square mask. Keep safe outer margin and make the motif recognizable at 18×18 points.
```

`scripts/build_icons.py`が緑背景をalphaへ変換し、全可視pixelのRGBを黒へ固定して36×36pxへ縮小する。
