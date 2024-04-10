#!/bin/zsh

<<notes
References:
https://blog.rampatra.com/how-to-notarize-a-dmg-or-zip-file-with-the-help-of-xcode-s-notary-tool
https://www.reddit.com/r/rust/comments/q8r90b/notarization_of_rust_binary_for_distribution_on/
https://christiantietze.de/posts/2022/07/mac-app-notarization-workflow-in-2022/

1. Sign binary: codesign -s "DEV_ID_APPLICATION" --deep -v -f -o runtime <target_dir>
2. Run installer.sh
3. Sign installer: codesign -s "DEV_ID_INSTALLER" --deep -v -f -o runtime Artemis-Installer.dmg
4. Create keychain profile: xcrun notarytool store-credentials "PROFILE" --apple-id "APPLE_ID"
5. Submit to notarization: xcrun notarytool submit Artemis-Installer.dmg --keychain-profile "PROFILE" --wait
6. Check status: xcrun notarytool history --keychain-profile "PROFILE"
7. Staple installer: xcrun stapler staple "Artemis-Installer.dmg"
8. Can verify everything with: "spctl --assess -vv --type install Artemis-Installer.dmg" or WhatsYourSign
notes

APP_NAME="Artemis"
DMG_FILE_NAME="${APP_NAME}-Installer.dmg"
VOLUME_NAME="${APP_NAME} Installer"

SOURCE="out/"

create-dmg \
  --volname "${VOLUME_NAME}" \
  --window-pos 200 120 \
  --window-size 800 400 \
  --icon-size 100 \
  --icon "${APP_NAME}.app" 200 190 \
  --hide-extension "${APP_NAME}.app" \
  --app-drop-link 600 185 \
  --skip-jenkins \
  --volicon "artemis_icon.icns" \
  "${DMG_FILE_NAME}" \
  "${SOURCE}"