"""
LN command - create symbolic links in AGFS.
"""

import os
from ..process import Process
from ..command_decorators import command
from . import register_command


@command(needs_path_resolution=True)
@register_command('ln')
def cmd_ln(process: Process) -> int:
    """
    Create symbolic links

    Usage:
        ln [-s] <target> <link_name>

    Options:
        -s    Create symbolic link (default, always creates symlink)

    Examples:
        ln -s /path/to/target /path/to/link
        ln /path/to/target /path/to/link
    """
    # Parse arguments
    args = process.args[:]

    # Skip -s flag if present (we always create symlinks)
    if args and args[0] == '-s':
        args = args[1:]

    if len(args) != 2:
        process.stderr.write("ln: usage: ln [-s] <target> <link_name>\n")
        return 1

    target = args[0]
    link_name = args[1]

    # Resolve link_name to absolute path
    # target can be relative or absolute, we keep it as-is
    if not link_name.startswith('/'):
        link_name = os.path.join(process.cwd, link_name)
        link_name = os.path.normpath(link_name)

    try:
        process.filesystem.symlink(target, link_name)
        return 0
    except Exception as e:
        error_msg = str(e)
        if "not supported" in error_msg.lower() or "not implemented" in error_msg.lower():
            process.stderr.write(f"ln: symbolic links not supported by this filesystem\n")
        elif "already exists" in error_msg.lower():
            process.stderr.write(f"ln: {link_name}: File exists\n")
        else:
            process.stderr.write(f"ln: {error_msg}\n")
        return 1
