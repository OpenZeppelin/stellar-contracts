# Contributing to OpenZeppelin Soroban Contracts

We really appreciate and value contributions to OpenZeppelin Soroban Contracts. Please take 5' to review the items listed below to make sure that your contributions are merged as soon as possible.

For ANY of the items below, if they seem too complicated or hard, you can always ask for help from us. We love that you are helping us, and we would love to help you back!

## Start With an Issue, Not a PR

**Do not open a pull request without a corresponding issue and prior discussion** (unless the change is trivial — e.g., fixing a typo or a broken link).

Getting the design right is the most important step in the contribution process. Once we agree on the approach, conventions, and scope, the implementation itself becomes straightforward. Skipping this step often leads to PRs that are fundamentally misaligned with the library's direction, which wastes time for both the contributor and the maintainers.

**The expected workflow is:**

1. [Open an issue](https://github.com/OpenZeppelin/stellar-contracts/issues/new/choose) describing what you want to add or change, and why.
2. Discuss the design with the maintainers in the issue. We will help you understand the relevant conventions, identify edge cases, and agree on the scope.
3. Once the design is finalized and the issue is approved, start the implementation and open a PR.

PRs that arrive without prior discussion may be closed and redirected to an issue first. This is not to create bureaucracy — it's to make sure your effort results in something that can actually be merged.


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


## Code Conventions




## Use of AI Tools & Code Conventions

We welcome contributions regardless of how they are written — including with the help of AI coding assistants. That said, AI-generated code requires the same level of scrutiny as any other code, and in practice it often requires *more*.

Even capable AI models frequently produce subtle mistakes: incorrect assumptions about library conventions, unnecessary abstractions, hallucinated APIs, or code that compiles but doesn't align with the project's design. These issues are not always obvious at first glance, but they add up quickly during review.

**What we expect from AI-assisted contributions:**

-
- **You are responsible for the code you submit.** Treat AI output as a first draft, not a finished product. Review it thoroughly, understand every line, and verify that it follows the conventions of this library.
- **Run the full CI pipeline locally before opening a PR.** At a minimum, ensure that `cargo test`, `cargo clippy`, and `cargo fmt` all pass. The [workflow section above](#a-typical-workflow) has the exact commands.
- **Match the library's patterns and style.** All code submitted to this repository, whether written by hand or with the help of an AI assistant, is expected to follow the conventions documented in [`.claude/skills/code-quality.md`](.claude/skills/code-quality.md). The rules cover module and file layout, naming, storage TTL patterns, the `#[contracttrait]` / `ContractType` pattern, error / event / section ordering, the high-level vs `_no_auth` authorization split, macro selection (`only_*` vs `has_*`), testing, and more. PRs that violate these conventions may be rejected. This file is not only designed for AI, it is also human-readable, so we strongly advise you to read it if it is your first contribution to our repository.

**What happens with low-effort, unreviewed submissions:**

Our team has limited time and a tight development schedule. When a PR is clearly unreviewed AI output — full of basic mistakes, inconsistent style, or misaligned design — we cannot justify the time it takes to review it. In such cases:

- The PR will be closed without a detailed review.
- Repeated low-effort submissions from the same contributor may result in future PRs not being considered (blacklisting the author).

This is not about discouraging AI usage — it's about respecting everyone's time. A good contribution, whether written by hand or with AI assistance, should feel like it has already been reviewed by someone who understands the library.

We genuinely want to encourage external contributions, and we are always happy to help you get your PR across the finish line. If something is unclear or you are unsure about a design choice, open an issue or ask in the PR — we'd much rather help early than close a PR late.


## All set

If you have any questions, feel free to post them as an [issue](https://github.com/OpenZeppelin/stellar-contracts/issues).

Finally, if you're looking to collaborate and want to find easy tasks to start, look at the issues we marked as ["Good first issue"](https://github.com/OpenZeppelin/stellar-contracts/labels/good%20first%20issue).

Thanks for your time and code!
