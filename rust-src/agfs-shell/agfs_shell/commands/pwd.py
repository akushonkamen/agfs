"""
PWD command - print working directory.
"""

from ..process import Process
from ..command_decorators import command
from . import register_command


@command()
@register_command('pwd')
def cmd_pwd(process: Process) -> int:
    """
    Print working directory

    Usage: pwd

    Note:
        Uses process.context.cwd instead of process.shell.cwd for better decoupling.
    """
    # Use virtual_cwd for display (shows path relative to chroot)
    # Falls back to context.cwd if virtual_cwd is not set
    cwd = getattr(process, 'virtual_cwd', None) or process.context.cwd
    process.stdout.write(f"{cwd}\n".encode('utf-8'))
    return 0
