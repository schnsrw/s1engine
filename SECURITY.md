# Security Policy

## Supported Versions

| Version | Supported |
|---|---|
| 1.x | Yes |
| < 1.0 | No |

## Reporting a Vulnerability

If you discover a security vulnerability in s1engine, please report it responsibly.

**Do not open a public GitHub issue for security vulnerabilities.**

Instead, please email security concerns to the maintainers or use GitHub's private vulnerability reporting feature.

### What to Include

- Description of the vulnerability
- Steps to reproduce
- Affected versions
- Potential impact

### Response Timeline

- **Acknowledgment**: Within 48 hours
- **Assessment**: Within 1 week
- **Fix**: Depends on severity (critical: ASAP, high: 1-2 weeks, medium/low: next release)

## Security Measures

s1engine implements the following security measures:

### Input Validation

- **ZIP bomb protection**: 256 MB text entry limit, 64 MB media entry limit for DOCX/ODT
- **Image dimension cap**: 16,384 px maximum width/height
- **XML parsing limits**: Configured via quick-xml defaults (no entity expansion attacks)
- **Encoding detection**: Safe fallback chain (UTF-8 -> UTF-16 BOM -> Latin-1)

### Code Safety

- No `.unwrap()` or `.expect()` in library code — all public APIs return `Result`
- No `unsafe` code (with rare documented exceptions)
- No network access — the library is purely offline
- No file system access in core crates — I/O is handled at the boundary
- All dependencies are pure Rust (no C/C++ FFI in the dependency tree)

### Dependency Auditing

Run `cargo audit` to check for known vulnerabilities in dependencies:

```bash
cargo install cargo-audit
cargo audit
```
