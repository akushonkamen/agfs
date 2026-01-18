"""
ENV command - display all environment variables.
"""

from ..process import Process
from ..command_decorators import command
from . import register_command


@command()
@register_command('env')
def cmd_env(process: Process) -> int:
    """
    Display all environment variables

    Usage: env

    Note:
        Uses process.context.env instead of process.shell.env for better decoupling.
    """
    for key, value in sorted(process.context.env.items()):
        process.stdout.write(f"{key}={value}\n".encode('utf-8'))
    return 0
