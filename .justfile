# Small Justfile (https://github.com/casey/just and https://just.systems/man/en). 
# `just` is recommended. 
# Its useful when you want to run groups of tests and do not want to type the full test path
# Windows users will need to use PowerShell `just --shell pwsh.exe --shell-arg -c`

import ".setup/ubuntu.just"
import ".setup/fedora.just"
import ".setup/windows.just"
import ".setup/macos.just"

# Run cargo clippy on artemis project 
default:
  cargo clippy

_test target:
  cargo test --release {{target}}

_pretest:
  cargo test --no-run --release

# Test only the ESE parsing functions
[group('artifacts')]
ese: (_test "artifacts::os::windows::ese")

# Test only the WMI parsing functions
[group('artifacts')]
wmi: (_test "artifacts::os::windows::wmi")

# Test only the ShellItems parsing functions
[group('artifacts')]
shellitems: (_test "artifacts::os::windows::shellitems")

# Test only the Outlook parsing functions
[group('artifacts')]
outlook: (_test "artifacts::os::windows::outlook")

# Test only the Spotlight parsing functions
[group('artifacts')]
spotlight: (_test "artifacts::os::macos::spotlight")

# Test only the Registry parsing functions
[group('artifacts')]
registry: (_test "artifacts::os::windows::registry")

# Test only the Eventlog parsing functions
[group('artifacts')]
eventlogs: (_test "artifacts::os::windows::eventlogs")

# Test only the MFT parsing functions
[group('artifacts')]
mft: (_test "artifacts::os::windows::mft")

# Test only the JavaScript runtime
runtime: (_test "runtime::")

# Test only the FileSystem functions
filesystem: (_test "filesystem::")

# Test only the timelining functions
timeline: (_test "timeline::")

# Test all the Windows artifacts
[group('os')]
windows: (_test "artifacts::os::windows")

# Test all the macOS artifacts
[group('os')]
macos: (_test "artifacts::os::macos")

# Test all the Linux artifacts
[group('os')]
linux: (_test "artifacts::os::linux")

# Build the entire artemis project.
build:
  cargo build --release

# Run tests for code coverage. Used by CI
_coverage:
  cargo llvm-cov --release --workspace --exclude daemon --lcov --output-path lcov.info

# Build Artemis for GitHub Actions
_ci_release target:
  cargo auditable build --profile release-action --bin artemis --target {{target}}

# Build Artemis for GitHub Actions using Cross
_ci_release_cross target:
  cross build --profile release-action --bin artemis --target {{target}}

# Test the entire artemis project
test:
  cargo test --release

# Test the entire artemis project using nextest
nextest:
  cargo nextest run --release

# Just build the artemis binary
[group('workspace')]
cli:
  cd cli && cargo build --release

# Just build the artemis binary. But do not enable Yara-X
[group('workspace')]
slim:
  cd cli && cargo build --release --no-default-features

# Just build the forensics library
[group('workspace')]
forensics:
  cd forensics && cargo build --release

# Review complexity with scc
complex:
  scc -i rs --by-file -s complexity

# Setup Artemis development environment for Ubuntu
[group('setup')]
setup-ubuntu: (_ubuntu)

# Setup Artemis development environment for Fedora
[group('setup')]
setup-fedora: (_fedora)

# Setup Artemis development environment for Windows
[group('setup')]
setup-windows: (_windows)

# Setup Artemis development environment for macOS
[group('setup')]
setup-macos: (_macos)

# Package Artemis into RPM file
[group('package')]
rpm: (cli)
  @mkdir -p ~/rpmbuild/SOURCES
  @cp target/release/artemis ~/rpmbuild/SOURCES
  @cp README.md ~/rpmbuild/SOURCES
  @cp LICENSE ~/rpmbuild/SOURCES
  @cp .packages/artemis.man ~/rpmbuild/SOURCES

  rpmbuild --quiet -bb .packages/artemis.spec
  @echo ""
  @echo "RPM package built you may find it at ~/rpmbuild/RPMS"
  @echo "You can sign the package with rpmsign using your own GPG key"

# Package Artemis into RPM file for CI Releases
[group('package')]
_ci_rpm target: (_ci_release target)
  @mkdir -p ~/rpmbuild/SOURCES
  @mv "target/${TARGET}/release-action/${NAME}" ~/rpmbuild/SOURCES
  @cp README.md ~/rpmbuild/SOURCES
  @cp LICENSE ~/rpmbuild/SOURCES
  @cp .packages/artemis.man ~/rpmbuild/SOURCES

  rpmbuild --quiet -bb .packages/artemis_ci.spec
  @mv ~/rpmbuild/RPMS/x86_64/artemis* "target/${TARGET}/release-action/artemis-${VERSION}-1.${TARGET}.rpm"
  rpmsign --define "_gpg_name PuffyCid" --addsign target/${TARGET}/release-action/artemis*.rpm
  cd "target/${TARGET}/release-action" && echo -n "$(shasum -ba 256 artemis*.rpm | cut -d " " -f 1)" > artemis-${VERSION}-1.${TARGET}.rpm.sha256


