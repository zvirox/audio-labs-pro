# RPM spec for Audio Labs Pro.
#
# Built by .github/workflows/release.yml: the workflow produces a release
# build, installs it into a staged /usr tree, packs that tree into a Source0
# tarball and stamps the version from the git tag (dashes become `~`, e.g.
# v0.0.1-alpha.1 -> 0.0.1~alpha.1, which RPM orders before a final release).
Name:           audio-labs-pro
Version:        %{alp_version}
Release:        1%{?dist}
Summary:        Professional Voice Audio Workstation

License:        Apache-2.0
URL:            https://github.com/zvirox/audio-labs-pro
Source0:        audio-labs-pro-bin.tar.gz
BuildArch:      x86_64

# The staged binaries are built in CI against glibc/ALSA; runtime libraries
# resolve through the standard RPM file dependencies, so only the audio stack
# is pinned here.
Requires:       alsa-lib
Recommends:     pipewire-alsa

%description
Audio Labs Pro is a modern, responsive voice audio workstation built in Rust.
It combines non-destructive multitrack audio editing with local speech
synthesis, voice cloning, automated transcription, speech cleanup DSP, and an
undoable session engine.

The package ships the graphical workstation (audio-labs-pro), the CLI
(alp-cli) and the local inference runner (alp-infer).

%prep
# The Source0 tarball already contains a relative usr/ tree.
%setup -q -c

%install
mkdir -p %{buildroot}/usr
cp -r usr/. %{buildroot}/usr/

%files
/usr/bin/audio-labs-pro
/usr/bin/alp-cli
/usr/bin/alp-infer
%dir /usr/share/applications
/usr/share/applications/io.github.zvirox.AudioLabsPro.desktop
%dir /usr/share/metainfo
/usr/share/metainfo/io.github.zvirox.AudioLabsPro.metainfo.xml
/usr/share/icons/hicolor/scalable/apps/io.github.zvirox.AudioLabsPro.svg
/usr/share/icons/hicolor/48x48/apps/io.github.zvirox.AudioLabsPro.png
/usr/share/icons/hicolor/64x64/apps/io.github.zvirox.AudioLabsPro.png
/usr/share/icons/hicolor/128x128/apps/io.github.zvirox.AudioLabsPro.png
/usr/share/icons/hicolor/48x48@2/apps/io.github.zvirox.AudioLabsPro.png
/usr/share/icons/hicolor/64x64@2/apps/io.github.zvirox.AudioLabsPro.png
/usr/share/icons/hicolor/128x128@2/apps/io.github.zvirox.AudioLabsPro.png
/usr/share/mime/packages/io.github.zvirox.AudioLabsPro.xml

%changelog
