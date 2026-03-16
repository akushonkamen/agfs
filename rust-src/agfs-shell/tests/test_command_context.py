"""
Tests for CommandContext.

This module tests the CommandContext dataclass that encapsulates
command execution context.
"""

from agfs_shell.context import CommandContext


class TestCommandContextCreation:
    """Test CommandContext creation and initialization"""

    def test_default_creation(self):
        """Test creating context with default values"""
        ctx = CommandContext()
        assert ctx.cwd == '/'
        assert ctx.env == {}
        assert ctx.functions == {}
        assert ctx.aliases == {}
        assert ctx.local_scopes == []
        assert ctx._shell is None

    def test_creation_with_values(self):
        """Test creating context with specific values"""
        env = {'USER': 'alice', 'HOME': '/home/alice'}
        ctx = CommandContext(cwd='/tmp', env=env)
        assert ctx.cwd == '/tmp'
        assert ctx.env == env
        assert ctx.get_variable('USER') == 'alice'


class TestPathResolution:
    """Test path resolution methods"""

    def test_resolve_absolute_path(self):
        """Test resolving absolute paths"""
        ctx = CommandContext(cwd='/home/user')
        assert ctx.resolve_path('/tmp/file.txt') == '/tmp/file.txt'
        assert ctx.resolve_path('/') == '/'

    def test_resolve_relative_path(self):
        """Test resolving relative paths"""
        ctx = CommandContext(cwd='/home/user')
        assert ctx.resolve_path('file.txt') == '/home/user/file.txt'
        assert ctx.resolve_path('docs/readme.md') == '/home/user/docs/readme.md'

    def test_resolve_parent_directory(self):
        """Test resolving paths with .."""
        ctx = CommandContext(cwd='/home/user/docs')
        assert ctx.resolve_path('../file.txt') == '/home/user/file.txt'
        assert ctx.resolve_path('../../tmp') == '/home/tmp'

    def test_resolve_current_directory(self):
        """Test resolving paths with ."""
        ctx = CommandContext(cwd='/home/user')
        assert ctx.resolve_path('./file.txt') == '/home/user/file.txt'
        assert ctx.resolve_path('.') == '/home/user'


class TestVariableManagement:
    """Test variable get/set operations"""

    def test_get_variable_from_env(self):
        """Test getting variable from environment"""
        env = {'USER': 'alice', 'HOME': '/home/alice'}
        ctx = CommandContext(env=env)
        assert ctx.get_variable('USER') == 'alice'
        assert ctx.get_variable('HOME') == '/home/alice'

    def test_get_missing_variable(self):
        """Test getting non-existent variable"""
        ctx = CommandContext()
        assert ctx.get_variable('MISSING') is None

    def test_set_variable_in_env(self):
        """Test setting variable in environment"""
        ctx = CommandContext()
        ctx.set_variable('FOO', 'bar')
        assert ctx.get_variable('FOO') == 'bar'
        assert ctx.env['FOO'] == 'bar'

    def test_set_variable_updates_existing(self):
        """Test updating existing variable"""
        ctx = CommandContext(env={'FOO': 'old'})
        ctx.set_variable('FOO', 'new')
        assert ctx.get_variable('FOO') == 'new'


class TestLocalScopes:
    """Test local variable scope management"""

    def test_push_local_scope(self):
        """Test creating new local scope"""
        ctx = CommandContext()
        ctx.push_local_scope()
        assert len(ctx.local_scopes) == 1
        assert ctx.local_scopes[0] == {}

    def test_pop_local_scope(self):
        """Test removing local scope"""
        ctx = CommandContext()
        ctx.push_local_scope()
        ctx.pop_local_scope()
        assert len(ctx.local_scopes) == 0

    def test_pop_empty_scopes(self):
        """Test popping when no scopes exist"""
        ctx = CommandContext()
        ctx.pop_local_scope()  # Should not raise error
        assert len(ctx.local_scopes) == 0

    def test_set_local_variable(self):
        """Test setting variable in local scope"""
        ctx = CommandContext(env={'x': 'global'})
        ctx.push_local_scope()
        ctx.set_variable('x', 'local', local=True)
        assert ctx.get_variable('x') == 'local'

    def test_local_variable_shadows_env(self):
        """Test local variable shadows environment variable"""
        ctx = CommandContext(env={'x': 'global'})
        ctx.push_local_scope()
        ctx.set_variable('x', 'local', local=True)
        assert ctx.get_variable('x') == 'local'
        ctx.pop_local_scope()
        assert ctx.get_variable('x') == 'global'

    def test_nested_local_scopes(self):
        """Test nested local scopes"""
        ctx = CommandContext(env={'x': '0'})

        # First level
        ctx.push_local_scope()
        ctx.set_variable('x', '1', local=True)
        assert ctx.get_variable('x') == '1'

        # Second level
        ctx.push_local_scope()
        ctx.set_variable('x', '2', local=True)
        assert ctx.get_variable('x') == '2'

        # Pop second level
        ctx.pop_local_scope()
        assert ctx.get_variable('x') == '1'

        # Pop first level
        ctx.pop_local_scope()
        assert ctx.get_variable('x') == '0'

    def test_local_variable_without_scope(self):
        """Test setting local variable without scope falls back to env"""
        ctx = CommandContext()
        ctx.set_variable('x', 'value', local=True)
        # Should set in env since no local scope exists
        assert ctx.env['x'] == 'value'