# Package Artemis into DEB file
[group('package')]
deb version: (cli)
  @mkdir -p ~/artemis_{{version}}-1/DEBIAN
  @mkdir -p -m 755 ~/artemis_{{version}}-1/usr/
  @mkdir -p -m 755 ~/artemis_{{version}}-1/usr/bin
  @mkdir -p -m 755 ~/artemis_{{version}}-1/usr/share
  @mkdir -p -m 755 ~/artemis_{{version}}-1/usr/share/man
  @mkdir -p -m 755 ~/artemis_{{version}}-1/usr/share/man/man1

  @mkdir -p -m 755 ~/artemis_{{version}}-1/usr/share/doc
  @mkdir -p -m 755 ~/artemis_{{version}}-1/usr/share/doc/artemis
  @cp LICENSE ~/artemis_{{version}}-1/usr/share/doc/artemis/copyright
  @chmod 0644 ~/artemis_{{version}}-1/usr/share/doc/artemis/copyright

  @cp CHANGELOG.md ~/artemis_{{version}}-1/usr/share/doc/artemis/changelog
  @gzip -9n ~/artemis_{{version}}-1/usr/share/doc/artemis/changelog
  @chmod 0644 ~/artemis_{{version}}-1/usr/share/doc/artemis/changelog.gz

  @cp .packages/artemis.man ~/artemis_{{version}}-1/usr/share/man/man1/artemis.1
  @gzip -9n ~/artemis_{{version}}-1/usr/share/man/man1/artemis.1
  @chmod 0644 ~/artemis_{{version}}-1/usr/share/man/man1/artemis.1.gz

  @cp target/release/artemis ~/artemis_{{version}}-1/usr/bin/artemis
  @chmod 0755 ~/artemis_{{version}}-1/usr/bin/artemis

  @cd ~/artemis_{{version}}-1/ && find usr -type f -exec md5sum '{}' \; > ./DEBIAN/md5sums
  @cp .packages/artemis.control ~/artemis_{{version}}-1/DEBIAN/control

  dpkg-deb --build --root-owner-group ~/artemis_{{version}}-1
  @echo ""
  @echo "DEB package built you may find it in your home directory"
  @echo "You can sign the package with debsigs using your own GPG key"

# Package Artemis into DEB file for CI Releases
[group('package')]
_ci_deb version target: (_ci_release target)
  @mkdir -p ~/artemis_{{version}}-1/DEBIAN
  @mkdir -p -m 755 ~/artemis_{{version}}-1/usr/
  @mkdir -p -m 755 ~/artemis_{{version}}-1/usr/bin
  @mkdir -p -m 755 ~/artemis_{{version}}-1/usr/share
  @mkdir -p -m 755 ~/artemis_{{version}}-1/usr/share/man
  @mkdir -p -m 755 ~/artemis_{{version}}-1/usr/share/man/man1

  @mkdir -p -m 755 ~/artemis_{{version}}-1/usr/share/doc
  @mkdir -p -m 755 ~/artemis_{{version}}-1/usr/share/doc/artemis
  @cp LICENSE ~/artemis_{{version}}-1/usr/share/doc/artemis/copyright
  @chmod 0644 ~/artemis_{{version}}-1/usr/share/doc/artemis/copyright

  @cp CHANGELOG.md ~/artemis_{{version}}-1/usr/share/doc/artemis/changelog
  @gzip -9n ~/artemis_{{version}}-1/usr/share/doc/artemis/changelog
  @chmod 0644 ~/artemis_{{version}}-1/usr/share/doc/artemis/changelog.gz

  @cp .packages/artemis.man ~/artemis_{{version}}-1/usr/share/man/man1/artemis.1
  @gzip -9n ~/artemis_{{version}}-1/usr/share/man/man1/artemis.1
  @chmod 0644 ~/artemis_{{version}}-1/usr/share/man/man1/artemis.1.gz

  @mv "target/${TARGET}/release-action/${NAME}" ~/artemis_{{version}}-1/usr/bin
  @chmod 0755 ~/artemis_{{version}}-1/usr/bin/artemis

  @cd ~/artemis_{{version}}-1/ && find usr -type f -exec md5sum '{}' \; > ./DEBIAN/md5sums
  @cp .packages/artemis.control ~/artemis_{{version}}-1/DEBIAN/control

  dpkg-deb --build --root-owner-group ~/artemis_{{version}}-1
  @rm -r target/${TARGET}/release-action/artemis.d
  @mv ~/artemis_{{version}}-1.deb "target/${TARGET}/release-action/"
  @debsigs --sign=origin --default-key=${PUB} target/${TARGET}/release-action/artemis*.deb
  
  cd "target/${TARGET}/release-action" && echo -n "$(shasum -ba 256 artemis*.deb | cut -d " " -f 1)" > artemis_{{version}}-1.deb.sha256

