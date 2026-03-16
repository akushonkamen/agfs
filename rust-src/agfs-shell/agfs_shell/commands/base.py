"""
Base utilities for command implementations.

This module provides common helper functions that command modules can use
to reduce code duplication and maintain consistency.
"""

from typing import List, Optional
from ..process import Process


def write_error(process: Process, message: str, prefix_command: bool = True):
    """
    Write an error message to stderr.

    Args:
        process: The process object
        message: The error message
        prefix_command: If True, prefix message with command name
    """
    if prefix_command:
        process.stderr.write(f"{process.command}: {message}\n")
    else:
        process.stderr.write(f"{message}\n")


def validate_arg_count(process: Process, min_args: int = 0, max_args: Optional[int] = None,
                       usage: str = "") -> bool:
    """
    Validate the number of arguments.

    Args:
        process: The process object
        min_args: Minimum required arguments
        max_args: Maximum allowed arguments (None = unlimited)
        usage: Usage string to display on error

    Returns:
        True if valid, False if invalid (error already written to stderr)
    """
    arg_count = len(process.args)

    if arg_count < min_args:
        write_error(process, "missing operand")
        if usage:
            process.stderr.write(f"usage: {usage}\n")
        return False

    if max_args is not None and arg_count > max_args:
        write_error(process, "too many arguments")
        if usage:
            process.stderr.write(f"usage: {usage}\n")
        return False

    return True


def parse_flags_and_args(args: List[str], known_flags: Optional[set] = None) -> tuple:
    """
    Parse command arguments into flags and positional arguments.

    Args:
        args: List of arguments
        known_flags: Set of known flag names (e.g., {'-r', '-h', '-a'})
                    If None, all args starting with '-' are treated as flags

    Returns:
        Tuple of (flags_dict, positional_args)
        flags_dict maps flag name to True (e.g., {'-r': True})
        positional_args is list of non-flag arguments
    """
    flags = {}
    positional = []
    i = 0

    while i < len(args):
        arg = args[i]

        # Check for '--' which stops flag parsing
        if arg == '--':
            # All remaining args are positional
            positional.extend(args[i + 1:])
            break

        # Check if it looks like a flag
        if arg.startswith('-') and len(arg) > 1:
            if known_flags is None or arg in known_flags:
                flags[arg] = True
                i += 1
            else:
                # Unknown flag, treat as positional
                positional.append(arg)
                i += 1
        else:
            # Positional argument
            positional.append(arg)
            i += 1

    return flags, positional


def has_flag(flags: dict, *flag_names: str) -> bool:
    """
    Check if any of the given flags are present.

    Args:
        flags: Dictionary of flags (from parse_flags_and_args)
        *flag_names: One or more flag names to check

    Returns:
        True if any of the flags are present

    Example:
        >>> flags = {'-r': True, '-v': True}
        >>> has_flag(flags, '-r')
        True
        >>> has_flag(flags, '-a')
        False
        >>> has_flag(flags, '-r', '--recursive')
        True
    """
    return any(name in flags for name in flag_names)


def handle_filesystem_error(process: Process, error: Exception, filename: str,
                           command_name: Optional[str] = None) -> int:
    """
    Handle filesystem errors with appropriate error messages.

    This function examines the error message and writes appropriate
    error output to stderr based on the error type.

    Args:
        process: Process object with stderr stream
        error: The exception that was caught
        filename: The file/path that caused the error
        command_name: Optional command name (defaults to process.command)

    Returns:
        Exit code (always 1 for errors)

    Example:
        try:
            process.context.filesystem.read_file(path)
        except Exception as e:
            return handle_filesystem_error(process, e, path)
    """
    cmd = command_name or process.command
    error_msg = str(error)

    # Check for specific error types by examining error message
    if "No such file or directory" in error_msg or "not found" in error_msg.lower():
        process.stderr.write(f"{cmd}: {filename}: No such file or directory\n")
    elif "Permission denied" in error_msg or "permission" in error_msg.lower():
        process.stderr.write(f"{cmd}: {filename}: Permission denied\n")
    elif "Is a directory" in error_msg or "is a directory" in error_msg.lower():
        process.stderr.write(f"{cmd}: {filename}: Is a directory\n")
    elif "Not a directory" in error_msg or "not a directory" in error_msg.lower():
        process.stderr.write(f"{cmd}: {filename}: Not a directory\n")
    elif "File exists" in error_msg or "already exists" in error_msg.lower():
        process.stderr.write(f"{cmd}: {filename}: File exists\n")
    elif "Directory not empty" in error_msg or "not empty" in error_msg.lower():
        process.stderr.write(f"{cmd}: {filename}: Directory not empty\n")
    else:
        # Generic error message
        process.stderr.write(f"{cmd}: {filename}: {error_msg}\n")

    return 1


