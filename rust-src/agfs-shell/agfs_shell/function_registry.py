"""Function registry for user-defined functions in agfs-shell.

This module provides the FunctionRegistry class which handles:
- Storing user-defined function definitions
- Looking up functions by name
- Managing function lifecycle (define, delete, list)
"""

from dataclasses import dataclass, field
from typing import Dict, List, Optional


@dataclass
class FunctionDefinition:
    """Represents a user-defined shell function.

    Attributes:
        name: Function name
        params: List of parameter names (optional, for future use)
        body: List of command strings OR AST Statement nodes
        is_ast: Whether body contains AST nodes (True) or strings (False)
    """

    name: str
    params: List[str] = field(default_factory=list)
    body: List = field(default_factory=list)
    is_ast: bool = False

    def __repr__(self) -> str:
        """String representation of the function."""
        return f"FunctionDefinition(name={self.name!r}, params={self.params!r}, body={len(self.body)} lines)"


class FunctionRegistry:
    """Registry for user-defined shell functions.

    This class manages the storage and retrieval of user-defined functions.
    Functions are stored as FunctionDefinition objects with their name,
    parameters, and body.

    Attributes:
        _functions: Internal dictionary mapping function names to definitions
    """

    def __init__(self):
        """Initialize an empty function registry."""
        self._functions: Dict[str, FunctionDefinition] = {}

    def define(
        self,
        name: str,
        params: Optional[List[str]] = None,
        body: Optional[List[str]] = None,
    ) -> None:
        """Define or redefine a function.

        Args:
            name: Function name
            params: List of parameter names (default: empty list)
            body: List of command strings (default: empty list)

        Examples:
            >>> registry = FunctionRegistry()
            >>> registry.define('greet', params=[], body=['echo Hello'])
            >>> registry.define('add', params=['a', 'b'], body=['echo $(($a + $b))'])
        """
        self._functions[name] = FunctionDefinition(
            name=name, params=params or [], body=body or []
        )

    def define_from_dict(self, name: str, func_dict: Dict) -> None:
        """Define a function from a dictionary (legacy compatibility).

        Args:
            name: Function name
            func_dict: Dictionary with 'params', 'body', and optionally 'is_ast' keys

        Examples:
            >>> registry = FunctionRegistry()
            >>> registry.define_from_dict('greet', {'params': [], 'body': ['echo Hello']})
        """
        self._functions[name] = FunctionDefinition(
            name=name,
            params=func_dict.get("params", []),
            body=func_dict.get("body", []),
            is_ast=func_dict.get("is_ast", False),
        )

    def get(self, name: str) -> Optional[FunctionDefinition]:
        """Get a function definition by name.

        Args:
            name: Function name to retrieve

        Returns:
            FunctionDefinition object if found, None otherwise
        """
        return self._functions.get(name)

    def get_as_dict(self, name: str) -> Optional[Dict]:
        """Get a function as a dictionary (legacy compatibility).

        Args:
            name: Function name to retrieve

        Returns:
            Dictionary with 'params', 'body', and 'is_ast' keys, or None if not found
        """
        func = self._functions.get(name)
        if func is None:
            return None
        return {"params": func.params, "body": func.body, "is_ast": func.is_ast}

    def exists(self, name: str) -> bool:
        """Check if a function exists.

        Args:
            name: Function name to check

        Returns:
            True if function exists, False otherwise
        """
        return name in self._functions

    def delete(self, name: str) -> bool:
        """Delete a function.

        Args:
            name: Function name to delete

        Returns:
            True if function was deleted, False if it didn't exist
        """
        if name in self._functions:
            del self._functions[name]
            return True
        return False

    def list_all(self) -> List[str]:
        """List all defined function names.

        Returns:
            List of function names (sorted alphabetically)
        """
        return sorted(self._functions.keys())

    def get_all(self) -> Dict[str, FunctionDefinition]:
        """Get all function definitions.

        Returns:
            Dictionary mapping function names to FunctionDefinition objects
        """
        return dict(self._functions)

    def get_all_as_dict(self) -> Dict[str, Dict]:
        """Get all functions as dictionaries (legacy compatibility).

        Returns:
            Dictionary mapping function names to function dictionaries
        """
        return {
            name: {"params": func.params, "body": func.body, "is_ast": func.is_ast}
            for name, func in self._functions.items()
        }

    def clear(self) -> None:
        """Clear all function definitions."""
        self._functions.clear()

    def count(self) -> int:
        """Get the number of defined functions.

        Returns:
            Number of functions in the registry
        """
        return len(self._functions)

    def __contains__(self, name: str) -> bool:
        """Check if a function exists (allows 'in' operator).

        Args:
            name: Function name to check

        Returns:
            True if function exists, False otherwise
        """
        return name in self._functions

    def __len__(self) -> int:
        """Get the number of defined functions (allows len() function).

        Returns:
            Number of functions in the registry
        """
        return len(self._functions)

    def __repr__(self) -> str:
        """String representation of the registry."""
        return f"FunctionRegistry({len(self._functions)} functions: {', '.join(self.list_all())})"
