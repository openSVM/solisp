# Solisp Publishing Checklist

Quick reference for publishing new versions of the Solisp crate.

## Pre-Release Checklist

### Code Quality
- [ ] All tests pass: `cargo test`
- [ ] Formatting clean: `cargo fmt --all -- --check`
- [ ] Clippy clean: `cargo clippy --all-targets --all-features`
- [ ] Examples work: Test all `.solisp` scripts
- [ ] Documentation builds: `cargo doc --no-deps`
- [ ] Package builds: `cargo package --allow-dirty`

### Documentation
- [ ] README.md updated
- [ ] CHANGELOG.md updated with new version
- [ ] Version number incremented in Cargo.toml
- [ ] Usage guides reflect new features
- [ ] API docs complete and accurate

### Files
- [ ] No sensitive data in files
- [ ] License file present
- [ ] All necessary files included
- [ ] Unnecessary files excluded

## Release Steps

### 1. Update Version

```bash
# Edit Cargo.toml
vim crates/solisp/Cargo.toml
# Change: version = "1.0.1"
```

### 2. Update Changelog

```bash
# Edit CHANGELOG.md
vim crates/solisp/CHANGELOG.md
# Add section for new version
```

### 3. Test Package

```bash
cd crates/solisp
cargo package --allow-dirty
cargo package --allow-dirty --list | less
```

### 4. Commit Changes

```bash
git add crates/solisp/Cargo.toml crates/solisp/CHANGELOG.md
git commit -m "chore(solisp): bump version to X.Y.Z"
git push origin main
```

### 5. Create Tag

```bash
git tag solisp-vX.Y.Z -m "Solisp vX.Y.Z"
git push origin solisp-vX.Y.Z
```

### 6. Monitor Workflow

- Go to: https://github.com/opensvm/solisp/actions
- Watch: "Publish Solisp Crate" workflow
- Verify: All jobs pass (green checkmarks)

### 7. Verify Publication

- [ ] Check crates.io: https://crates.io/crates/solisp
- [ ] Check docs.rs: https://docs.rs/solisp
- [ ] Check GitHub Release: https://github.com/opensvm/solisp/releases
- [ ] Test install: `cargo install solisp --version X.Y.Z`

## Post-Release Checklist

### Immediate
- [ ] Verify package on crates.io
- [ ] Verify docs on docs.rs
- [ ] Test installation works
- [ ] GitHub Release created with notes

### Within 24 Hours
- [ ] Update main repository README if needed
- [ ] Announce on social media / community channels
- [ ] Monitor for issues or bug reports
- [ ] Respond to questions

### Within 1 Week
- [ ] Review download statistics
- [ ] Address any reported issues
- [ ] Plan next release if needed

## Quick Commands

```bash
# Current version
grep "^version" crates/solisp/Cargo.toml

# Test everything
cd crates/solisp && cargo test && cargo run --example run_file examples/hello_world.solisp

# Package dry run
cargo package --allow-dirty --list

# Create and push tag
git tag solisp-v1.0.1 -m "Solisp v1.0.1" && git push origin solisp-v1.0.1

# Verify on crates.io (after publish)
cargo search solisp

# Install from crates.io (after publish)
cargo install solisp
```

## Version Numbering Guide

| Change Type | Example | When to Use |
|-------------|---------|-------------|
| MAJOR | 1.0.0 → 2.0.0 | Breaking API changes |
| MINOR | 1.0.0 → 1.1.0 | New features, backward compatible |
| PATCH | 1.0.0 → 1.0.1 | Bug fixes, backward compatible |

## Emergency Rollback

If you need to yank a broken release:

```bash
# Yank version from crates.io
cargo yank --vers X.Y.Z

# Create hotfix
# 1. Fix the issue
# 2. Increment patch version
# 3. Follow normal release process

# Unyank if needed
cargo yank --vers X.Y.Z --undo
```

## Common Issues

| Issue | Solution |
|-------|----------|
| Version already exists | Increment version number |
| Tests fail in CI | Check logs, fix tests, re-tag |
| Auth failed | Verify CARGO_REGISTRY_TOKEN secret |
| Tag exists | Delete and recreate with new name |
| Package too large | Add more items to exclude list |

## Files to Update

```
crates/solisp/
├── Cargo.toml          # Version number
├── CHANGELOG.md        # Release notes
├── README.md           # If needed
├── USAGE_GUIDE.md      # If features added
└── HOW_TO_USE.md       # If workflow changed
```

## Automation

The GitHub Actions workflow automatically:
- ✅ Runs all tests
- ✅ Checks formatting and linting
- ✅ Tests all examples
- ✅ Publishes to crates.io
- ✅ Creates GitHub Release
- ✅ Deploys documentation
- ✅ Notifies on success

## Manual Override

To publish manually (not recommended):

```bash
cd crates/solisp
export CARGO_REGISTRY_TOKEN="your-token"
cargo publish
```

## Support

- Documentation: See PUBLISHING.md
- Issues: https://github.com/opensvm/solisp/issues
- Actions: https://github.com/opensvm/solisp/actions

---

**Remember:** Once published, versions cannot be changed. Always test thoroughly!
