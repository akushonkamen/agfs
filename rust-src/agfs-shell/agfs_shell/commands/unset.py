"""
UNSET command - unset environment variables.
"""

from ..process import Process
from ..command_decorators import command
from . import register_command


@command()
@register_command('unset')
def cmd_unset(process: Process) -> int:
    """
    Unset environment variables

    Usage: unset VAR [VAR ...]

    Note:
        Uses process.context.env instead of process.env for better decoupling.
    """
    if not process.args:
        process.stderr.write("unset: missing variable name\n")
        return 1

    for var_name in process.args:
        if var_name in process.context.env:
            del process.context.env[var_name]

    return 0
