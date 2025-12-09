"""
RETURN command - return from a function with an optional exit status.

Note: Module name is return_cmd.py because 'return' is a Python keyword.
"""

from ..process import Process
from ..command_decorators import command
from ..exit_codes import EXIT_CODE_RETURN
from . import register_command


@command()
@register_command('return')
def cmd_return(process: Process) -> int:
    """
    Return from a function with an optional exit status

    Usage: return [n]

    Examples:
        return          # Return with status 0
        return 1        # Return with status 1
        return $?       # Return with last command's status
    """
    # Parse exit code
    exit_code = 0
    if process.args:
        try:
            exit_code = int(process.args[0])
        except ValueError:
            process.stderr.write(f"return: {process.args[0]}: numeric argument required\n")
            return 2

    # Store return value in env for shell to retrieve
    process.env['_return_value'] = str(exit_code)

    # Return special code to signal return statement
    return EXIT_CODE_RETURN
