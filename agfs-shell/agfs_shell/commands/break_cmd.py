"""
BREAK command - break out of a for loop.

Note: Module name is break_cmd.py because 'break' is a Python keyword.
"""

from ..process import Process
from ..command_decorators import command
from ..exit_codes import EXIT_CODE_BREAK
from . import register_command


@command()
@register_command('break')
def cmd_break(process: Process) -> int:
    """
    Break out of a for loop

    Usage: break

    Exit from the innermost for loop. Can only be used inside a for loop.

    Examples:
        for i in 1 2 3 4 5; do
            if test $i -eq 3; then
                break
            fi
            echo $i
        done
        # Output: 1, 2 (stops at 3)
    """
    # Return special exit code to signal break
    # This will be caught by execute_for_loop
    return EXIT_CODE_BREAK
