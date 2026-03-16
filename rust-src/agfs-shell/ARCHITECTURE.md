# AGFS-Shell Architecture Documentation

> Last Updated: 2026-01-17
> Version: 1.6.0

## Overview

AGFS-Shell is an experimental Unix-style shell with pipeline support that operates entirely through the AGFS distributed filesystem. It implements a pure-Python execution model without using subprocess for built-in commands, making it ideal for educational purposes and AGFS integration.

## Core Architecture

### High-Level Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                        CLI Entry (cli.py)                    │
│                  Command line parsing, modes                 │
└────────────────────────────┬────────────────────────────────┘
                             │
                    ┌────────▼────────┐
                    │   Shell         │
                    │   (shell.py)    │
                    │   - REPL        │
                    │   - Execution   │
                    │   - State mgmt  │
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
┌───────▼───────┐   ┌───────▼───────┐   ┌───────▼────────┐
│  Parser       │   │  Expression   │   │  Executor      │
│  (parser.py)  │   │  (expression) │   │  (executor.py) │
│  - Tokenize   │   │  - Variables  │   │  - AST exec    │
│  - Pipeline   │   │  - Arithmetic │   │  - Control flow│
│  - Redirection│   │  - Cmd subst  │   │  - Functions   │
└───────┬───────┘   └───────────────┘   └───────┬────────┘
        │                                        │
        └────────────┬───────────────────────────┘
                     │
            ┌────────▼────────┐
            │   Pipeline      │
            │   (pipeline.py) │
            │   - Streaming   │
            │   - Thread mgmt │
            └────────┬────────┘
                     │
            ┌────────▼────────┐
            │   Process       │
            │   (process.py)  │
            │   - Execution   │
            │   - Streams I/O │
            └────────┬────────┘
                     │
        ┌────────────┼────────────┐
        │            │            │
┌───────▼──────┐ ┌──▼──────┐ ┌──▼────────┐
│  Commands    │ │ Streams │ │ Filesystem│
│  (commands/) │ │ (I/O)   │ │ (AGFS)    │
└──────────────┘ └─────────┘ └───────────┘
```

## Core Modules

### 1. Entry Point & CLI (`cli.py`, 358 lines)

**Responsibility:** Command-line interface and execution modes

**Key Functions:**
- `main()` - Main entry point
- Parse CLI arguments (`-c`, `--webapp`, script files)
- Handle init scripts (`~/.agfsrc`)
- Environment variable injection (`--env`)

**Execution Modes:**
- Interactive REPL (default)
- Command string execution (`-c "command"`)
- Script file execution
- Web application mode (`--webapp`)

### 2. Shell Core (`shell.py`, 2,801 lines) ✅ **REFACTORED (Phase 6)**

**Responsibility:** Component coordinator and orchestrator

**Architecture:** Component-based design (refactored in Phase 6)

The Shell class has been transformed from a "god object" into a clean coordinator that delegates to specialized components:

#### Core Components (Phase 6)

1. **VariableManager** (`variable_manager.py`, 225 lines)
   - Environment variables (`env`)
   - Local scopes (`local_scopes`)
   - Exit code management (`?`)
   - Variable get/set/unset operations
   - Export functionality

2. **PathManager** (`path_manager.py`, 174 lines)
   - Current working directory (`cwd`)
   - Chroot support (`chroot_root`)
   - Path resolution (absolute/relative)
   - Directory change operations

3. **FunctionRegistry** (`function_registry.py`, 215 lines)
   - User-defined functions storage
   - Function definition management
   - FunctionDefinition dataclass abstraction

4. **AliasRegistry** (`alias_registry.py`, 207 lines)
   - Command aliases (`aliases`)
   - Alias expansion logic
   - Recursive expansion with cycle detection

#### Backward Compatibility

Shell class maintains 100% backward compatibility via properties:
- `shell.env` → `shell.variables.env`
- `shell.cwd` → `shell.path_manager.cwd`
- `shell.functions` → `shell.function_registry` (via proxy dict)
- `shell.aliases` → `shell.alias_registry._aliases`

#### Integration Points

**Direct Components:**
- `CommandParser` - Command parsing
- `ExpressionExpander` - Variable/arithmetic expansion
- `ControlParser` - Control flow parsing
- `ShellExecutor` - AST execution
- `AGFSFileSystem` - File operations
- `JobManager` - Background jobs

**Delegated to Components:**
- Variable operations → `VariableManager`
- Path operations → `PathManager`
- Function operations → `FunctionRegistry`
- Alias operations → `AliasRegistry`

**Benefits:**
- ✅ Clear separation of concerns
- ✅ Each component independently testable
- ✅ Easier to understand and modify
- ✅ Zero breaking changes (100% compatible)

### 3. Parsing System

#### CommandParser (`parser.py`, 360 lines)

**Responsibility:** Parse command lines into executable structures

**Parsing Capabilities:**
- Command tokenization
- Pipeline separation (`|`)
- I/O redirection (`>`, `>>`, `<`, `2>`, `&>`)
- Quote handling (single `'`, double `"`, escape `$'...'`)
- Comment stripping (`#`)

