Name: device
Version: 0.1.0
Release: 1%{?dist}
Summary: Battery, Display Brightness, Keyboard Backlight and Platform Profile Daemon

License: GPL-3.0-or-later
URL: https://github.com/whynotofu/device
Source0: %{url}/archive/v%{version}/%{name}-%{version}.tar.gz

BuildRequires: cargo
BuildRequires: rust-packaging >= 21
BuildRequires: systemd-rpm-macros

%description
Battery, Display Brightness, Keyboard Backlight and Power Profile Daemon

%prep
%autosetup
%cargo_prep -N
sed -i 's/^offline = true$//' .cargo/config.toml
sed -i 's/.*please-remove-me$//' .cargo/config.toml

%build
%cargo_build -- --package device --package device-cli

%install
install -d -m 0755 %{buildroot}%{_bindir}
install -p -m 0755 target/release/device %{buildroot}%{_bindir}/
install -p -m 0755 target/release/device-cli %{buildroot}%{_bindir}/
install -D -m 0644 systemd/device.service %{buildroot}%{_unitdir}/device.service

%post
%systemd_post device.service

%preun
%systemd_preun device.service

%postun
%systemd_postun_with_restart device.service

%files
%license LICENSE
%doc README.md
%{_bindir}/device
%{_bindir}/device-cli
%{_unitdir}/device.service

%changelog
* Fri Jul 24 2026 Steven <278405169+whynotofu@users.noreply.github.com> - 0.1.0-1
- initial release
