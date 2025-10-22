# Contributing to OpenZeppelin Soroban Contracts

We really appreciate and value contributions to OpenZeppelin Soroban Contracts. Please take 5' to review the items listed below to make sure that your contributions are merged as soon as possible.

For ANY of the items below, if they seem too complicated or hard, you can always ask for help from us. We love that you are helping us, and we would love to help you back!

## Contribution guidelines

Before starting development, please [create an issue](https://github.com/OpenZeppelin/stellar-contracts/issues/new/choose) to open the discussion, validate that the PR is wanted, and coordinate overall implementation details.

## Creating Pull Requests (PRs)

As a contributor, you are expected to fork this repository, work on your own fork and then submit pull requests. The pull requests will be reviewed and eventually merged into the main repo. See ["Fork-a-Repo"](https://help.github.com/articles/fork-a-repo/) for how this works.

## A typical workflow

1. Make sure your fork is up to date with the main repository:

    ```sh
    cd stellar-contracts
    git remote add upstream https://github.com/OpenZeppelin/stellar-contracts.git
    git fetch upstream
    git pull --rebase upstream main
    ```

    > NOTE: The directory `stellar-contracts` represents your fork's local copy.

2. Branch out from `main` into `fix/some-bug-short-description-#123` (ex: `fix/typos-in-docs-#123`):

    (Postfixing #123 will associate your PR with the issue #123 and make everyone's life easier =D)

    ```sh
    git checkout -b fix/some-bug-short-description-#123
    ```

3. Make your changes, add your files, update documentation ([see Documentation section](#documentation)), commit, and push to your fork.

    ```sh
    git add .
    git commit "Fix some bug short description #123"
    git push origin fix/some-bug-short-description-#123
    ```

4. Run tests and linter. This can be done by running local continuous integration and make sure it passes (the tests on GitHub will be run once they are approved by us for the external PRs).

    ```bash
    # run tests
    cargo test

    # build
    cargo build --target wasm32v1-none --release

    # run linter
    cargo clippy --all-targets --all-features -- -D warnings

    # run formatter
    cargo +nightly fmt --all -- --check

    # run documentation checks
    cargo doc --all --no-deps
    ```

5. Go to [OpenZeppelin/stellar-contracts](https://github.com/OpenZeppelin/stellar-contracts) in your web browser and issue a new pull request.
    Begin the body of the PR with "Fixes #123" or "Resolves #123" to link the PR to the issue that it is resolving.
    *IMPORTANT* Read the PR template very carefully and make sure to follow all the instructions. These instructions
    refer to some very important conditions that your PR must meet in order to be accepted, such as making sure that all PR checks pass.

6. Maintainers will review your code and possibly ask for changes before your code is pulled in to the main repository. We'll check that all tests pass, review the coding style, and check for general code correctness. If everything is OK, we'll merge your pull request and your code will be part of OpenZeppelin Soroban Contracts.

    *IMPORTANT* Please pay attention to the maintainer's feedback, since it's a necessary step to keep up with the standards OpenZeppelin Contracts attains to.


## Tests

If you are introducing a new feature, please add a new test to ensure that it works as expected. Unit tests are mandatory for each new feature. If you are unsure about whether to write an integration test, you can wait for the maintainer's feedback.

## All set

If you have any questions, feel free to post them as an [issue](https://github.com/OpenZeppelin/stellar-contracts/issues).

Finally, if you're looking to collaborate and want to find easy tasks to start, look at the issues we marked as ["Good first issue"](https://github.com/OpenZeppelin/stellar-contracts/labels/good%20first%20issue).

Thanks for your time and code!
