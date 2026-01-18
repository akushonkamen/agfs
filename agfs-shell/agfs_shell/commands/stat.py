"""
STAT command - display file status.
"""

from ..process import Process
from ..command_decorators import command
from ..utils.formatters import mode_to_rwx
from . import register_command
from .base import handle_filesystem_error


@command(needs_path_resolution=True)
@register_command('stat')
def cmd_stat(process: Process) -> int:
    """
    Display file status and check if file exists

    Usage: stat path
    """
    if not process.args:
        process.stderr.write("stat: missing operand\n")
        return 1

    if not process.context.filesystem:
        process.stderr.write("stat: filesystem not available\n")
        return 1

    path = process.args[0]

    try:
        # Get file info from the filesystem
        file_info = process.context.filesystem.get_file_info(path)

        # File exists, display information
        name = file_info.get('name', path.split('/')[-1] if '/' in path else path)
        is_dir = file_info.get('isDir', False) or file_info.get('type') == 'directory'
        size = file_info.get('size', 0)

        # Get mode/permissions
        mode_str = file_info.get('mode', '')
        if mode_str and isinstance(mode_str, str) and len(mode_str) >= 9:
            perms = mode_str[:9]
        elif mode_str and isinstance(mode_str, int):
            perms = mode_to_rwx(mode_str)
        else:
            perms = 'rwxr-xr-x' if is_dir else 'rw-r--r--'

        # Get modification time
        mtime = file_info.get('modTime', file_info.get('mtime', ''))
        if mtime:
            if 'T' in mtime:
                mtime = mtime.replace('T', ' ').replace('Z', '').split('.')[0]
            elif len(mtime) > 19:
                mtime = mtime[:19]
        else:
            mtime = 'unknown'

        # Build output
        file_type = 'directory' if is_dir else 'regular file'
        output = f"  File: {name}\n"
        output += f"  Type: {file_type}\n"
        output += f"  Size: {size} bytes\n"
        output += f"  Mode: {perms}\n"
        output += f"  Modified: {mtime}\n"

        process.stdout.write(output.encode('utf-8'))
        return 0

    except Exception as e:
        return handle_filesystem_error(process, e, path)
