"""
Built-in shell commands registry.

All built-in commands have been migrated to the commands/ directory.
This module now serves as a thin wrapper that loads and exposes those commands.
"""

# Load all commands from the commands/ directory
from .commands import load_all_commands, BUILTINS as COMMANDS

# Load all command modules to populate the registry
load_all_commands()

# Export the commands registry for backward compatibility
BUILTINS = COMMANDS


def get_builtin(command: str):
    """
    Get a built-in command executor.

    Args:
        command: The command name to look up

    Returns:
        The command function, or None if not found

    Example:
        >>> executor = get_builtin('echo')
        >>> if executor:
        ...     executor(process)
    """
    return BUILTINS.get(command)
