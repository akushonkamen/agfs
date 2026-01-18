"""Process class for command execution in pipelines"""

from typing import List, Optional, Callable, TYPE_CHECKING

if TYPE_CHECKING:
    from .filesystem_interface import FileSystemInterface
    from .shell import Shell
    from .context import CommandContext

from .streams import InputStream, OutputStream, ErrorStream
from .control_flow import ControlFlowException


class Process:
    """Represents a single process/command in a pipeline"""

    def __init__(
        self,
        command: str,
        args: List[str],
        stdin: Optional[InputStream] = None,
        stdout: Optional[OutputStream] = None,
        stderr: Optional[ErrorStream] = None,
        executor: Optional[Callable] = None,
        # NEW: CommandContext parameter (preferred way)
        context: Optional['CommandContext'] = None,
        # OLD: Legacy parameters (for backward compatibility)
        filesystem: Optional['FileSystemInterface'] = None,
        env: Optional[dict] = None,
        shell: Optional['Shell'] = None
    ):
        """
        Initialize a process

        Args:
            command: Command name
            args: Command arguments
            stdin: Input stream
            stdout: Output stream
            stderr: Error stream
            executor: Callable that executes the command
            context: CommandContext with all execution context (preferred)
            filesystem: AGFS file system instance (legacy, for backward compatibility)
            env: Environment variables dictionary (legacy, for backward compatibility)
            shell: Shell instance (legacy, for backward compatibility)

        Note:
            If context is provided, it will be used directly.
            Otherwise, a context will be created from the legacy parameters.
            Commands should prefer using process.context instead of process.shell.
        """
        self.command = command
        self.args = args
        self.stdin = stdin or InputStream.from_bytes(b'')
        self.stdout = stdout or OutputStream.to_buffer()
        self.stderr = stderr or ErrorStream.to_buffer()
        self.executor = executor

        # Initialize context
        if context is not None:
            # Use provided context
            self.context = context
        else:
            # Backward compatibility: create context from legacy parameters
            from .context import CommandContext

            self.context = CommandContext(
                cwd=shell.cwd if shell else '/',
                env=env or {},
                filesystem=filesystem,
                functions=shell.functions if shell else {},
                aliases=shell.aliases if shell else {},
                local_scopes=shell.local_scopes if shell else [],
                _shell=shell
            )

        # Backward compatibility properties
        # Commands should migrate to use self.context instead
        self._filesystem = filesystem
        self._env = env
        self._shell = shell

        self.exit_code = 0

    # Backward compatibility properties

    @property
    def filesystem(self):
        """
        Get filesystem from context (backward compatibility).

        Deprecated: Use process.context.filesystem instead.
        """
        return self.context.filesystem

    @filesystem.setter
    def filesystem(self, value):
        """Set filesystem in context"""
        self.context.filesystem = value
        self._filesystem = value

    @property
    def env(self):
        """
        Get environment variables from context (backward compatibility).

        Deprecated: Use process.context.env instead.
        """
        return self.context.env

    @env.setter
    def env(self, value):
        """Set env in context"""
        self.context.env = value
        self._env = value

    @property
    def shell(self):
        """
        Get shell reference from context (backward compatibility).

        Deprecated: Use process.context methods instead of accessing shell.
        """
        return self.context._shell

    @shell.setter
    def shell(self, value):
        """Set shell in context"""
        self.context._shell = value
        self._shell = value

    @property
    def cwd(self):
        """
        Get current working directory from context (backward compatibility).

        Deprecated: Use process.context.cwd instead.
        """
        return self.context.cwd

    @cwd.setter
    def cwd(self, value):
        """Set cwd in context"""
        self.context.cwd = value

    def execute(self) -> int:
        """
        Execute the process

        Returns:
            Exit code (0 for success, non-zero for error)
        """
        if self.executor is None:
            self.stderr.write(f"Error: No such command '{self.command}'\n")
            self.exit_code = 127
            return self.exit_code

        try:
            # Execute the command
            self.exit_code = self.executor(self)
        except KeyboardInterrupt:
            # Let KeyboardInterrupt propagate for proper Ctrl-C handling
            raise
        except ControlFlowException:
            # Let control flow exceptions (break, continue, return) propagate
            raise
        except Exception as e:
            self.stderr.write(f"Error executing '{self.command}': {str(e)}\n")
            self.exit_code = 1

        # Flush all streams
        self.stdout.flush()
        self.stderr.flush()

        return self.exit_code

    def get_stdout(self) -> bytes:
        """Get stdout contents"""
        return self.stdout.get_value()

    def get_stderr(self) -> bytes:
        """Get stderr contents"""
        return self.stderr.get_value()

    def __repr__(self):
        args_str = ' '.join(self.args) if self.args else ''
        return f"Process({self.command} {args_str})"
