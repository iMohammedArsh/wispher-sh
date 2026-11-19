# Play Store listing assets

Files uploaded by hand in Play Console (they are not part of the app bundle).

- `icon-512.png`: the store listing icon (512×512, full square; Play rounds the corners).
  Upload under Grow users → Store presence → Store listings → App icon.
- `icon.svg`: the source for that PNG. It is the launcher icon
  (`app/src/main/res/drawable/ic_launcher_*.xml`) cropped to the visible 72 dp centre.
  Keep it in step with those two files, then re-render:
  `qlmanage -t -s 512 -o . icon.svg && mv icon.svg.png icon-512.png`
- `feature-graphic.png`: the 1024×500 feature graphic (Grow users → Store presence → Store listings → Graphics).
  Source: `feature.svg` (draws a 1024×500 banner centred in a 1024×1024 canvas, with Fraunces and Hanken Grotesk embedded). Re-render:
  `qlmanage -t -s 1024 -o . feature.svg && sips -c 500 1024 feature.svg.png --out feature-graphic.png && rm feature.svg.png`

The app bundle itself is a build output and is not kept here:
`./gradlew bundleRelease` → `app/build/outputs/bundle/release/app-release.aab`.
