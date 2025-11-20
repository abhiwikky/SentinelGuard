# Contributing to SentinelGuard

Thank you for your interest in contributing to SentinelGuard! This document provides guidelines and instructions for contributing.

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help maintain a positive community

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/yourusername/SentinelGuard.git`
3. Create a branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Test your changes thoroughly
6. Commit with clear messages
7. Push to your fork and open a Pull Request

## Development Setup

### Prerequisites

- Windows 10/11 (x64)
- Visual Studio 2019+ with C++ Desktop Development
- Windows Driver Kit (WDK) 10
- Rust (stable)
- Node.js 18+
- Python 3.10+

### Building

See the main [README.md](README.md) for detailed build instructions.

## Coding Standards

### Rust

- Follow Rust style guide (`rustfmt`)
- Use `clippy` for linting
- Document public APIs
- Write unit tests for new features

```bash
cd agent
cargo fmt
cargo clippy
cargo test
```

### C++

- Follow Microsoft C++ coding standards
- Use consistent naming conventions
- Comment complex logic
- Handle errors properly

### TypeScript/React

- Use TypeScript for type safety
- Follow React best practices
- Use Tailwind CSS for styling
- Write component tests

## Testing

- Write tests for new features
- Ensure all tests pass before submitting
- Add integration tests for complex workflows
- Update E2E tests if needed

## Commit Messages

Use clear, descriptive commit messages:

```
feat: Add entropy spike detector
fix: Resolve memory leak in event processing
docs: Update architecture documentation
test: Add unit tests for quarantine module
```

## Pull Request Process

1. Update documentation if needed
2. Add tests for new features
3. Ensure CI passes
4. Request review from maintainers
5. Address feedback promptly

## Areas for Contribution

- New detector modules
- ML model improvements
- UI enhancements
- Documentation
- Performance optimizations
- Security hardening
- Test coverage

## Questions?

Open an issue or contact maintainers for guidance.

Thank you for contributing to SentinelGuard!

