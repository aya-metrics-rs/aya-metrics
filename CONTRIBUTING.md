# Contributing to AyaMetrics

Thank you for your interest in contributing to the project.

## Reporting issues

If you believe you've discovered a bug in aya, please check if the bug is
already known or [create an issue](https://github.com/aya-metrics-rs/aya-metrics/issues) on
github. Please also report an issue if you find documentation that you think is
confusing or could be improved.

When creating a new issue, make sure to include as many details as possible to
help us understand the problem. When reporting a bug, always specify which
version of aya you're using and which version of the linux kernel.

## Documentation

If you find an API that is not documented, unclear or missing examples, please
file an issue. If you make changes to the documentation, please read
[How To Write Documentation] and make sure your changes conform to the
format outlined in [Documenting Components].

[How To Write Documentation]: https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html
[Documenting Components]: https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html#documenting-components

## Fixing bugs and implementing new features

Make sure that your work is tracked by an issue or a (draft) pull request, this
helps us avoid duplicating work. If your work includes publicly visible changes,
make sure those are properly documented as explained in the section above.

### Running tests

Run the unit tests with `cargo test`.
Integration tests have not yet been implemented. See [Aya Integration Tests] for inspiration.
Consider using rbpf to test the BPF code.

[Aya Integration Tests]: https://github.com/aya-rs/aya/blob/main/test/README.md

### Commits

It is a recommended best practice to keep your changes as logically grouped as
possible within individual commits. If while you're developing you prefer doing
a number of commits that are "checkpoints" and don't represent a single logical
change, please squash those together before asking for a review.

#### Commit message guidelines

A good commit message should describe what changed and why.

1. The first line should:
    - Contain a short description of the change (preferably 50 characters or less,
      and no more than 72 characters)
    - Be entirely in lowercase with the exception of proper nouns, acronyms, and
      the words that refer to code, like function/variable names
    - Be prefixed with the name of the sub crate being changed

    Examples:
    - `aya-metrics: support custom number of counters`

1. Keep the second line blank.
1. Wrap all other lines at 72 columns (except for long URLs).
1. If your patch fixes an open issue, you can add a reference to it at the end
   of the log. Use the `Fixes: #` prefix and the issue number. For other
   references use `Refs: #`. `Refs` may include multiple issues, separated by a
   comma.

   Examples:

   - `Fixes: #1337`
   - `Refs: #1234`

Sample complete commit message:

```txt
subcrate: explain the commit in one line

Body of commit message is a few lines of text, explaining things
in more detail, possibly giving some background about the issue
being fixed, etc.

The body of the commit message can be several paragraphs, and
please do proper word-wrap and keep columns shorter than about
72 characters or so. That way, `git log` will show things
nicely even when it is indented.

Fixes: #1337
Refs: #453, #154
```

### Code style

This project follows the standard conventions for Rust projects that are imposed by 
[`rustfmt`](https://github.com/rust-lang/rustfmt). `rustfmt` is exposed via the 
`cargo fmt` sub-command.

```console
$ cargo fmt
```

To assist with writing idiomatic code, you should also regularly apply the `clippy`
code linter. This can also be invoked by `cargo`:

```console
$ cargo clippy
```

