"""
CONTINUE command - continue to next iteration of a for loop.

Note: Module name is continue_cmd.py because 'continue' is a Python keyword.
"""

from ..process import Process
from ..command_decorators import command
from ..exit_codes import EXIT_CODE_CONTINUE
from . import register_command


@command()
@register_command('continue')
def cmd_continue(process: Process) -> int:
    """
    Continue to next iteration of a for loop

    Usage: continue

    Skip the rest of the current loop iteration and continue with the next one.
    Can only be used inside a for loop.

    Examples:
        for i in 1 2 3 4 5; do
            if test $i -eq 3; then
                continue
            fi
            echo $i
        done
        # Output: 1, 2, 4, 5 (skips 3)
    """
    # Return special exit code to signal continue
    # This will be caught by execute_for_loop
    return EXIT_CODE_CONTINUE
