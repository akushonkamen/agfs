"""
Custom exception hierarchy for agfs-shell.

This module defines a structured exception hierarchy that provides:
- Clear error categorization
- Consistent error messages
- Proper exit codes
- Better error handling in commands

Usage:
    from agfs_shell.exceptions import FileNotFoundError, PermissionDeniedError

    try:
        filesystem.read_file(path)
    except FileNotFoundError as e:
        print(f"Error: {e}")
        return e.exit_code
"""

from typing import Optional


class ShellError(Exception):
    """
    Base class for all shell errors.

    All custom exceptions should inherit from this class.
    This allows catching all shell-specific errors with a single except clause.

    Attributes:
        message: Error message
        exit_code: Suggested exit code (default: 1)
    """

    def __init__(self, message: str, exit_code: int = 1):
        super().__init__(message)
        self.message = message
        self.exit_code = exit_code

    def __str__(self):
        return self.message


# =============================================================================
# File System Errors
# =============================================================================

class FileSystemError(ShellError):
    """
    Base class for filesystem-related errors.

    Raised when filesystem operations fail.
    """

    def __init__(self, message: str, path: Optional[str] = None, exit_code: int = 1):
        super().__init__(message, exit_code)
        self.path = path


class FileNotFoundError(FileSystemError):
    """
    Raised when a file or directory does not exist.

    Example:
        raise FileNotFoundError("/path/to/file")
    """

    def __init__(self, path: str, message: Optional[str] = None):
        if message is None:
            message = f"{path}: No such file or directory"
        super().__init__(message, path, exit_code=1)


class FileExistsError(FileSystemError):
    """
    Raised when attempting to create a file that already exists.

    Example:
        raise FileExistsError("/path/to/file")
    """

    def __init__(self, path: str, message: Optional[str] = None):
        if message is None:
            message = f"{path}: File exists"
        super().__init__(message, path, exit_code=1)


class PermissionDeniedError(FileSystemError):
    """
    Raised when permission is denied for a filesystem operation.

    Example:
        raise PermissionDeniedError("/path/to/file", "read")
    """

    def __init__(self, path: str, operation: Optional[str] = None, message: Optional[str] = None):
        if message is None:
            if operation:
                message = f"{path}: Permission denied ({operation})"
            else:
                message = f"{path}: Permission denied"
        super().__init__(message, path, exit_code=1)
        self.operation = operation


class IsADirectoryError(FileSystemError):
    """
    Raised when a file operation is attempted on a directory.

    Example:
        raise IsADirectoryError("/path/to/dir", "read")
    """

    def __init__(self, path: str, operation: Optional[str] = None, message: Optional[str] = None):
        if message is None:
            if operation:
                message = f"{path}: Is a directory (cannot {operation})"
            else:
                message = f"{path}: Is a directory"
        super().__init__(message, path, exit_code=1)
        self.operation = operation


class NotADirectoryError(FileSystemError):
    """
    Raised when a directory operation is attempted on a file.

    Example:
        raise NotADirectoryError("/path/to/file")
    """

    def __init__(self, path: str, message: Optional[str] = None):
        if message is None:
            message = f"{path}: Not a directory"
        super().__init__(message, path, exit_code=1)


class DirectoryNotEmptyError(FileSystemError):
    """
    Raised when attempting to remove a non-empty directory.

    Example:
        raise DirectoryNotEmptyError("/path/to/dir")
    """

    def __init__(self, path: str, message: Optional[str] = None):
        if message is None:
            message = f"{path}: Directory not empty"
        super().__init__(message, path, exit_code=1)


class FileSystemConnectionError(FileSystemError):
    """
    Raised when unable to connect to the filesystem (AGFS server).

    Example:
        raise FileSystemConnectionError("http://localhost:8080")
    """

    def __init__(self, server_url: str, message: Optional[str] = None):
        if message is None:
            message = f"Cannot connect to AGFS server at {server_url}"
        super().__init__(message, path=None, exit_code=1)
        self.server_url = server_url


# =============================================================================
# Command Errors
# =============================================================================

class CommandError(ShellError):
    """
    Base class for command-related errors.

    Raised when command execution fails.
    """

    def __init__(self, command: str, message: str, exit_code: int = 1):
        super().__init__(message, exit_code)
        self.command = command


class CommandNotFoundError(CommandError):
    """
    Raised when a command is not found.

    Example:
        raise CommandNotFoundError("nonexistent")
    """

    def __init__(self, command: str):
        message = f"{command}: command not found"
        super().__init__(command, message, exit_code=127)


class InvalidArgumentError(CommandError):
    """
    Raised when invalid arguments are provided to a command.

    Example:
        raise InvalidArgumentError("ls", "-z: invalid option")
    """

    def __init__(self, command: str, argument: str, message: Optional[str] = None):
        if message is None:
            message = f"{command}: {argument}: invalid argument"
        super().__init__(command, message, exit_code=1)
        self.argument = argument


class CommandSyntaxError(CommandError):
    """
    Raised when command syntax is invalid.

    Example:
        raise CommandSyntaxError("test", "missing argument")
    """

    def __init__(self, command: str, details: str):
        message = f"{command}: {details}"
        super().__init__(command, message, exit_code=2)


