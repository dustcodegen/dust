# Dust Agent Rules

## CI Policy

- GitHub Actions is reserved for release workflows only.
- Do not add or trigger Actions for ordinary pushes, pull requests, scheduled
  validation, or development checks.
- GitHub pull requests may show external checks such as SonarQube or SonarCloud.
  Those checks must run on the provider's infrastructure, consume no GitHub
  Actions runner time, and never be invoked by an Actions job.
- Run `./.githooks/pre-commit` locally before every commit and fix every
  failure before pushing.
- The release workflows in `.github/workflows/release.yml` and
  `.github/workflows/rust-release.yml` are the only allowed workflow files.
- Do not change this policy unless the repository owner explicitly approves
  the change.
