# Create a release

  1. Run unit tests: `cargo test`
  2. Update version in `Cargo.toml`
  3. Update `README.md` if needed.
  4. Commit current version: `git commit -m 'chore: prepare release vX.Y.Z'`
  5. Tag version: `git tag vX.Y.Z -m 'tag release vX.Y.Z' -s`
  6. Push release.
    - `git push`
    - `git push --tags`
