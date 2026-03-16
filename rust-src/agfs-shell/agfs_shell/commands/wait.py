"""
WAIT command - wait for background jobs to complete.
"""

from ..process import Process
from ..command_decorators import command
from . import register_command


@command()
@register_command('wait')
def cmd_wait(process: Process) -> int:
    """
    Wait for background jobs to complete

    Usage:
      wait          # Wait for all jobs
      wait <job_id> # Wait for specific job

    Returns:
      Exit code of waited job, or 0 if waiting for all jobs

    Examples:
      wait       # Wait for all background jobs to complete
      wait 1     # Wait for job [1] to complete
    """
    shell = process.shell

    if not shell:
        process.stderr.write("wait: shell instance not available\n")
        return 1

    if not process.args:
        # Wait for all jobs
        shell.job_manager.wait_for_all()
        return 0

    # Wait for specific job
    try:
        job_id = int(process.args[0])
    except ValueError:
        process.stderr.write(f"wait: {process.args[0]}: invalid job id\n")
        return 1

    exit_code = shell.job_manager.wait_for_job(job_id)

    if exit_code is None:
        process.stderr.write(f"wait: {job_id}: no such job\n")
        return 127

    return exit_code
