"""
JOBS command - list background jobs.
"""

from ..process import Process
from ..command_decorators import command
from . import register_command


@command()
@register_command('jobs')
def cmd_jobs(process: Process) -> int:
    """
    List background jobs

    Usage: jobs [-l]
    Options:
      -l  Long format (include thread ID)

    Examples:
      jobs       # List all background jobs
      jobs -l    # List jobs with thread IDs
    """
    show_pid = '-l' in process.args
    shell = process.shell

    if not shell:
        process.stderr.write("jobs: shell instance not available\n")
        return 1

    jobs = shell.job_manager.get_all_jobs()

    if not jobs:
        return 0

    for job in sorted(jobs, key=lambda j: j.job_id):
        status = job.state.value

        if show_pid and job.thread:
            pid = job.thread.ident
            line = f"[{job.job_id}] {pid} {status:12s} {job.command}\n"
        else:
            line = f"[{job.job_id}] {status:12s} {job.command}\n"

        process.stdout.write(line)

    return 0