**Output:** `ParsedCommand` objects with:
- Command segments (each pipeline stage)
- Redirection info (stdin, stdout, stderr, append flags)

#### Lexer (`lexer.py`, 368 lines)

**Responsibility:** Low-level tokenization

**Components:**
- `ShellLexer` - Main tokenizer
- `QuoteTracker` - Quote state machine
- `strip_comments()` - Comment removal
- `split_respecting_quotes()` - Quote-aware splitting

**Features:**
- Handles nested quotes
- Tracks escape sequences
- Preserves whitespace in quoted strings

### 4. Expression System (`expression.py`, 1,155 lines)

**Responsibility:** Variable expansion, arithmetic, command substitution

**Components:**

#### ExpressionExpander (Main Class)
- Variable expansion: `$VAR`, `${VAR}`, `${VAR:-default}`
- Arithmetic evaluation: `$((expr))`
- Command substitution: `$(command)`, `` `command` ``
- Escape sequences: `$'...'` syntax
- Brace expansion: `{a,b,c}`

#### Specialized Handlers
- `ParameterExpander` - Parameter expansion with defaults/alternatives
- `ArithmeticEvaluator` - Safe arithmetic evaluation
- `EscapeSequenceHandler` - Escape sequence processing

**Evaluation Order:**
1. Escape sequences (`$'...'`)
2. Variable expansion
3. Command substitution
4. Arithmetic evaluation
5. Brace expansion

### 5. Control Flow System

#### ControlParser (`control_parser.py`, 535 lines)

**Responsibility:** Parse control structures

**Supported Structures:**
- `if`/`then`/`else`/`elif`/`fi`
- `for var in list; do ... done`
- `while condition; do ... done`
- `until condition; do ... done`
- `function name() { ... }`

**Output:** AST nodes defined in `ast_nodes.py`

#### ShellExecutor (`executor.py`, 381 lines)

**Responsibility:** Execute parsed AST

**Execution Model:**
- Statement-by-statement execution
- Control flow via exceptions (`BreakException`, `ContinueException`, `ReturnException`)
- Function call with parameter binding
- Local scope management

#### Control Flow Exceptions (`control_flow.py`, 76 lines)

**Exception Hierarchy:**
```python
ControlFlowException (base)
├── BreakException
├── ContinueException
└── ReturnException(exit_code)
```

**Purpose:** Clean control flow propagation through nested structures

### 6. Execution System

#### Pipeline (`pipeline.py`, 292 lines)

**Responsibility:** Execute command pipelines with streaming

**Key Features:**
- **True streaming:** Queue-based inter-process communication
- **Parallel execution:** Each command runs in its own thread
- **Backpressure handling:** Bounded queues prevent memory exhaustion
- **Error propagation:** Tracks exit codes from all stages

**Architecture:**
```
Input → [Process 1] → Queue → [Process 2] → Queue → [Process N] → Output
         Thread 1              Thread 2              Thread N
```

**Configuration:**
- Queue size: 100 items (configurable)
- Chunk size: 8192 bytes for streaming
- Timeout: 30s default

#### Process (`process.py`, 94 lines)

**Responsibility:** Single command execution context

**State:**
- Command name and arguments
- I/O streams (stdin, stdout, stderr)
- Environment variables
- Filesystem reference
- Shell reference (for special commands)
- Optional executor function

**Methods:**
- `get_stdout()` - Retrieve stdout contents
- `get_stderr()` - Retrieve stderr contents
- `run()` - Execute the process

### 7. Stream System (`streams.py`, 260 lines)

**Responsibility:** Unix-style I/O abstraction

**Stream Classes:**

```
Stream (base class)
├── InputStream
│   ├── from_bytes()
│   ├── from_string()
│   └── from_file()
├── OutputStream
│   ├── to_buffer()
│   ├── to_file()
│   └── AGFSOutputStream (streaming writes)
└── ErrorStream (extends OutputStream)
```

**Features:**
- Buffered and streaming modes
- Binary and text data support
- Iterator protocol for line-by-line reading
- Context manager support

**Streaming Model:**
- Chunk-based processing (8KB default)
- Memory-efficient for large files
- Supports infinite streams (e.g., network data)

### 8. Filesystem Abstraction (`filesystem.py`, 297 lines)

**Responsibility:** AGFS integration layer

**Class:** `AGFSFileSystem`

**Methods:**
- `read_file(path, offset, size, stream)` - Read file content
- `write_file(path, data, append)` - Write file content
- `list_directory(path)` - List directory contents
- `get_file_info(path)` - Get file metadata
- `create_directory(path, parents)` - Create directory
- And more...

