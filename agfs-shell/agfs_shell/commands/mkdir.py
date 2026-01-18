"""
MKDIR command - create directory.
"""

from ..process import Process
from ..command_decorators import command
from . import register_command
from .base import handle_filesystem_error


@command(needs_path_resolution=True)
@register_command('mkdir')
def cmd_mkdir(process: Process) -> int:
    """
    Create directory

    Usage: mkdir path
    """
    if not process.args:
        process.stderr.write("mkdir: missing operand\n")
        return 1

    if not process.context.filesystem:
        process.stderr.write("mkdir: filesystem not available\n")
        return 1

    path = process.args[0]

    try:
        # Use AGFS client to create directory
        process.context.filesystem.client.mkdir(path)
        return 0
    except Exception as e:
        return handle_filesystem_error(process, e, path)
