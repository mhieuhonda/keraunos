# Security policy

## Supported versions

| Version | Supported |
| ------- | --------- |
| `main` (development) | yes — security fixes land on `main` and are tagged |
| tagged releases | latest tag only |

Keraunos is pre-release software; we still treat security reports
seriously and respond to all of them.

## Reporting a vulnerability

**Do not open a public issue for security problems.**

Report privately via GitHub's *Report a vulnerability* flow on the
[Security tab](https://github.com/mhieuhonda/keraunos/security), or, if
unavailable to you, by starting a private discussion with a maintainer on
[GitHub Discussions](https://github.com/mhieuhonda/keraunos/discussions).

Please include:

* description of the issue and its impact;
* reproduction steps (boot log, QEMU invocation, patch or PoC if any);
* affected components (`kernel/src/…` paths);

You will receive an acknowledgement within 7 days. Fixes follow
responsible disclosure: patched on `main`, credited in the release notes
(unless you prefer to remain anonymous).

## Scope

In scope: the kernel (`kernel/`), repository tooling (`tools/`), and CI.
The `UI/` directory is a verbatim vendored snapshot of upstream GNOME /
Yaru codebases — vulnerabilities in that code belong to those upstream
projects' security processes, not here.
