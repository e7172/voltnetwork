# Contributing to Volt

This document provides guidelines for contributing to the Volt project. We welcome contributions from everyone, whether you're fixing a typo, improving documentation, adding a new feature, or fixing a bug.

## Table of Contents

1. [Code of Conduct](#code-of-conduct)
2. [Getting Started](#getting-started)
3. [Development Environment](#development-environment)
4. [Contribution Workflow](#contribution-workflow)
5. [Coding Standards](#coding-standards)
6. [Testing Guidelines](#testing-guidelines)
7. [Documentation Guidelines](#documentation-guidelines)
8. [Pull Request Process](#pull-request-process)
9. [Issue Reporting](#issue-reporting)
10.   [Community](#community)

## Code of Conduct

The Volt project is committed to fostering an open and welcoming environment. By participating, you are expected to uphold this code. Please report unacceptable behavior to conduct@voltnetwork.org.

### Our Standards

-  Be respectful and inclusive
-  Be collaborative
-  Be transparent
-  Be responsive
-  Be constructive

## Getting Started

### Prerequisites

-  Rust 1.60 or later
-  Cargo
-  Git
-  RocksDB

### Setting Up Your Development Environment

1. Fork the repository on GitHub
2. Clone your fork locally:

```bash
git clone https://github.com/your-username/volt.git
cd volt
```

3. Add the upstream repository as a remote:

```bash
git remote add upstream https://github.com/volt/volt.git
```

4. Build the project:

```bash
cargo build
```

5. Run the tests:

```bash
cargo test
```

## Development Environment

### IDE Setup

We recommend using Visual Studio Code or IntelliJ IDEA/CLion with the Rust plugin. See [DEVELOPER_GUIDE.md](DEVELOPER_GUIDE.md#ide-setup) for detailed setup instructions.

### Useful Commands

-  `cargo build`: Build the project
-  `cargo test`: Run the tests
-  `cargo fmt`: Format the code
-  `cargo clippy`: Run the linter
-  `cargo doc`: Generate documentation
-  `cargo bench`: Run benchmarks

## Contribution Workflow

1. **Find an Issue**: Look for open issues or create a new one if you have a feature request or bug report.
2. **Discuss**: Discuss the issue with maintainers and other contributors to ensure your approach aligns with the project's goals.
3. **Branch**: Create a feature branch from the latest `main` branch.
4. **Develop**: Make your changes, following the coding standards and testing guidelines.
5. **Test**: Run the tests to ensure your changes don't break existing functionality.
6. **Document**: Update documentation as needed.
7. **Commit**: Commit your changes with a clear and descriptive commit message.
8. **Push**: Push your changes to your fork.
9. **Pull Request**: Create a pull request to the main repository.
10.   **Review**: Address any feedback from the code review.
11.   **Merge**: Once approved, your changes will be merged into the main repository.

### Branch Naming

Use the following naming convention for branches:

-  `feature/your-feature-name`: For new features
-  `fix/issue-number-description`: For bug fixes
-  `docs/what-you-are-documenting`: For documentation changes
-  `refactor/what-you-are-refactoring`: For code refactoring
-  `test/what-you-are-testing`: For adding or updating tests

### Commit Messages

Write clear and descriptive commit messages:

```
Short (50 chars or less) summary of changes

More detailed explanatory text, if necessary. Wrap it to about 72
characters or so. The blank line separating the summary from the body
is critical.

Further paragraphs come after blank lines.

- Bullet points are okay, too
- Typically a hyphen or asterisk is used for the bullet, preceded by a
  single space, with blank lines in between

If you use an issue tracker, put references to them at the bottom,
like this:

Resolves: #123
See also: #456, #789
```

## Coding Standards

### Rust Style Guide

We follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/) and use `rustfmt` and `clippy` to enforce a consistent style.

#### Key Points

-  Use `rustfmt` to format your code
-  Use `clippy` to catch common mistakes
-  Follow the naming conventions in the Rust API Guidelines
-  Write clear and concise comments
-  Use meaningful variable and function names
-  Keep functions small and focused
-  Avoid unnecessary complexity

### Error Handling

-  Use `Result` and `Option` types for error handling
-  Use the `?` operator for error propagation
-  Use custom error types with `thiserror` for library code
-  Use `anyhow` for application code

### Documentation

-  Document all public items
-  Use doc comments (`///`) for documentation
-  Include examples in documentation
-  Explain the purpose and behavior of functions
-  Document panics, errors, and edge cases

## Testing Guidelines

### Unit Tests

-  Write unit tests for all public functions
-  Use the `#[test]` attribute for test functions
-  Place tests in a `tests` module at the end of the file
-  Use `assert!`, `assert_eq!`, and `assert_ne!` for assertions
-  Use `#[should_panic]` for tests that should panic

Example:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }

    #[test]
    #[should_panic]
    fn test_divide_by_zero() {
        divide(1, 0);
    }
}
```

### Integration Tests

-  Place integration tests in the `tests` directory
-  Use the `#[test]` attribute for test functions
-  Import the crate being tested with `use crate_name;`
-  Test the public API of the crate

### End-to-End Tests

-  Place end-to-end tests in the `tests` directory
-  Use the `#[test]` attribute with `#[ignore]` for long-running tests
-  Test the entire system from end to end

### Test Coverage

-  Aim for high test coverage
-  Use `cargo tarpaulin` to measure test coverage
-  Focus on testing edge cases and error conditions

## Documentation Guidelines

### Code Documentation

-  Document all public items
-  Use doc comments (`///`) for documentation
-  Include examples in documentation
-  Explain the purpose and behavior of functions
-  Document panics, errors, and edge cases

### User Documentation

-  Keep documentation up to date
-  Use clear and concise language
-  Include examples and use cases
-  Explain concepts in simple terms
-  Use diagrams and illustrations where appropriate

### Markdown Style

-  Use ATX-style headers (`#` for h1, `##` for h2, etc.)
-  Use fenced code blocks with language specifiers
-  Use reference-style links for better readability
-  Use tables for structured information
-  Use lists for sequential or unordered items

## Pull Request Process

1. **Create a Pull Request**: Create a pull request from your feature branch to the `main` branch of the main repository.
2. **Describe Your Changes**: Provide a clear description of the changes and the problem they solve.
3. **Reference Issues**: Reference any related issues using the `Resolves: #123` syntax.
4. **Pass CI Checks**: Ensure that all CI checks pass.
5. **Code Review**: Address any feedback from the code review.
6. **Approval**: Wait for approval from maintainers.
7. **Merge**: Once approved, your changes will be merged into the main repository.

### Pull Request Template

```markdown
## Description

[Describe the changes you've made]

## Related Issues

[Reference any related issues]

## Checklist

-  [ ] I have read the [CONTRIBUTING.md](CONTRIBUTING.md) document
-  [ ] My code follows the code style of this project
-  [ ] I have added tests to cover my changes
-  [ ] All new and existing tests passed
-  [ ] I have updated the documentation accordingly
-  [ ] I have added a changelog entry if necessary
```

## Issue Reporting

### Bug Reports

When reporting a bug, please include:

-  A clear and descriptive title
-  Steps to reproduce the bug
-  Expected behavior
-  Actual behavior
-  Screenshots or logs (if applicable)
-  Environment information (OS, Rust version, etc.)

### Feature Requests

When requesting a feature, please include:

-  A clear and descriptive title
-  A detailed description of the feature
-  The motivation for the feature
-  Examples of how the feature would be used
-  Any alternatives you've considered

### Issue Template

```markdown
## Description

[Describe the issue or feature request]

## Steps to Reproduce (for bugs)

1. [First Step]
2. [Second Step]
3. [and so on...]

## Expected Behavior

[What you expected to happen]

## Actual Behavior

[What actually happened]

## Environment

-  OS: [e.g. Ubuntu 20.04]
-  Rust Version: [e.g. 1.60.0]
-  Volt Version: [e.g. 0.1.0]
```

## Community

### Communication Channels

-  **Discord**: [https://discord.gg/NcKvqbwg](https://discord.gg/NcKvqbwg)
-  **GitHub Discussions**: [https://github.com/volt/volt/discussions](https://github.com/volt/volt/discussions)
-  **Email**: community@voltnetwork.org

### Meetings

-  **Community Call**: Every two weeks on Thursday at 3:00 PM UTC
-  **Developer Meeting**: Every week on Tuesday at 2:00 PM UTC

### Recognition

We recognize and appreciate all contributions to the Volt project. Contributors are listed in the [CONTRIBUTORS.md](CONTRIBUTORS.md) file and on the project website.

### Becoming a Maintainer

If you're interested in becoming a maintainer, please reach out to the existing maintainers. Maintainers are selected based on their contributions to the project, their knowledge of the codebase, and their ability to work with the community.

## License

By contributing to the Volt project, you agree that your contributions will be licensed under the project's MIT License.
