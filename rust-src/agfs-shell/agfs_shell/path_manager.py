"""Path and working directory management for agfs-shell.

This module provides the PathManager class which handles:
- Current working directory tracking
- Path resolution (relative to absolute)
- Chroot support for sandboxed environments
"""

import os
from typing import Optional


class PathManager:
    """Manages paths and working directory.

    This class encapsulates path-related operations including:
    - Tracking current working directory (virtual path)
    - Resolving relative paths to absolute paths
    - Supporting chroot environments

    Attributes:
        cwd: Current working directory (virtual path when chroot is set)
        chroot_root: Root directory for chroot (None means no chroot)
    """

    def __init__(self, initial_cwd: str = "/", chroot_root: Optional[str] = None):
        """Initialize the path manager.

        Args:
            initial_cwd: Initial current working directory (default: '/')
            chroot_root: Optional chroot root directory (default: None)
        """
        self.cwd = initial_cwd
        self.chroot_root = chroot_root

    def resolve_path(self, path: str) -> str:
        """Resolve a relative or absolute path to an absolute path.

        If chroot is set, paths are confined within chroot_root.

        Args:
            path: Path to resolve (can be relative or absolute)

        Returns:
            Absolute path (real path when chroot is set)

        Examples:
            Without chroot:
                resolve_path('/foo/bar') -> '/foo/bar'
                resolve_path('bar') with cwd='/foo' -> '/foo/bar'
                resolve_path('../baz') with cwd='/foo/bar' -> '/foo/baz'

            With chroot='/real/root':
                resolve_path('/foo') -> '/real/root/foo'
                resolve_path('foo') with cwd='/bar' -> '/real/root/bar/foo'
        """
        if not path:
            path = self.cwd

        # No chroot - use original logic
        if self.chroot_root is None:
            if path.startswith("/"):
                return os.path.normpath(path)
            full_path = os.path.join(self.cwd, path)
            return os.path.normpath(full_path)

        # With chroot: user sees virtual paths, we return real paths
        if path.startswith("/"):
            # User input absolute path (relative to chroot_root)
            virtual_path = path
        else:
            # User input relative path (relative to virtual cwd)
            virtual_path = os.path.join(self.cwd, path)

        # Normalize virtual path (handles .. etc)
        virtual_path = os.path.normpath(virtual_path)

        # Ensure virtual path doesn't escape "/"
        # normpath turns "/../.." into "/" which is what we want
        if not virtual_path.startswith("/"):
            virtual_path = "/" + virtual_path

        # Construct real path
        real_path = os.path.join(self.chroot_root, virtual_path.lstrip("/"))
        return os.path.normpath(real_path)

    def change_directory(self, path: str) -> None:
        """Change the current working directory.

        Args:
            path: New directory path (can be relative or absolute)

        Note:
            This updates the virtual cwd. The path should be validated
            (e.g., exists and is a directory) by the caller before calling this.
        """
        # Resolve to get the virtual path
        if self.chroot_root is None:
            # No chroot - straightforward
            if path.startswith("/"):
                self.cwd = os.path.normpath(path)
            else:
                self.cwd = os.path.normpath(os.path.join(self.cwd, path))
        else:
            # With chroot - need to get virtual path
            if path.startswith("/"):
                virtual_path = path
            else:
                virtual_path = os.path.join(self.cwd, path)
            self.cwd = os.path.normpath(virtual_path)

            # Ensure cwd doesn't escape "/"
            if not self.cwd.startswith("/"):
                self.cwd = "/" + self.cwd

    def get_cwd(self) -> str:
        """Get the current working directory.

        Returns:
            Current working directory (virtual path)
        """
        return self.cwd

    def get_real_path(self, virtual_path: str) -> str:
        """Get the real filesystem path for a virtual path.

        This is particularly useful with chroot, where virtual_path
        is what the user sees, and real_path is the actual filesystem location.

        Args:
            virtual_path: Virtual path (as seen by user)

        Returns:
            Real filesystem path

        Examples:
            Without chroot:
                get_real_path('/foo/bar') -> '/foo/bar'

            With chroot='/real/root':
                get_real_path('/foo/bar') -> '/real/root/foo/bar'
        """
        return self.resolve_path(virtual_path)

    def set_chroot(self, chroot_root: Optional[str]) -> None:
        """Set or clear the chroot root directory.

        Args:
            chroot_root: New chroot root directory, or None to disable chroot

        Note:
            When setting chroot, the current working directory is reset to '/'.
        """
        self.chroot_root = chroot_root
        if chroot_root is not None:
            # Reset cwd to root when setting chroot
            self.cwd = "/"

    def is_chrooted(self) -> bool:
        """Check if chroot is enabled.

        Returns:
            True if chroot is enabled, False otherwise
        """
        return self.chroot_root is not None

    def normalize_path(self, path: str) -> str:
        """Normalize a path without resolving it relative to cwd.

        This is useful for cleaning up paths without context.

        Args:
            path: Path to normalize

        Returns:
            Normalized path
        """
        normalized = os.path.normpath(path)
        # Ensure absolute paths start with /
        if path.startswith("/") and not normalized.startswith("/"):
            normalized = "/" + normalized
        return normalized
