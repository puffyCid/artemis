# Package files

The files listed here are used to package artemis. All commands assume you are in root directory of artemis git repo.

- artemis.spec - Generates a RPM file
- artemis.control - Generates a DEB file
- artemis.man - Simple manpage for artemis
- artemis.wixproj - MSI Template project
- artemis.wxs - MSI configuration

## RPM
1. Ensure just, rpmbuild, rpmlint are installed
2. Run `just rpm`
3. Navigate to ~/rpmbuild/RPMS/
4. Run rpmsign --define "_gpg_name YOUR_KEY" --addsign artemis*
5. Validate with rpmlint artemis*.rpm

You can validate the signed rpm by importing the public key and running rpm -K artemis*.rpm

## DEB
1. Ensure just, dpkg-build, lintian, debsigs are installed
2. Run `just deb VERSION`
3. Run debsigs --sign=origin --default-key=YOUR_KEY artemis*.deb
4. Validate with lintian artemis*.deb

You can validate the signed deb by importing the public key and configuring [dpkg](https://stackoverflow.com/questions/78421733/how-do-you-sign-and-verify-a-deb-file-using-debsigs-and-debsig-verify)

## macOS PKG Installer (Requires macOS)
0. Create paid Apple Dev account ($99 per year)
1. Create [cert signing request](https://developer.apple.com/help/account/certificates/create-a-certificate-signing-request) (CSR)
2. Create and upload CSR to get your certificate. Select Developer ID Application
3. Create and upload CSR to get package installer certificate, Select Developer ID Installer
4. Import the apple dev cert (ex: Developer ID - G2) See [reference](https://blog.verslu.is/app-publishing/unable-to-build-chain-for-self-signed-root/) and [reference](https://www.apple.com/certificateauthority/).
5. Download your certs and import to keychain
6. Execute xcrun notarytool store-credentials "CUSTOM PROFILE NAME" --apple-id "APPLE_ID"
7. Sign binary: codesign --timestamp -s "TEAM ID" --deep -v -f -o runtime "PATH TO ARTEMIS BINARY"
8. Create directory and move signed artemis binary to it
9. Create a pkg file with pkgbuild --timestamp --sign "TEAM ID" --root "PATH TO DIRECTORY" --install-location /usr/local/bin --identifier io.github.puffycid.artemis --version 0.XX.0 Artemis.pkg
11. xcrun notarytool submit Artemis.pkg --keychain-profile "CUSTOM PROFILE NAME" --wait
12. xcrun stapler staple "Artemis.pkg"

Once you have steps 0-6 complete you may run: `just pkg "TEAM ID" "VERSION" "CUSTOM PROFILE NAME"` to complete steps 6-12.  

You can validate the PKG is signed and notarized with:
- spctl --assess -vv --type install Artemis-*.pkg
- [WhatsYourSign](https://objective-see.org/products/whatsyoursign.html)

References for signing and notarizing cli/Rust tools:
- https://www.reddit.com/r/rust/comments/q8r90b/notarization_of_rust_binary_for_distribution_on/
- https://victoronsoftware.com/posts/script-only-macos-install-package/
- https://forum.hise.audio/topic/12146/pkg-notarisation-issue/5
- https://www.davidebarranca.com/2019/04/notarizing-installers-for-macos-catalina/
- https://github.com/GuillaumeFalourd/sign-and-notarize-gha


# MSI 
1. Install [dotnet](https://dotnet.microsoft.com/en-us/)
2. Disable telemetry `setx DOTNET_CLI_TELEMETRY_OPTOUT 1`
3. Install WiX via dotnet: `dotnet tool install --global wix`
4. Run dotnet build -c Release to build MSI