class TestVariableExpansion:
    """Test variable expansion in text"""

    def test_expand_simple_variable(self):
        """Test expanding $VAR syntax"""
        ctx = CommandContext(env={'USER': 'alice'})
        result = ctx.expand_variables('Hello $USER')
        assert result == 'Hello alice'

    def test_expand_braced_variable(self):
        """Test expanding ${VAR} syntax"""
        ctx = CommandContext(env={'HOME': '/home/alice'})
        result = ctx.expand_variables('Path: ${HOME}/docs')
        assert result == 'Path: /home/alice/docs'

    def test_expand_multiple_variables(self):
        """Test expanding multiple variables"""
        ctx = CommandContext(env={'USER': 'alice', 'HOST': 'server'})
        result = ctx.expand_variables('$USER@$HOST')
        assert result == 'alice@server'

    def test_expand_missing_variable(self):
        """Test expanding non-existent variable"""
        ctx = CommandContext()
        result = ctx.expand_variables('Value: $MISSING')
        assert result == 'Value: '

    def test_expand_with_local_scope(self):
        """Test expansion with local variables"""
        ctx = CommandContext(env={'x': 'global'})
        ctx.push_local_scope()
        ctx.set_variable('x', 'local', local=True)
        result = ctx.expand_variables('Value: $x')
        assert result == 'Value: local'


class TestFunctionManagement:
    """Test function-related methods"""

    def test_get_function(self):
        """Test getting function definition"""
        ctx = CommandContext()
        func_def = {'params': ['name'], 'body': ['echo Hello $1']}
        ctx.functions['greet'] = func_def
        assert ctx.get_function('greet') == func_def

    def test_get_missing_function(self):
        """Test getting non-existent function"""
        ctx = CommandContext()
        assert ctx.get_function('missing') is None

    def test_has_function(self):
        """Test checking if function exists"""
        ctx = CommandContext()
        ctx.functions['test'] = {}
        assert ctx.has_function('test') is True
        assert ctx.has_function('missing') is False


class TestAliasManagement:
    """Test alias-related methods"""

    def test_get_alias(self):
        """Test getting alias expansion"""
        ctx = CommandContext()
        ctx.aliases['ll'] = 'ls -l'
        assert ctx.get_alias('ll') == 'ls -l'

    def test_get_missing_alias(self):
        """Test getting non-existent alias"""
        ctx = CommandContext()
        assert ctx.get_alias('missing') is None

    def test_has_alias(self):
        """Test checking if alias exists"""
        ctx = CommandContext()
        ctx.aliases['ll'] = 'ls -l'
        assert ctx.has_alias('ll') is True
        assert ctx.has_alias('missing') is False


class TestContextRepresentation:
    """Test string representation"""

    def test_repr(self):
        """Test __repr__ method"""
        ctx = CommandContext(cwd='/tmp', env={'x': '1'})
        ctx.functions['f1'] = {}
        ctx.aliases['a1'] = 'cmd'
        ctx.push_local_scope()

        repr_str = repr(ctx)
        assert 'CommandContext' in repr_str
        assert "cwd='/tmp'" in repr_str
        assert 'env_vars=1' in repr_str
        assert 'functions=1' in repr_str
        assert 'aliases=1' in repr_str
        assert 'local_scopes=1' in repr_str
