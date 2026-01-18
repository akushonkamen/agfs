# Contributing to AGFS-Shell

Thank you for your interest in contributing to AGFS-Shell! This document provides guidelines and best practices for contributors.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Adding New Commands](#adding-new-commands)
- [Testing Guidelines](#testing-guidelines)
- [Code Style](#code-style)
- [Pull Request Process](#pull-request-process)

## Getting Started

AGFS-Shell is an experimental Unix-style shell that operates through the AGFS distributed filesystem. Before contributing, please:

1. Read the [ARCHITECTURE.md](ARCHITECTURE.md) to understand the system design
2. Check existing issues and pull requests
3. Familiarize yourself with the codebase structure

## Development Setup

### Prerequisites

- Python 3.9 or higher
- [uv](https://github.com/astral-sh/uv) package manager (recommended)
- Access to an AGFS server (for integration testing)

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/agfs-shell.git
cd agfs-shell

# Install dependencies with uv
uv sync

# Or with pip
pip install -e ".[dev]"

# Run tests to verify setup
uv run pytest tests/
```

## Project Structure

```
agfs-shell/
├── agfs_shell/           # Main package
│   ├── cli.py           # CLI entry point
│   ├── shell.py         # Shell coordinator
│   ├── commands/        # Built-in commands (57+ commands)
│   ├── variable_manager.py   # Environment & variables
│   ├── path_manager.py       # Path resolution & cwd
│   ├── function_registry.py  # User-defined functions
│   ├── alias_registry.py     # Command aliases
│   ├── parser.py        # Command parser
│   ├── executor.py      # AST executor
│   ├── expression.py    # Variable expansion
│   ├── process.py       # Process abstraction
│   ├── pipeline.py      # Pipeline execution
│   ├── streams.py       # I/O streams
│   ├── filesystem.py    # AGFS filesystem
│   └── exceptions.py    # Custom exceptions
├── tests/               # Test suite
│   ├── conftest.py     # pytest fixtures
│   ├── test_*.py       # Unit tests
│   └── integration/    # Integration tests
└── docs/               # Documentation
```

## Adding New Commands

### Step 1: Create Command File

Create a new file in `agfs_shell/commands/`:

```python
# agfs_shell/commands/mycommand.py
"""
MYCOMMAND - Brief description of what it does.

Similar to Unix's mycommand, this does XYZ.
"""

from ..process import Process
from ..command_decorators import command
from . import register_command


@command(needs_path_resolution=True)  # Set to True if command uses file paths
@register_command('mycommand')
def cmd_mycommand(process: Process) -> int:
    """
    Detailed docstring explaining the command.

    Usage: mycommand [OPTIONS] [ARGS]

    Options:
        -h, --help    Show this help message
        -v            Verbose mode

    Examples:
        mycommand file.txt          # Process file.txt
        mycommand -v *.txt          # Verbose mode on all .txt files
    """
    # Access context
    cwd = process.context.cwd
    env = process.context.env
    filesystem = process.context.filesystem

    # Parse arguments
    if not process.args:
        process.stderr.write(b"mycommand: missing operand\n")
        return 1

    # Implement command logic
    for arg in process.args:
        # Do something with arg
        process.stdout.write(f"Processing {arg}\n".encode())

    return 0  # Success exit code
```

### Step 2: Handle Errors Properly

Use custom exceptions from `agfs_shell.exceptions`:

```python
from ..exceptions import FileNotFoundError, PermissionDeniedError

try:
    content = process.context.filesystem.read_file(filename)
except FileNotFoundError as e:
    process.stderr.write(f"mycommand: {e}\n".encode())
    return 1
except PermissionDeniedError as e:
    process.stderr.write(f"mycommand: {e}\n".encode())
    return 1
```

### Step 3: Add Tests

Create test file in `tests/`:

```python
# tests/test_mycommand.py
def test_mycommand_basic(mock_filesystem):
    """Test basic mycommand functionality."""
    from agfs_shell.commands.mycommand import cmd_mycommand
    from agfs_shell.process import Process
    from agfs_shell.context import CommandContext

    # Setup
    context = CommandContext(filesystem=mock_filesystem)
    process = Process(
        command='mycommand',
        args=['test.txt'],
        context=context
    )

    # Execute
    exit_code = cmd_mycommand(process)

    # Assert
    assert exit_code == 0
    # Add more assertions
```

### Step 4: Update Documentation

- Add command to README.md command list
- Include usage examples
- Document any special behaviors

## Testing Guidelines

### Running Tests

```bash
# Run all tests
uv run pytest tests/

# Run specific test file
uv run pytest tests/test_mycommand.py

# Run with coverage
uv run pytest tests/ --cov=agfs_shell --cov-report=html

# Run specific test
uv run pytest tests/test_mycommand.py::test_mycommand_basic -v
```

### Test Organization

Tests are organized by component:

- `test_shell_core.py` - Shell class tests
- `test_process.py` - Process class tests
- `test_commands/` - Command-specific tests
- `test_variable_manager.py` - VariableManager tests
- `test_path_manager.py` - PathManager tests
- `test_function_registry.py` - FunctionRegistry tests
- `test_alias_registry.py` - AliasRegistry tests

### Writing Good Tests

1. **Use descriptive names**
   ```python
   def test_cat_reads_file_content():  # Good
   def test_cat():  # Bad
   ```

2. **Arrange-Act-Assert pattern**
   ```python
   def test_something():
       # Arrange - Setup
       data = create_test_data()

       # Act - Execute
       result = function_under_test(data)

       # Assert - Verify
       assert result == expected_value
   ```

3. **Test edge cases**
   - Empty input
   - Missing files
   - Invalid arguments
   - Permission errors

4. **Use fixtures** (from `conftest.py`)
   ```python
   def test_with_fixture(mock_filesystem):
       # mock_filesystem is automatically provided
       mock_filesystem.write_file('/test.txt', b'content')
   ```

### Test Coverage Goals

- New commands: 100% coverage
- Core components: >80% coverage
- Overall project: >70% coverage

## Code Style

### Formatting

We use automated formatters:

```bash
# Format code
uv run black agfs_shell/ tests/

# Sort imports
uv run isort agfs_shell/ tests/

# Check code quality
uv run ruff check agfs_shell/ tests/
```

### Type Hints

Use type hints for all public APIs:

```python
def my_function(name: str, count: int = 0) -> List[str]:
    """Function with type hints."""
    return [name] * count
```

### Docstrings

Follow Google-style docstrings:

```python
def complex_function(param1: str, param2: int) -> dict:
    """Brief description of function.

    Longer description if needed. Can span
    multiple lines.

    Args:
        param1: Description of param1
        param2: Description of param2

    Returns:
        Dictionary with the results

    Raises:
        ValueError: If param2 is negative
        FileNotFoundError: If file doesn't exist

    Examples:
        >>> complex_function("test", 42)
        {'result': 'test processed 42 times'}
    """
    if param2 < 0:
        raise ValueError("param2 must be non-negative")
    return {'result': f'{param1} processed {param2} times'}
```

### Code Organization

1. **Imports order:**
   - Standard library
   - Third-party packages
   - Local imports

2. **Class/function order:**
   - Public before private
   - `__init__` first
   - Related methods grouped together

3. **Line length:**
   - Maximum 88 characters (Black default)

## CI/CD and Pre-commit Hooks

### Automated Quality Checks

This project uses GitHub Actions to automatically run tests and quality checks on every push and pull request.

**Workflows:**
- `.github/workflows/test.yml` - Runs tests on Python 3.9, 3.10, 3.11, 3.12
- `.github/workflows/lint.yml` - Runs black, isort, ruff checks

**Coverage Reports:**
- Coverage reports are uploaded to Codecov
- View coverage at: `https://codecov.io/gh/YOUR_USERNAME/agfs-shell`

### Using Pre-commit Hooks Locally

Pre-commit hooks run quality checks before each commit, catching issues early.

**Setup (one-time):**
```bash
# Install pre-commit (already in dev dependencies)
uv sync --dev

# Install the git hooks
uv run pre-commit install
```

**What it checks:**
- Trailing whitespace
- File endings
- YAML/TOML syntax
- Code formatting (black)
- Import sorting (isort)
- Linting (ruff with auto-fix)

**Manual run:**
```bash
# Run on all files
uv run pre-commit run --all-files

# Run on staged files only
uv run pre-commit run
```

### Quick Quality Check Script

Run all quality checks locally before pushing:

```bash
# Run the quality check script
./scripts/quality-check.sh
```

This script runs:
1. Black formatting check
2. isort import sorting check
3. ruff linting
4. Full test suite
5. Coverage report

**Expected output:**
```
✅ Black check passed
✅ isort check passed
✅ ruff check passed
✅ Tests passed
🎉 All quality checks passed!
```

## Pull Request Process

### Before Submitting

1. **Run tests**
   ```bash
   uv run pytest tests/ -v
   ```

2. **Check code quality**
   ```bash
   uv run black --check agfs_shell/ tests/
   uv run ruff check agfs_shell/ tests/
   ```

3. **Update documentation**
   - Add/update docstrings
   - Update README if needed
   - Update ARCHITECTURE.md for structural changes

4. **Write clear commit messages**
   ```
   Add grep command with regex support

   - Implements basic grep functionality
   - Supports -i (case-insensitive) flag
   - Supports -r (recursive) flag
   - Includes comprehensive tests
   ```

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] All tests pass
- [ ] Added new tests for new functionality
- [ ] Tested manually with AGFS server

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] No unnecessary dependencies added
```

### Review Process

1. Automated tests must pass
2. Code review by maintainer
3. Address feedback
4. Merge after approval

## Architecture Guidelines

### Component Design Principles

1. **Single Responsibility**
   - Each component should have one clear purpose
   - Examples: VariableManager, PathManager, FunctionRegistry

2. **Clear Interfaces**
   - Define clear public APIs
   - Hide implementation details

3. **Testability**
   - Design for easy unit testing
   - Avoid hard dependencies on Shell class

4. **Backward Compatibility**
   - Maintain existing APIs when refactoring
   - Use properties/wrappers for transitions

### When to Create a New Component

Create a new component when:
- Responsibility is distinct and cohesive
- Code can be tested independently
- Multiple commands/modules need the functionality
- Component will have 100+ lines of code

Don't create components for:
- One-time utilities
- Simple helper functions
- Tightly coupled code

## Common Patterns

### Error Handling

```python
from ..exceptions import FileNotFoundError, PermissionDeniedError

try:
    result = risky_operation()
except FileNotFoundError as e:
    process.stderr.write(f"{process.command}: {e}\n".encode())
    return 1
except PermissionDeniedError as e:
    process.stderr.write(f"{process.command}: {e}\n".encode())
    return 1
```

### Path Resolution

```python
# Always resolve paths relative to cwd
resolved_path = process.context.resolve_path(user_path)

# Access filesystem with resolved path
content = process.context.filesystem.read_file(resolved_path)
```

### Variable Access

```python
# Get variable
value = process.context.get_variable('VAR_NAME')

# Set variable
process.context.set_variable('VAR_NAME', 'value')

# Set local variable (in function)
process.context.set_variable('LOCAL_VAR', 'value', local=True)
```

## Getting Help

- **Questions:** Open a discussion on GitHub
- **Bugs:** Open an issue with detailed steps to reproduce
- **Features:** Open an issue describing the use case

## License

By contributing to AGFS-Shell, you agree that your contributions will be licensed under the project's license.

---

Thank you for contributing to AGFS-Shell! 🚀
