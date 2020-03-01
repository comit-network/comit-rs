# Releasing

Releases of comit-rs are mostly automated.
Here is what you need to know:

The repository uses the GitFlow branching model.
Hence, releases are started by branching a release-branch (`release/x.y.z`) off `dev`.

1. Start with creating said release branch locally.
1. Update the changelog to reflect the newest release.
1. Bump necessary versions in the manifest files.
1. Build to ensure the lock files are updated.
1. Commit and push your changes
1. Create a pull request targeting the __master__ branch.
1. Get approvals and merge it.

From here, [the automation](./.github/workflows/publish-new-release.yml) takes over and:

1. Builds the release artifacts for Linux and MacOS.
1. Pushes a new tag of the [comitnetwork/cnd](https://hub.docker.com/repository/docker/comitnetwork/cnd) docker image.
1. Tags the merge commit and creates a release on GitHub with the binaries attached.
1. Opens a PR to merge back to _dev_ branch: **Make sure to merge this one in a timely fashion**.
