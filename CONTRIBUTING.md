### Reporting new issues/bugs. {#issues}

When [opening a new issue](https://github.com/Akylas/mbview-rs/issues), always make sure to fill out the issue template. We use GitHub issues to track public bugs. Please ensure your description is clear and has sufficient instructions to be able to reproduce the issue.

-   _One issue, one bug_: Please report a single bug per issue.
-   _Provide reproduction steps_: List all the steps necessary to reproduce the issue. The person reading your bug report should be able to follow these steps to reproduce your issue with minimal effort.

### Feature Request {#feat}

We use [GitHub Discussions](https://github.com/Akylas/mbview-rs/discussions) and [GitHub Issues](https://github.com/Akylas/mbview-rs) to track ideas from users. Suggest a new feature [here](https://github.com/Akylas/mbview-rs/discussions/new)!
Great Feature Requests tend to have:

-   A quick idea summary.
-   What & why you wanted to add the specific feature.
-   Additional references like images, links of resources about the feature, etc.

## Working on App code

### Prerequisite

-   [Tauri environment](https://tauri.studio/en/docs/getting-started/intro#setting-up-your-environment)
-   [Node JS](https://nodejs.org/en/)
-   [Git](https://git-scm.com/)
-   npm or [yarn](https://yarnpkg.com/) or [pnpm](https://pnpm.io/)
-   Code Editor, we recommend you to use [VS Code](https://code.visualstudio.com/)

### Installation

1. After cloning the repository, run `yarn|npm|pnpm` in the root of the repository.
2. To start the app locally, run `yarn dev` or `pmpm dev` or `npm run dev`.

### Semantic commit messages {#commit-msg}

See how a minor change to your commit message style can make you a better programmer.

Format: `<type>(<scope>): <subject>`

`<scope>` is optional

#### Example

```
feat: allow overriding of webpack config
^--^  ^------------^
|     |
|     +-> Summary in present tense.
|
+-------> Type: chore, docs, feat, fix, refactor, style, or test.
```

the various types of commits:

-   `feat`: new feature for the user
-   `fix`: bug fix for the user
-   `docs`: changes to the documentation
-   `style`: formatting, missing semi-colons, etc.
-   `refactor`: refactoring production code, eg. renaming a variable
-   `test`: adding missing tests, refactoring tests.
-   `chore`: updating grunt tasks etc

Use lower case not the upper case!

## Pull requests

### Your first pull request. {#first-pull-request}

So you have decided to contribute code back to upstream by opening a pull request. You've invested a good chunk of time, and we appreciate it. We will do our best to work with you and get the PR looked at.

### Proposing a change

If you would like to request a new feature or enhancement but are not yet thinking about opening a pull request, you can also [open a discussion](#feat) and others will code it!

If you intend to fix a bug, please discuss it through [Issues](#issues) before submitting a pull request.

If you intend to add a new feature, please discuss it through [GitHub Discussions](#feat) to avoid multiple people working on the same feature request.

### Sending a Pull Request

make sure the PR does only one thing, otherwise please split it. It is recommended to follow this [commit message style](#commit-msg).

1. Fork [the repository](https://github.com/Akylas/mbview-rs) and create your branch from `master`.
2. Make changes and ensure your commit message is understandable.
3. Open a [PR](https://github.com/Akylas/mbview-rs/pulls) and ensure to describe your pull request clearly.