# Package Artemis into macOS PKG installer file
[group('package')]
pkg team_id version profile: (cli)
  @cd target/release && codesign --timestamp -s {{team_id}} --deep -v -f -o runtime artemis
  @mkdir target/release/pkg && mv target/release/artemis target/release/pkg
  @pkgbuild --timestamp --sign {{team_id}} --root target/release/pkg --install-location /usr/local/bin --identifier io.github.puffycid.artemis --version {{version}} artemis-{{version}}.pkg
  @xcrun notarytool submit artemis-{{version}}.pkg --keychain-profile {{profile}} --wait
  @xcrun stapler staple artemis-{{version}}.pkg
  @mv artemis-{{version}}.pkg ~/

  @echo ""
  @echo "PKG installer should be in your home directory"

# Package Artemis into macOS PKG installer file for CI Releases
[group('package')]
_ci_pkg version profile target: (_ci_release target)
  @cd target/${TARGET}/release-action && codesign --keychain ${RUNNER_TEMP}/app-signing.keychain-db --timestamp -s "${TEAM_ID}" --deep -v -f -o runtime artemis
  @mkdir target/${TARGET}/release-action/pkg && mv target/${TARGET}/release-action/artemis target/${TARGET}/release-action/pkg
  @pkgbuild --keychain ${RUNNER_TEMP}/app-signing.keychain-db --timestamp --sign "${TEAM_ID}" --root target/${TARGET}/release-action/pkg --install-location /usr/local/bin --identifier io.github.puffycid.artemis --version {{version}} artemis-{{version}}.{{target}}.pkg
  @xcrun notarytool submit artemis-{{version}}.{{target}}.pkg --keychain-profile {{profile}} --keychain ${RUNNER_TEMP}/app-signing.keychain-db --wait 
  @rm -r target/${TARGET}/release-action/* && mv artemis-{{version}}.{{target}}.pkg "target/${TARGET}/release-action/"

  cd "target/${TARGET}/release-action" && echo "$(shasum -ba 256 artemis*.pkg | cut -d " " -f 1)" > artemis-{{version}}.{{target}}.pkg.sha256

# Package Artemis into Windows MSI installer file
[group('package')]
msi: (cli)
  @copy-item .\.packages\artemis.wixproj .\target\release\artemis.wixproj
  @copy-item .\.packages\artemis.wxs .\target\release\artemis.wxs
  cd target\release && dotnet build -c Release

  @echo ""
  @echo "MSI installer created in target\release\bin\Release"

# Package Artemis into Windows MSI installer file for CI Releases
[group('package')]
_ci_msi target: (_ci_release target)
  @copy-item .\.packages\artemis.wixproj .\target\{{target}}\release-action\artemis.wixproj
  @copy-item .\.packages\artemis.wxs .\target\{{target}}\release-action\artemis.wxs
  cd target\{{target}}\release-action\ && dotnet build -c Release

  @mv target\{{target}}\release-action\bin\Release\artemis.msi "target\{{target}}\"
  @Remove-Item -Path target\{{target}}\release-action\* -Recurse && mv target\{{target}}\artemis.msi target\{{target}}\release-action\artemis-{{target}}.msi
  cd "target\{{target}}\release-action" && (Get-FileHash artemis-{{target}}.msi -Algorithm SHA256).Hash | Out-File -Encoding ASCII -NoNewline artemis-{{target}}.msi.sha256

# Start the example daemon server in a Podman container
[group('daemon')]
server-podman:
  @cd "tools/daemon server" && npm run build
  podman network create daemonnet --ignore
  podman build -f daemon/Dockerfile -t daemon-server --target server ./
  podman run -p 127.0.0.1:8000:8000 --network daemonnet --name daemonserver --replace -d localhost/daemon-server
  @echo "Server should be running. You can use Insomnia client to connect."
  @echo "You can use podman logs to view any console output"

# Start the example artemis daemon in a Podman container and connect to server Podman container
[group('daemon')]
daemon-podman: (server-podman)
  podman build -f daemon/Dockerfile -t daemon-endpoint --target daemon ./
  podman run -d --network daemonnet localhost/daemon-endpoint

# Stop ALL Podman containers and remove and prune ALL container data
[group('daemon')]
cleanup-podman:
  podman stop --all -t 2
  podman system prune --all --force && podman rmi --all

# Spawn three daemon containers to connect to server.
[group('daemon')]
daemon-preview: (server-podman)
  podman build -f daemon/Dockerfile -t daemon-endpoint --target daemon ./
  podman run -d --network daemonnet localhost/daemon-endpoint
  podman run -d --network daemonnet localhost/daemon-endpoint
  podman run -d --network daemonnet localhost/daemon-endpoint