def handle_not_found_error(process: Process, filename: str,
                          command_name: Optional[str] = None) -> int:
    """
    Handle file/directory not found errors.

    Args:
        process: Process object with stderr stream
        filename: The file/path that was not found
        command_name: Optional command name (defaults to process.command)

    Returns:
        Exit code (always 1 for errors)

    Example:
        if not file_exists:
            return handle_not_found_error(process, path)
    """
    cmd = command_name or process.command
    process.stderr.write(f"{cmd}: {filename}: No such file or directory\n")
    return 1


def handle_permission_error(process: Process, filename: str, operation: str = "access",
                           command_name: Optional[str] = None) -> int:
    """
    Handle permission denied errors.

    Args:
        process: Process object with stderr stream
        filename: The file/path that had permission issues
        operation: Optional operation that was denied (e.g., "read", "write")
        command_name: Optional command name (defaults to process.command)

    Returns:
        Exit code (always 1 for errors)

    Example:
        if permission_denied:
            return handle_permission_error(process, path, "write")
    """
    cmd = command_name or process.command
    process.stderr.write(f"{cmd}: {filename}: Permission denied\n")
    return 1


def handle_is_directory_error(process: Process, filename: str,
                              command_name: Optional[str] = None) -> int:
    """
    Handle 'is a directory' errors.

    Args:
        process: Process object with stderr stream
        filename: The path that is a directory
        command_name: Optional command name (defaults to process.command)

    Returns:
        Exit code (always 1 for errors)

    Example:
        if is_directory:
            return handle_is_directory_error(process, path)
    """
    cmd = command_name or process.command
    process.stderr.write(f"{cmd}: {filename}: Is a directory\n")
    return 1


def handle_not_directory_error(process: Process, filename: str,
                               command_name: Optional[str] = None) -> int:
    """
    Handle 'not a directory' errors.

    Args:
        process: Process object with stderr stream
        filename: The path that is not a directory
        command_name: Optional command name (defaults to process.command)

    Returns:
        Exit code (always 1 for errors)

    Example:
        if not is_directory:
            return handle_not_directory_error(process, path)
    """
    cmd = command_name or process.command
    process.stderr.write(f"{cmd}: {filename}: Not a directory\n")
    return 1


def handle_generic_error(process: Process, error: Exception, context: str = "",
                        command_name: Optional[str] = None) -> int:
    """
    Handle generic errors with optional context.

    Args:
        process: Process object with stderr stream
        error: The exception that was caught
        context: Optional context string (e.g., filename, operation)
        command_name: Optional command name (defaults to process.command)

    Returns:
        Exit code (always 1 for errors)

    Example:
        except Exception as e:
            return handle_generic_error(process, e, f"processing {file}")
    """
    cmd = command_name or process.command
    error_msg = str(error)

    if context:
        process.stderr.write(f"{cmd}: {context}: {error_msg}\n")
    else:
        process.stderr.write(f"{cmd}: {error_msg}\n")

    return 1


__all__ = [
    'write_error',
    'validate_arg_count',
    'parse_flags_and_args',
    'has_flag',
    'handle_filesystem_error',
    'handle_not_found_error',
    'handle_permission_error',
    'handle_is_directory_error',
    'handle_not_directory_error',
    'handle_generic_error',
]
