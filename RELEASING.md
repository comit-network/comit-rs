# Releasing

Releases of comit-rs components are mostly automated based on the GitFlow branching model.

## Necessary GH action secrets

For the release workflows to work, the repository needs to expose the following "secrets":

- `BOTTY_GITHUB_TOKEN`: A personal access token of our GitHub bot user [
COMIT Botty McBotface](https://github.com/comit-botty-mc-botface)
- `DOCKER_REGISTRY_USERNAME`: The username to use for logging into DockerHub
- `DOCKER_REGISTRY_PASSWORD`: The password of said user

The reason we are using a dedicated bot user is because [GitHub doesn't allow recursive workflows](https://docs.github.com/en/actions/reference/events-that-trigger-workflows#triggering-new-workflows-using-a-personal-access-token) by default.
As a result, the release branch created by GH actions would for example not trigger the CI build.

## Releasing cnd

Trigger [this](../../actions?query=workflow%3A%22Draft+new+release+of+cnd%22) workflow with the version you want to release.
The workflow will create a release branch and tag you in a PR for merging this branch into `master`.
Once you are ready to do the release, simply merge the PR.
This will trigger further workflows and eventually:

- Create a GH release
- Build a release binary of cnd for Linux, MacOS and Windows and attach them to the release
- Build a docker image and publish it on DockerHub

### Technical documentation

In total, the automation is composed of three GitHub action workflows:

1. [Draft new release](./.github/workflows/draft-new-cnd-release.yml)
2. [Create GitHub release](./.github/workflows/create-cnd-gh-release.yml)
3. [Build the release binary and attach it to the GH release](./.github/workflows/release-cnd.yml)

The following diagram illustrates how these work together: ![Cnd release sequence diagram](http://www.plantuml.com/plantuml/proxy?cache=no&src=https://raw.githubusercontent.com/comit-network/comit-rs/857a666cf16b94a00663ee8649bf67b3c1646028/docs/cnd-release.puml)

We split the release into different workflows for several reasons.

1. Building and attaching the binary needs to happen for several platforms whereas the actual release (and tag) only needs to be created once.
2. It allows for multiple entry-points into the release process:
Building binaries is triggered by a "release" event, which could in theory be created manually and doesn't necessarily have to be created through the release automation.

## Releasing nectar

Trigger [this](../../actions?query=workflow%3A%22Draft+new+release+of+nectar%22) workflow with the version you want to release.

Releasing nectar is identical to cnd except that we don't publish a docker image for nectar.

Even though the release workflow is very similar to cnd, we chose to duplicate it and make it specific to nectar.
While it would probably be possible to make one workflow that can handle both, it would make it more complicated.
Given that it is really hard to test these workflows (basically have to be tested manually), we choose to avoid as much complexity as possible.
