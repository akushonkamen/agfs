"""Alias registry for command aliases in agfs-shell.

This module provides the AliasRegistry class which handles:
- Storing command alias definitions
- Expanding aliases in command lines
- Managing alias lifecycle (define, delete, list)
- Preventing infinite recursion in alias expansion
"""

from typing import Dict, List, Optional, Set


class AliasRegistry:
    """Registry for command aliases.

    This class manages command aliases, which are text substitutions
    that replace one command with another string.

    Example:
        alias ll='ls -l'
        When user types 'll', it's expanded to 'ls -l'

    Attributes:
        _aliases: Internal dictionary mapping alias names to expansion strings
    """

    def __init__(self):
        """Initialize an empty alias registry."""
        self._aliases: Dict[str, str] = {}

    def define(self, name: str, expansion: str) -> None:
        """Define or redefine an alias.

        Args:
            name: Alias name (the command to replace)
            expansion: Expansion string (what to replace it with)

        Examples:
            >>> registry = AliasRegistry()
            >>> registry.define('ll', 'ls -l')
            >>> registry.define('la', 'ls -la')
        """
        self._aliases[name] = expansion

    def get(self, name: str) -> Optional[str]:
        """Get an alias expansion by name.

        Args:
            name: Alias name to retrieve

        Returns:
            Expansion string if found, None otherwise
        """
        return self._aliases.get(name)

    def exists(self, name: str) -> bool:
        """Check if an alias exists.

        Args:
            name: Alias name to check

        Returns:
            True if alias exists, False otherwise
        """
        return name in self._aliases

    def delete(self, name: str) -> bool:
        """Delete an alias.

        Args:
            name: Alias name to delete

        Returns:
            True if alias was deleted, False if it didn't exist
        """
        if name in self._aliases:
            del self._aliases[name]
            return True
        return False

    def list_all(self) -> List[str]:
        """List all defined alias names.

        Returns:
            List of alias names (sorted alphabetically)
        """
        return sorted(self._aliases.keys())

    def get_all(self) -> Dict[str, str]:
        """Get all alias definitions.

        Returns:
            Dictionary mapping alias names to expansion strings
        """
        return dict(self._aliases)

    def clear(self) -> None:
        """Clear all alias definitions."""
        self._aliases.clear()

    def count(self) -> int:
        """Get the number of defined aliases.

        Returns:
            Number of aliases in the registry
        """
        return len(self._aliases)

    def expand(self, command: str, recursive: bool = True, max_depth: int = 10) -> str:
        """Expand aliases in a command string.

        This method expands only the first word of the command (the command name).
        It handles recursive expansion with cycle detection.

        Args:
            command: Command string to expand
            recursive: If True, expand aliases recursively (default: True)
            max_depth: Maximum recursion depth to prevent infinite loops (default: 10)

        Returns:
            Expanded command string

        Examples:
            >>> registry = AliasRegistry()
            >>> registry.define('ll', 'ls -l')
            >>> registry.expand('ll /tmp')
            'ls -l /tmp'

            >>> registry.define('la', 'll -a')
            >>> registry.expand('la /tmp')
            'ls -l -a /tmp'
        """
        if not command or not command.strip():
            return command

        # Split command into first word and rest
        parts = command.split(None, 1)
        if not parts:
            return command

        cmd = parts[0]
        rest = parts[1] if len(parts) > 1 else ""

        # Check if this command is an alias
        if cmd not in self._aliases:
            return command

        if not recursive:
            # Non-recursive: expand once
            expansion = self._aliases[cmd]
            return expansion + (" " + rest if rest else "")

        # Recursive expansion with cycle detection
        return self._expand_recursive(command, set(), 0, max_depth)

    def _expand_recursive(
        self, command: str, seen: Set[str], depth: int, max_depth: int
    ) -> str:
        """Recursively expand aliases with cycle detection.

        Args:
            command: Command string to expand
            seen: Set of alias names already seen in this expansion chain
            depth: Current recursion depth
            max_depth: Maximum allowed recursion depth

        Returns:
            Expanded command string
        """
        # Depth limit to prevent infinite recursion
        if depth >= max_depth:
            return command

        # Split command into first word and rest
        parts = command.split(None, 1)
        if not parts:
            return command

        cmd = parts[0]
        rest = parts[1] if len(parts) > 1 else ""

        # Check if this command is an alias
        if cmd not in self._aliases:
            return command

        # Cycle detection: if we've seen this alias before, stop
        if cmd in seen:
            return command

        # Get the expansion
        expansion = self._aliases[cmd]

        # Mark this alias as seen
        new_seen = seen | {cmd}

        # Recursively expand the expansion
        expanded = self._expand_recursive(expansion, new_seen, depth + 1, max_depth)

        # Combine with the rest of the command
        return expanded + (" " + rest if rest else "")

    def __contains__(self, name: str) -> bool:
        """Check if an alias exists (allows 'in' operator).

        Args:
            name: Alias name to check

        Returns:
            True if alias exists, False otherwise
        """
        return name in self._aliases

    def __len__(self) -> int:
        """Get the number of defined aliases (allows len() function).

        Returns:
            Number of aliases in the registry
        """
        return len(self._aliases)

    def __repr__(self) -> str:
        """String representation of the registry."""
        if self._aliases:
            aliases_str = ", ".join(
                f"{k}={v!r}" for k, v in sorted(self._aliases.items())
            )
            return f"AliasRegistry({len(self._aliases)} aliases: {aliases_str})"
        return "AliasRegistry(0 aliases)"