**Integration:**
- Uses `pyagfs.AGFSClient` for AGFS communication
- Default server: `http://localhost:8080`
- Configurable timeout (default: 30s)
- Streaming support for large files

**AGFS Path Format:**
- `/local/` - Local filesystem
- `/s3fs/` - S3 filesystem
- `/sqlfs/` - SQL filesystem
- `/vectorfs/` - Vector filesystem (semantic search)
- `/streamfs/` - Stream filesystem

### 9. Command System (`commands/`, 41+ files)

**Responsibility:** Built-in command implementations

**Command Registry:**
- Dynamic loading via `commands/__init__.py`
- `@register_command(name)` decorator for registration
- `@command(...)` decorator for metadata

**Command Categories:**

1. **File Operations:** cat, ls, cp, mv, rm, mkdir, touch, ln, stat, tree
2. **Text Processing:** grep, fsgrep, wc, head, tail, sort, uniq, cut, tr, rev, tee
3. **Environment:** env, export, unset, local
4. **Control Flow:** break, continue, return, exit
5. **Shell Features:** alias, unalias, source, read, help
6. **AGFS Specific:** mount, plugins, chroot, upload
7. **Advanced:** http (HTTP client), llm (AI integration), jq (JSON)

**Command Interface:**
```python
def cmd_name(process: Process) -> int:
    """
    Command implementation.

    Args:
        process: Process with stdin/stdout/stderr, env, filesystem

    Returns:
        Exit code (0 = success)
    """
    # Command logic here
    return 0
```

### 10. Job Management (`job_manager.py`, 154 lines)

**Responsibility:** Background job control

**Features:**
- Thread-based background execution
- Job state tracking (Running, Completed, Failed)
- Job listing and waiting
- Thread-safe operations

**Commands:**
- `cmd &` - Run in background
- `jobs` - List background jobs
- `wait [job_id]` - Wait for job completion

### 11. Web Application (`webapp_server.py` + `webapp/`)

**Responsibility:** Web-based terminal interface

**Backend (`webapp_server.py`, 643 lines):**
- aiohttp WebSocket server
- Real-time shell interaction
- File browsing and editing
- Process management

**Frontend (`webapp/`):**
- React 18 + Vite
- Monaco Editor for file editing
- XTerm.js for terminal emulation
- React Split for resizable panes

**Components:**
- Terminal - Shell interaction
- FileTree - Browse AGFS filesystem
- Editor - Edit files with Monaco
- MenuBar - Application controls

## Data Flow

### Command Execution Flow

```
1. User Input
   │
   ▼
2. Shell.execute(command)
   │
   ├─> Parse command line (CommandParser)
   │   ├─> Tokenize (Lexer)
   │   └─> Extract redirections
   │
   ├─> Expand expressions (ExpressionExpander)
   │   ├─> Variable expansion
   │   ├─> Command substitution
   │   └─> Arithmetic evaluation
   │
   ├─> Check for control structures (ControlParser)
   │   └─> If found: Parse AST → Execute (ShellExecutor)
   │
   └─> Execute command
       │
       ├─> Single command → Process.run()
       │   └─> Lookup command (builtin/function/external)
       │       └─> Execute with I/O streams
       │
       └─> Pipeline → Pipeline.execute()
           └─> Create process chain
               └─> Thread per process
                   └─> Queue-based streaming
```

### Variable Expansion Flow

```
Text with variables
   │
   ▼
ExpressionExpander.expand(text)
   │
   ├─> 1. Escape sequences ($'...')
   │
   ├─> 2. Brace expansion ({a,b,c})
   │
   ├─> 3. Command substitution
   │   ├─> $(cmd) - Modern syntax
   │   └─> `cmd` - Legacy syntax
   │       └─> Capture stdout → Insert into string
   │
   ├─> 4. Parameter expansion
   │   ├─> $VAR - Simple
   │   ├─> ${VAR} - Braced
   │   ├─> ${VAR:-default} - With default
   │   ├─> ${VAR:=default} - Assign default
   │   └─> ${VAR:?error} - Error if unset
   │
   └─> 5. Arithmetic expansion
       └─> $((expr)) - Evaluate arithmetic
           └─> Safe integer arithmetic
```

## Testing Infrastructure

### Test Organization

```
tests/
├── conftest.py            # Fixtures and mocks
├── test_builtins.py       # Command tests (16 tests)
├── test_parser.py         # Parser tests
├── test_pipeline.py       # Pipeline tests
├── test_shell_core.py     # Shell tests (50+ tests) ✨ NEW
├── test_process.py        # Process tests (25+ tests) ✨ NEW
└── integration/           # Integration tests ⏳ TODO
```

### Mock Infrastructure (`conftest.py`)

