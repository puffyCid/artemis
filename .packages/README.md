# Package files

The files listed here are used to package artemis.

- artemis.spec - Generates a RPM file
- artemis.control - Generates a DEB file
- artemis.man - Simple manpage for artemis

## RPM
1. Ensure just, rpmbuild, rpmlint are installed
2. Run `just rpm`
3. Navigate to ~/rpmbuild/RPMS
4. Run rpmsign --define "_gpg_name YOUR_KEY" --addsign artemis*
5. Validate with rpmlint artemis*.rpm

You can validated the sign rpm by importing the public key and running rpm -K artemis*.rpm

## DEB
1. Ensure just,dpkg-build, lintian are installed
2. Run `just deb`
3. Run debsigs --sign=origin --default-key=YOUR_KEY artemis*.deb
4. Validate with lintian artemis*.deb

You can validate the signed deb by importing the public key and configuring [dpkg](https://stackoverflow.com/questions/78421733/how-do-you-sign-and-verify-a-deb-file-using-debsigs-and-debsig-verify)