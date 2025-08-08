Name: artemis
Release: 1
Summary: A cross platform forensic parser
License: MIT
Version: 0.16.0
Group: Application/Security
URL: https://puffycid.github.io/artemis-api
BugURL: https://github.com/puffyCid/artemis
Requires: glibc >= 2.17

%description
Provides a command line digital forensic and incident 
response (DFIR) tool that collects forensic data from systems. 
Artemis can be used to investigation suspicious or malicious
activity on a system.

%changelog


%install
mkdir -p $RPM_BUILD_ROOT%{_bindir}
mkdir -p $RPM_BUILD_ROOT%{_docdir}/%{name}-%{version}
mkdir -p $RPM_BUILD_ROOT%{_mandir}/man1

mv %{_sourcedir}/%{name} $RPM_BUILD_ROOT%{_bindir}
mv %{_sourcedir}/README.md $RPM_BUILD_ROOT%{_docdir}/%{name}-%{version}/
mv %{_sourcedir}/LICENSE $RPM_BUILD_ROOT%{_docdir}/%{name}-%{version}/
mv %{_sourcedir}/artemis.man $RPM_BUILD_ROOT%{_mandir}/man1/artemis.1

%files
%{_bindir}/artemis
%doc %{name}-%{version}/README.md
%doc %{name}-%{version}/LICENSE
%{_mandir}/man1/%{name}.1.gz
