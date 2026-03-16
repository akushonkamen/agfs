"""
FileSystemInterface - Abstract interface for filesystem operations.

This module provides the FileSystemInterface abstract base class that allows
commands to work with any filesystem implementation (AGFS, Mock, Local).
"""

from abc import ABC, abstractmethod
from typing import Union, Iterator, List, Dict, Any, Optional


class FileSystemInterface(ABC):
    """
    Abstract interface for filesystem operations.

    This allows commands to work with any filesystem implementation:
    - AGFSFileSystem (production with AGFS server)
    - MockFileSystem (testing without server)
    - LocalFileSystem (local-only mode)

    All implementations must provide these core operations.
    """

    @abstractmethod
    def read_file(
        self,
        path: str,
        stream: bool = False,
        binary: bool = False,
        offset: int = 0,
        limit: Optional[int] = None
    ) -> Union[bytes, str, Iterator[bytes]]:
        """
        Read file contents.

        Args:
            path: File path (absolute or relative)
            stream: If True, return iterator; else return complete content
            binary: If True, return bytes; else decode to string
            offset: Byte offset to start reading from
            limit: Maximum number of bytes to read

        Returns:
            File contents as bytes/str or iterator of bytes

        Raises:
            FileNotFoundError: If file doesn't exist
            IsADirectoryError: If path is a directory
            PermissionError: If access denied
        """
        pass

    @abstractmethod
    def write_file(
        self,
        path: str,
        data: Union[bytes, str, Iterator[bytes]],
        append: bool = False
    ) -> Optional[str]:
        """
        Write file contents.

        Args:
            path: File path (absolute or relative)
            data: Data to write (bytes, str, or iterator)
            append: If True, append to file; else overwrite

        Returns:
            Error message if failed, None if success

        Raises:
            PermissionError: If access denied
            IsADirectoryError: If path is a directory
        """
        pass

    @abstractmethod
    def list_directory(self, path: str) -> List[Dict[str, Any]]:
        """
        List directory contents.

        Args:
            path: Directory path

        Returns:
            List of file/directory metadata dicts with keys:
            - name: File/directory name
            - path: Full path
            - is_dir: True if directory
            - size: File size in bytes (for files)
            - mtime: Modification time (timestamp)
            - mode: File mode/permissions

        Raises:
            FileNotFoundError: If directory doesn't exist
            NotADirectoryError: If path is not a directory
            PermissionError: If access denied
        """
        pass

    @abstractmethod
    def file_exists(self, path: str) -> bool:
        """
        Check if file or directory exists.

        Args:
            path: File or directory path

        Returns:
            True if exists, False otherwise
        """
        pass

    @abstractmethod
    def is_directory(self, path: str) -> bool:
        """
        Check if path is a directory.

        Args:
            path: Path to check

        Returns:
            True if path exists and is a directory, False otherwise
        """
        pass

    @abstractmethod
    def is_file(self, path: str) -> bool:
        """
        Check if path is a regular file.

        Args:
            path: Path to check

        Returns:
            True if path exists and is a file, False otherwise
        """
        pass

    @abstractmethod
    def create_directory(self, path: str) -> Optional[str]:
        """
        Create directory.

        Args:
            path: Directory path to create

        Returns:
            Error message if failed, None if success

        Raises:
            FileExistsError: If directory already exists
            PermissionError: If access denied
        """
        pass

    @abstractmethod
    def delete_file(self, path: str, recursive: bool = False) -> Optional[str]:
        """
        Delete file or directory.

        Args:
            path: File or directory path to delete
            recursive: If True, delete directory and contents recursively

        Returns:
            Error message if failed, None if success

        Raises:
            FileNotFoundError: If path doesn't exist
            IsADirectoryError: If path is a directory and recursive=False
            PermissionError: If access denied
        """
        pass

    @abstractmethod
    def get_metadata(self, path: str) -> Dict[str, Any]:
        """
        Get file or directory metadata.

        Args:
            path: File or directory path

        Returns:
            Metadata dict with keys:
            - name: File/directory name
            - path: Full path
            - size: File size in bytes
            - mtime: Modification time (timestamp)
            - mode: File mode/permissions
            - is_dir: True if directory
            - is_file: True if regular file
            - is_symlink: True if symbolic link

        Raises:
            FileNotFoundError: If path doesn't exist
            PermissionError: If access denied
        """
        pass

    @abstractmethod
    def copy_file(
        self,
        source: str,
        dest: str,
        recursive: bool = False
    ) -> Optional[str]:
        """
        Copy file or directory.

        Args:
            source: Source path
            dest: Destination path
            recursive: If True, copy directory recursively

        Returns:
            Error message if failed, None if success

        Raises:
            FileNotFoundError: If source doesn't exist
            FileExistsError: If dest already exists
            IsADirectoryError: If source is a directory and recursive=False
            PermissionError: If access denied
        """
        pass

    @abstractmethod
    def move_file(self, source: str, dest: str) -> Optional[str]:
        """
        Move/rename file or directory.

        Args:
            source: Source path
            dest: Destination path

        Returns:
            Error message if failed, None if success

        Raises:
            FileNotFoundError: If source doesn't exist
            FileExistsError: If dest already exists
            PermissionError: If access denied
        """
        pass

    @abstractmethod
    def get_size(self, path: str) -> int:
        """
        Get file size in bytes.

        Args:
            path: File path

        Returns:
            File size in bytes

        Raises:
            FileNotFoundError: If file doesn't exist
            IsADirectoryError: If path is a directory
        """
        pass

    def read_text_file(self, path: str) -> str:
        """
        Convenience method to read file as text.

        Args:
            path: File path

        Returns:
            File contents as string

        Raises:
            FileNotFoundError: If file doesn't exist
            IsADirectoryError: If path is a directory
        """
        content = self.read_file(path, binary=False)
        if isinstance(content, str):
            return content
        # Convert bytes to string
        return content.decode('utf-8') if isinstance(content, bytes) else str(content)

    def write_text_file(self, path: str, text: str, append: bool = False) -> Optional[str]:
        """
        Convenience method to write text to file.

        Args:
            path: File path
            text: Text to write
            append: If True, append; else overwrite

        Returns:
            Error message if failed, None if success
        """
        return self.write_file(path, text, append=append)
