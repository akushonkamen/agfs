"""Variable and scope management for agfs-shell.

This module provides the VariableManager class which handles:
- Environment variables (global)
- Local variable scopes (function/block scopes)
- Special variables like exit code (?)
- Variable expansion and resolution
"""

import os
from typing import Dict, List, Optional


class VariableManager:
    """Manages shell variables and scopes.

    This class encapsulates all variable-related state and operations,
    including environment variables, local scopes, and special variables.

    Attributes:
        env: Global environment variables dictionary
        local_scopes: Stack of local variable scopes
    """

    def __init__(self, initial_env: Optional[Dict[str, str]] = None):
        """Initialize the variable manager.

        Args:
            initial_env: Optional initial environment variables to set
        """
        self.env: Dict[str, str] = {}

        # Inject initial environment variables if provided
        if initial_env:
            self.env.update(initial_env)

        # Initialize special variables
        self.env["?"] = "0"  # Last command exit code

        # Set default history file location
        home = os.path.expanduser("~")
        self.env.setdefault("HISTFILE", os.path.join(home, ".agfs_shell_history"))

        # Variable scope stack for local variables
        # Each entry is a dict of local variables for that scope
        self.local_scopes: List[Dict[str, str]] = []

    def get(self, var_name: str, default: Optional[str] = None) -> Optional[str]:
        """Get variable value, checking local scopes first, then global env.

        Args:
            var_name: Variable name to retrieve
            default: Default value if variable not found (defaults to None)

        Returns:
            Variable value, or default if not found
        """
        # Check if we're in a function and have a local variable
        if self.env.get("_function_depth"):
            local_key = f"_local_{var_name}"
            if local_key in self.env:
                return self.env[local_key]

        # Check local scopes from innermost to outermost
        for scope in reversed(self.local_scopes):
            if var_name in scope:
                return scope[var_name]

        # Fall back to global env
        return self.env.get(var_name, default or "")

    def set(self, var_name: str, value: str, local: bool = False) -> None:
        """Set variable value.

        Args:
            var_name: Variable name to set
            value: Value to assign
            local: If True, set in current local scope; otherwise set in global env
        """
        if local and self.local_scopes:
            # Set in current local scope
            self.local_scopes[-1][var_name] = value
            # Also set in env with _local_ prefix for compatibility
            self.env[f"_local_{var_name}"] = value
        elif self.env.get("_function_depth") and f"_local_{var_name}" in self.env:
            # We're in a function and this variable was declared local
            # Update the local variable, not the global one
            self.env[f"_local_{var_name}"] = value
        else:
            # Set in global env
            self.env[var_name] = value

    def unset(self, var_name: str) -> None:
        """Remove a variable from the environment.

        Args:
            var_name: Variable name to unset
        """
        # Remove from local scopes
        for scope in self.local_scopes:
            if var_name in scope:
                del scope[var_name]

        # Remove from global env
        if var_name in self.env:
            del self.env[var_name]

        # Remove local variant if exists
        local_key = f"_local_{var_name}"
        if local_key in self.env:
            del self.env[local_key]

    def push_scope(self) -> None:
        """Push a new local variable scope onto the stack.

        This is typically called when entering a function or block.
        """
        self.local_scopes.append({})

    def pop_scope(self) -> Dict[str, str]:
        """Pop the current local variable scope from the stack.

        This is typically called when exiting a function or block.

        Returns:
            The popped scope dictionary

        Raises:
            IndexError: If no scopes are on the stack
        """
        if not self.local_scopes:
            raise IndexError("Cannot pop from empty scope stack")

        scope = self.local_scopes.pop()

        # Clean up _local_ prefixed variables from env
        for var_name in scope:
            local_key = f"_local_{var_name}"
            if local_key in self.env:
                del self.env[local_key]

        return scope

    def set_exit_code(self, code: int) -> None:
        """Set the exit code special variable (?).

        Args:
            code: Exit code value (typically 0-255)
        """
        self.env["?"] = str(code)

    def get_exit_code(self) -> int:
        """Get the current exit code.

        Returns:
            Exit code as integer (defaults to 0)
        """
        try:
            return int(self.env.get("?", "0"))
        except ValueError:
            return 0

    def export(self, var_name: str, value: Optional[str] = None) -> None:
        """Export a variable (mark it for export to child processes).

        In this implementation, all env variables are exported by default.
        If value is provided, sets the variable before exporting.

        Args:
            var_name: Variable name to export
            value: Optional value to set before exporting
        """
        if value is not None:
            self.env[var_name] = value
        elif var_name not in self.env:
            # If exporting without value and var doesn't exist, set to empty
            self.env[var_name] = ""

    def has_variable(self, var_name: str) -> bool:
        """Check if a variable exists (in any scope).

        Args:
            var_name: Variable name to check

        Returns:
            True if variable exists, False otherwise
        """
        # Check local scopes
        for scope in reversed(self.local_scopes):
            if var_name in scope:
                return True

        # Check global env
        if var_name in self.env:
            return True

        # Check local variant
        if f"_local_{var_name}" in self.env:
            return True

        return False

    def get_all_variables(self) -> Dict[str, str]:
        """Get all variables (global + current local scope).

        Returns:
            Dictionary with all visible variables
        """
        result = dict(self.env)

        # Overlay local scopes
        for scope in self.local_scopes:
            result.update(scope)

        return result

    def clear_local_scopes(self) -> None:
        """Clear all local scopes (emergency cleanup).

        This should typically not be needed in normal operation,
        but can be useful for error recovery.
        """
        # Clean up _local_ prefixed variables
        for scope in self.local_scopes:
            for var_name in scope:
                local_key = f"_local_{var_name}"
                if local_key in self.env:
                    del self.env[local_key]

        self.local_scopes.clear()
