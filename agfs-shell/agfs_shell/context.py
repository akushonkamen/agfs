"""
CommandContext - Encapsulates all context needed for command execution.

This module provides the CommandContext dataclass that decouples commands
from the Shell class, making commands more testable and the architecture
more modular.
"""

from dataclasses import dataclass, field
from typing import Dict, List, Optional, Any, TYPE_CHECKING
import os

if TYPE_CHECKING:
    from .filesystem_interface import FileSystemInterface
    from .shell import Shell


@dataclass
class CommandContext:
    """
    Encapsulates all context needed for command execution.

    This provides commands with access to:
    - Current working directory
    - Environment variables
    - File system operations
    - User-defined functions and aliases
    - Variable scopes

    Commands should use this context instead of direct Shell access.

    Example:
        >>> from agfs_shell.context import CommandContext
        >>> ctx = CommandContext(cwd='/tmp', env={'FOO': 'bar'})
        >>> ctx.get_variable('FOO')
        'bar'
        >>> ctx.resolve_path('file.txt')
        '/tmp/file.txt'
    """

    # Core state
    cwd: str = '/'
    env: Dict[str, str] = field(default_factory=dict)
    filesystem: Optional['FileSystemInterface'] = None

    # Functions and aliases
    functions: Dict[str, Any] = field(default_factory=dict)
    aliases: Dict[str, str] = field(default_factory=dict)

    # Variable scopes for local variables
    local_scopes: List[Dict[str, str]] = field(default_factory=list)

    # Optional Shell reference for backward compatibility
    # Commands should avoid using this directly - use context methods instead
    _shell: Optional['Shell'] = None

    def resolve_path(self, path: str) -> str:
        """
        Resolve relative paths to absolute paths.

        Args:
            path: Path to resolve (relative or absolute)

        Returns:
            Absolute path normalized

        Examples:
            >>> ctx = CommandContext(cwd='/home/user')
            >>> ctx.resolve_path('file.txt')
            '/home/user/file.txt'
            >>> ctx.resolve_path('/tmp/file.txt')
            '/tmp/file.txt'
            >>> ctx.resolve_path('../data')
            '/home/data'
        """
        if path.startswith('/'):
            return os.path.normpath(path)
        return os.path.normpath(os.path.join(self.cwd, path))

    def get_variable(self, name: str) -> Optional[str]:
        """
        Get variable value, checking local scopes first, then env.

        Local scopes are checked from innermost to outermost (LIFO).
        If not found in local scopes, checks environment variables.

        Args:
            name: Variable name

        Returns:
            Variable value or None if not found

        Examples:
            >>> ctx = CommandContext(env={'USER': 'alice'})
            >>> ctx.get_variable('USER')
            'alice'
            >>> ctx.get_variable('MISSING')
            None
            >>> ctx.push_local_scope()
            >>> ctx.set_variable('LOCAL', 'value', local=True)
            >>> ctx.get_variable('LOCAL')
            'value'
        """
        # Check local scopes (innermost first)
        for scope in reversed(self.local_scopes):
            if name in scope:
                return scope[name]

        # Check environment variables
        return self.env.get(name)

    def set_variable(self, name: str, value: str, local: bool = False):
        """
        Set variable value.

        Args:
            name: Variable name
            value: Variable value
            local: If True, set in local scope; otherwise in env

        Examples:
            >>> ctx = CommandContext()
            >>> ctx.set_variable('FOO', 'bar')
            >>> ctx.get_variable('FOO')
            'bar'
            >>> ctx.push_local_scope()
            >>> ctx.set_variable('FOO', 'local_value', local=True)
            >>> ctx.get_variable('FOO')
            'local_value'
            >>> ctx.pop_local_scope()
            >>> ctx.get_variable('FOO')
            'bar'
        """
        if local and self.local_scopes:
            # Set in current local scope
            self.local_scopes[-1][name] = value
        else:
            # Set in environment
            self.env[name] = value

    def push_local_scope(self):
        """
        Create a new local variable scope.

        Used when entering a function or block that needs local variables.

        Example:
            >>> ctx = CommandContext()
            >>> ctx.push_local_scope()
            >>> ctx.set_variable('x', '10', local=True)
            >>> len(ctx.local_scopes)
            1
        """
        self.local_scopes.append({})

    def pop_local_scope(self):
        """
        Remove the current local variable scope.

        Used when exiting a function or block.

        Example:
            >>> ctx = CommandContext()
            >>> ctx.push_local_scope()
            >>> ctx.set_variable('x', '10', local=True)
            >>> ctx.pop_local_scope()
            >>> len(ctx.local_scopes)
            0
        """
        if self.local_scopes:
            self.local_scopes.pop()

    def expand_variables(self, text: str) -> str:
        """
        Expand variables in text (basic implementation).

        Supports:
        - ${VAR} syntax (braced)
        - $VAR syntax (simple)

        For full expansion (arithmetic, command substitution),
        delegate to Shell's ExpressionExpander.

        Args:
            text: Text with variables like $VAR or ${VAR}

        Returns:
            Text with variables expanded

        Examples:
            >>> ctx = CommandContext(env={'USER': 'alice', 'HOME': '/home/alice'})
            >>> ctx.expand_variables('Hello $USER')
            'Hello alice'
            >>> ctx.expand_variables('Path: ${HOME}/docs')
            'Path: /home/alice/docs'
        """
        if self._shell:
            # Delegate to Shell's full expression expander for complete expansion
            # (arithmetic, command substitution, etc.)
            return self._shell.expression_expander.expand(text)

        # Basic expansion for testing contexts without Shell
        import re

        # Replace ${VAR} syntax
        def replace_braced(match):
            var_name = match.group(1)
            return self.get_variable(var_name) or ''

        text = re.sub(r'\$\{([a-zA-Z_][a-zA-Z0-9_]*)\}', replace_braced, text)

        # Replace $VAR syntax
        def replace_simple(match):
            var_name = match.group(1)
            return self.get_variable(var_name) or ''

        text = re.sub(r'\$([a-zA-Z_][a-zA-Z0-9_]*)', replace_simple, text)

        return text

    def get_function(self, name: str) -> Optional[Any]:
        """
        Get function definition by name.

        Args:
            name: Function name

        Returns:
            Function definition or None if not found

        Example:
            >>> ctx = CommandContext()
            >>> ctx.functions['greet'] = {'params': ['name'], 'body': ['echo Hello $1']}
            >>> ctx.get_function('greet')
            {'params': ['name'], 'body': ['echo Hello $1']}
        """
        return self.functions.get(name)

    def has_function(self, name: str) -> bool:
        """
        Check if function is defined.

        Args:
            name: Function name

        Returns:
            True if function exists, False otherwise

        Example:
            >>> ctx = CommandContext()
            >>> ctx.functions['test'] = {}
            >>> ctx.has_function('test')
            True
            >>> ctx.has_function('missing')
            False
        """
        return name in self.functions

    def get_alias(self, name: str) -> Optional[str]:
        """
        Get alias expansion by name.

        Args:
            name: Alias name

        Returns:
            Alias expansion or None if not found

        Example:
            >>> ctx = CommandContext()
            >>> ctx.aliases['ll'] = 'ls -l'
            >>> ctx.get_alias('ll')
            'ls -l'
        """
        return self.aliases.get(name)

    def has_alias(self, name: str) -> bool:
        """
        Check if alias is defined.

        Args:
            name: Alias name

        Returns:
            True if alias exists, False otherwise

        Example:
            >>> ctx = CommandContext()
            >>> ctx.aliases['ll'] = 'ls -l'
            >>> ctx.has_alias('ll')
            True
            >>> ctx.has_alias('missing')
            False
        """
        return name in self.aliases

    def __repr__(self):
        """String representation for debugging"""
        return (
            f"CommandContext(cwd={self.cwd!r}, "
            f"env_vars={len(self.env)}, "
            f"functions={len(self.functions)}, "
            f"aliases={len(self.aliases)}, "
            f"local_scopes={len(self.local_scopes)})"
        )