**MockFileSystem:**
- In-memory filesystem for testing
- Simulates AGFS operations
- No server dependency
- Supports files, directories, metadata

**Fixtures:**
- `mock_filesystem` - MockFileSystem instance
- `mock_shell` - Mock Shell instance
- `mock_process` - Process with mocked components
- `capture_output` - stdout/stderr capture
- `test_data_dir` - Temporary test files

### Coverage Status (Current)

```
Module              Coverage    Status
──────────────────────────────────────
parser.py             75%       ✅ Good
process.py            77%       ✅ Good
pipeline.py           89%       ✅ Excellent
streams.py            65%       ✅ Good
expression.py         37%       ⚠️  Needs work
shell.py              18%       ❌ Low (large file)
commands/*        Variable     ⚠️  Mixed
──────────────────────────────────────
Overall               20%       🎯 Baseline achieved
```

## Extension Points

### 1. Adding New Commands

```python
# commands/mycommand.py
from ..process import Process
from ..command_decorators import command
from . import register_command

@command(needs_path_resolution=True, supports_streaming=True)
@register_command('mycommand')
def cmd_mycommand(process: Process) -> int:
    """My custom command"""
    # Implementation
    return 0
```

### 2. Adding New Filesystem Backends

Extend `AGFSFileSystem` or implement server-side plugin:
- Server plugins: Mounted at runtime via `mount` command
- Client plugins: Located in `~/.agfs/plugins/` or `$AGFS_PLUGIN_PATH`

### 3. Custom Expression Handlers

Extend `ExpressionExpander`:
```python
class CustomExpander(ExpressionExpander):
    def expand(self, text: str) -> str:
        # Custom logic
        return super().expand(text)
```

## Configuration

### Environment Variables

- `AGFS_SERVER_URL` - AGFS server URL (default: `http://localhost:8080`)
- `AGFS_TIMEOUT` - Request timeout in seconds (default: 30)
- `AGFS_PLUGIN_PATH` - Plugin search path
- `HISTFILE` - History file location (default: `~/.agfs_shell_history`)

### Init Script

`~/.agfsrc` - Executed on shell startup:
```bash
# Example .agfsrc
export PATH="/agfs/bin:$PATH"
alias ll='ls -la'
mount vectorfs vectorfs http://localhost:8080/vectorfs
```

## Performance Characteristics

### Streaming Pipeline

- **Memory Usage:** O(chunk_size × num_processes)
- **Throughput:** Limited by slowest pipeline stage
- **Parallelism:** All stages run concurrently
- **Backpressure:** Queue-based (prevents unbounded growth)

### File Operations

- **Small Files (<1MB):** Buffered read/write
- **Large Files (>1MB):** Streaming in 8KB chunks
- **Network Latency:** Amortized via HTTP keep-alive

### Expression Expansion

- **Simple variables:** O(n) where n = text length
- **Command substitution:** O(cmd_exec_time)
- **Nested expansion:** Recursive with stack depth limit

## Security Considerations

### Sandboxing

- **No subprocess by default:** Built-ins execute in-process
- **AGFS isolation:** All file I/O through AGFS server
- **Chroot support:** `chroot /path` restricts filesystem view

### Input Validation

- **Command injection:** Variables quoted in expansions
- **Path traversal:** Normalized paths prevent `../` exploits
- **Resource limits:** No explicit limits (relies on Python/OS)

### Authentication

- Delegated to AGFS server
- Shell inherits server's auth model
- No credential storage in shell

## Known Limitations

1. **No external command execution** - Pure Python builtins only
2. **Limited POSIX compatibility** - Educational implementation
3. **Single-user focus** - No multi-user isolation
4. **Synchronous REPL** - One command at a time
5. **God object (shell.py)** - Needs refactoring (Phase 6 target)

## Refactoring Roadmap

See `REFACTORING.md` and `/Users/dongxu/.claude/plans/stateful-tumbling-stardust.md` for detailed 7-phase refactoring plan:

1. ✅ **Phase 1:** Testing Infrastructure (In Progress)
2. **Phase 2:** Code Deduplication
3. **Phase 3:** Split builtins.py
4. **Phase 4:** CommandContext Abstraction
5. **Phase 5:** Exception Hierarchy
6. **Phase 6:** Split shell.py
7. **Phase 7:** Final Optimization

## References

- Project README: `/Users/dongxu/agfs/agfs-shell/README.md`
- AGFS SDK: `/Users/dongxu/agfs/agfs-sdk/python/`
- Refactoring Plan: `/Users/dongxu/.claude/plans/stateful-tumbling-stardust.md`
- Work Progress: `/Users/dongxu/agfs/agfs-shell/WORK.md`

---

**Document Version:** 1.0
**Last Review:** 2026-01-17
**Next Review:** After Phase 1 completion
