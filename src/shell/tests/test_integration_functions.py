"""Integration tests for function execution with control flow.

These tests verify that functions can contain control flow statements
like if/while/for loops and execute correctly.
"""

import pytest
from agfs_shell.shell import Shell


class TestFunctionsWithControlFlow:
    """Test functions containing control flow statements."""

    @pytest.fixture
    def shell(self):
        """Create a shell instance for testing."""
        return Shell(server_url="http://localhost:8080")

    def test_function_with_if_statement(self, shell):
        """Test that functions can contain if statements."""
        # Define a function with an if statement
        script = """
greet() {
    if [ -z "$1" ]; then
        echo "Hello, World!"
    else
        echo "Hello, $1!"
    fi
}
"""
        exit_code = shell.execute_script_content(script)
        assert exit_code == 0

        # Call the function without arguments
        result = shell._execute_command_substitution("greet")
        assert result.strip() == "Hello, World!"

        # Call the function with an argument
        shell.env['1'] = 'Alice'
        result = shell._execute_command_substitution("greet Alice")
        assert 'Alice' in result

    def test_function_with_nested_if(self, shell):
        """Test that functions can contain nested if statements."""
        script = """
check_number() {
    if [ "$1" -gt 0 ]; then
        if [ "$1" -gt 10 ]; then
            echo "big"
        else
            echo "small"
        fi
    else
        echo "negative"
    fi
}
"""
        exit_code = shell.execute_script_content(script)
        assert exit_code == 0

        # Verify function is defined
        assert 'check_number' in shell.functions

    def test_function_with_for_loop(self, shell):
        """Test that functions can contain for loops."""
        script = """
count_items() {
    for i in 1 2 3; do
        echo "$i"
    done
}
"""
        exit_code = shell.execute_script_content(script)
        assert exit_code == 0

        # Verify function is defined
        assert 'count_items' in shell.functions

    def test_function_with_while_loop(self, shell):
        """Test that functions can contain while loops."""
        script = """
countdown() {
    i=3
    while [ $i -gt 0 ]; do
        echo "$i"
        i=$((i - 1))
    done
}
"""
        exit_code = shell.execute_script_content(script)
        assert exit_code == 0

        # Verify function is defined
        assert 'countdown' in shell.functions

    def test_function_is_ast_flag(self, shell):
        """Test that functions are correctly marked as AST-based."""
        script = """
test_func() {
    if [ "$1" = "yes" ]; then
        echo "confirmed"
    fi
}
"""
        exit_code = shell.execute_script_content(script)
        assert exit_code == 0

        # Verify the function has is_ast=True
        func_dict = shell.function_registry.get_as_dict('test_func')
        assert func_dict is not None
        assert func_dict.get('is_ast') is True
        assert isinstance(func_dict['body'], list)
        # Body should contain Statement objects, not strings
        if len(func_dict['body']) > 0:
            from agfs_shell.ast_nodes import Statement
            # At least one element should be a Statement (not a string)
            has_statement = any(isinstance(item, Statement) for item in func_dict['body'])
            assert has_statement, "Function body should contain Statement objects when is_ast=True"
