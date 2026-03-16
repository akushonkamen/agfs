"""
I/O wrapper utilities for agfs-shell.

This module provides utility classes for handling I/O operations,
particularly for cases where both text and binary data need to be captured.
"""

import io
from typing import Union


class BufferedTextIO:
    """
    Unified text/binary buffer for stdout/stderr capture.

    This class handles both text (str) and binary (bytes) writes,
    maintaining separate buffers and combining them on read.

    Usage:
        buffer = BufferedTextIO()
        buffer.write(b"binary data")
        buffer.write("text data")
        output = buffer.getvalue()  # Returns combined text

    Attributes:
        text_buffer: StringIO for text data
        byte_buffer: BytesIO for binary data
        buffer: Reference to self for compatibility
    """

    def __init__(self):
        """Initialize text and binary buffers."""
        self.text_buffer = io.StringIO()
        self.byte_buffer = io.BytesIO()
        # Create buffer attribute for binary writes
        self.buffer = self

    def write(self, data: Union[str, bytes]) -> int:
        """
        Write data to appropriate buffer.

        Args:
            data: Text string or binary bytes to write

        Returns:
            Number of bytes/characters written
        """
        if isinstance(data, bytes):
            self.byte_buffer.write(data)
        else:
            self.text_buffer.write(data)
        return len(data)

    def flush(self):
        """Flush buffers (no-op for in-memory buffers)."""
        pass

    def getvalue(self) -> str:
        """
        Get combined buffer contents as text.

        Binary data is decoded to UTF-8 with error replacement.

        Returns:
            Combined text from both buffers
        """
        text = self.text_buffer.getvalue()
        binary = self.byte_buffer.getvalue()

        if binary:
            try:
                text += binary.decode('utf-8', errors='replace')
            except Exception:
                # Silently ignore decode errors
                pass

        return text

    def close(self):
        """Close both buffers."""
        self.text_buffer.close()
        self.byte_buffer.close()

    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit - close buffers."""
        self.close()
        return False
