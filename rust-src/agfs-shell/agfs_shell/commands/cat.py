"""
CAT command - concatenate and print files.
"""

import sys
from ..process import Process
from ..command_decorators import command
from . import register_command
from .base import handle_filesystem_error


@command(needs_path_resolution=True, supports_streaming=True)
@register_command('cat')
def cmd_cat(process: Process) -> int:
    """
    Concatenate and print files or stdin (streaming mode)

    Usage: cat [file...]
    """
    if not process.args:
        # Read from stdin in chunks
        # Use read() instead of get_value() to properly support streaming pipelines
        stdin_value = process.stdin.read()

        if stdin_value:
            # Data from stdin (from pipeline or buffer)
            process.stdout.write(stdin_value)
            process.stdout.flush()
        else:
            # No data in stdin, read from real stdin (interactive mode)
            try:
                while True:
                    chunk = sys.stdin.buffer.read(8192)
                    if not chunk:
                        break
                    process.stdout.write(chunk)
                    process.stdout.flush()
            except KeyboardInterrupt:
                # Re-raise to allow proper signal propagation in script mode
                raise
    else:
        # Read from files in streaming mode
        for filename in process.args:
            try:
                if process.context.filesystem:
                    # Stream file in chunks
                    stream = process.context.filesystem.read_file(filename, stream=True)
                    try:
                        for chunk in stream:
                            if chunk:
                                process.stdout.write(chunk)
                                process.stdout.flush()
                    except KeyboardInterrupt:
                        # Re-raise to allow proper signal propagation in script mode
                        raise
                else:
                    # Fallback to local filesystem
                    with open(filename, 'rb') as f:
                        while True:
                            chunk = f.read(8192)
                            if not chunk:
                                break
                            process.stdout.write(chunk)
                            process.stdout.flush()
            except Exception as e:
                return handle_filesystem_error(process, e, filename)
    return 0
