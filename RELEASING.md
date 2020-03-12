# Releasing

Releases of comit-rs are mostly automated based on the GitFlow branching model.

To release a new version, create an issue with the title "Release version x.y.z" and label it with the "release" label.

From here, [the automation](./.github/workflows/draft-new-release.yml) takes over and:

1. Creates a new branch `release/x.y.z`
1. Updates the changelog to the new version
1. Bumps the version of the cnd/Cargo.toml manifest
1. Commits and pushes the changes
1. Creates a pull request for merging the release branch into master.

Merging this pull request will trigger another [workflow](./.github/workflows/publish-new-release.yml) that:

1. Builds the release artifacts for Linux and MacOS.
1. Pushes a new tag of the [comitnetwork/cnd](https://hub.docker.com/repository/docker/comitnetwork/cnd) docker image.
1. Tags the merge commit and creates a release on GitHub with the binaries attached.
1. Opens a PR to merge back to _dev_ branch: **Make sure to merge this one in a timely fashion**.