# =============================================================================
# Parsing Errors
# =============================================================================

class ParsingError(ShellError):
    """
    Base class for parsing-related errors.

    Raised when parsing shell input fails.
    """

    def __init__(self, message: str, line: Optional[str] = None, position: Optional[int] = None):
        super().__init__(message, exit_code=2)
        self.line = line
        self.position = position


class UnmatchedQuoteError(ParsingError):
    """
    Raised when quotes are not properly matched.

    Example:
        raise UnmatchedQuoteError("echo 'hello", quote_char="'")
    """

    def __init__(self, line: str, quote_char: str = '"'):
        message = f"Unmatched {quote_char} in command"
        super().__init__(message, line=line)
        self.quote_char = quote_char


class UnmatchedBracketError(ParsingError):
    """
    Raised when brackets/braces are not properly matched.

    Example:
        raise UnmatchedBracketError("echo ${VAR", bracket_char="{")
    """

    def __init__(self, line: str, bracket_char: str):
        message = f"Unmatched {bracket_char} in command"
        super().__init__(message, line=line)
        self.bracket_char = bracket_char


class InvalidSyntaxError(ParsingError):
    """
    Raised when shell syntax is invalid.

    Example:
        raise InvalidSyntaxError("| grep foo", "unexpected pipe at start")
    """

    def __init__(self, line: str, details: str):
        message = f"Syntax error: {details}"
        super().__init__(message, line=line)


# =============================================================================
# Expression Errors
# =============================================================================

class ExpressionError(ShellError):
    """
    Base class for expression evaluation errors.

    Raised when expression expansion or evaluation fails.
    """
    pass


class UndefinedVariableError(ExpressionError):
    """
    Raised when referencing an undefined variable with strict mode.

    Example:
        raise UndefinedVariableError("MISSING_VAR")
    """

    def __init__(self, var_name: str):
        message = f"{var_name}: undefined variable"
        super().__init__(message, exit_code=1)
        self.var_name = var_name


class ArithmeticError(ExpressionError):
    """
    Raised when arithmetic evaluation fails.

    Example:
        raise ArithmeticError("1 / 0", "division by zero")
    """

    def __init__(self, expression: str, details: str):
        message = f"Arithmetic error in '{expression}': {details}"
        super().__init__(message, exit_code=1)
        self.expression = expression


class InvalidExpressionError(ExpressionError):
    """
    Raised when expression syntax is invalid.

    Example:
        raise InvalidExpressionError("$(( 1 + ))", "incomplete expression")
    """

    def __init__(self, expression: str, details: str):
        message = f"Invalid expression '{expression}': {details}"
        super().__init__(message, exit_code=1)
        self.expression = expression


# =============================================================================
# Network Errors
# =============================================================================

class NetworkError(ShellError):
    """
    Base class for network-related errors.

    Raised when network operations fail.
    """
    pass


class ConnectionError(NetworkError):
    """
    Raised when network connection fails.

    Example:
        raise ConnectionError("http://example.com", "connection refused")
    """

    def __init__(self, url: str, details: Optional[str] = None):
        if details:
            message = f"Connection error for {url}: {details}"
        else:
            message = f"Cannot connect to {url}"
        super().__init__(message, exit_code=1)
        self.url = url


class TimeoutError(NetworkError):
    """
    Raised when network operation times out.

    Example:
        raise TimeoutError("http://example.com", timeout=30)
    """

    def __init__(self, url: str, timeout: Optional[int] = None):
        if timeout:
            message = f"Timeout after {timeout}s connecting to {url}"
        else:
            message = f"Connection timeout for {url}"
        super().__init__(message, exit_code=1)
        self.url = url
        self.timeout = timeout


# =============================================================================
# Utility Functions
# =============================================================================

def translate_agfs_error(error: Exception, path: Optional[str] = None) -> FileSystemError:
    """
    Translate AGFSClientError to specific FileSystemError.

    This function examines the error message from AGFS SDK and returns
    the appropriate specific exception.

    Args:
        error: The AGFSClientError to translate
        path: Optional path that caused the error

    Returns:
        Specific FileSystemError subclass

    Example:
        try:
            client.read_file(path)
        except AGFSClientError as e:
            raise translate_agfs_error(e, path)
    """
    error_str = str(error).lower()

    # File not found
    if 'not found' in error_str or '404' in error_str:
        return FileNotFoundError(path or "unknown")

    # Permission denied
    if 'permission denied' in error_str or '403' in error_str:
        return PermissionDeniedError(path or "unknown")

    # Is a directory
    if 'is a directory' in error_str:
        return IsADirectoryError(path or "unknown")

    # Not a directory
    if 'not a directory' in error_str:
        return NotADirectoryError(path or "unknown")

    # File exists
    if 'already exists' in error_str or 'file exists' in error_str:
        return FileExistsError(path or "unknown")

    # Directory not empty
    if 'not empty' in error_str:
        return DirectoryNotEmptyError(path or "unknown")

    # Connection errors
    if 'connection refused' in error_str or 'cannot connect' in error_str:
        return FileSystemConnectionError("unknown")

    # Generic filesystem error
    return FileSystemError(str(error), path=path)
