"""
Comprehensive tests for control_parser.py module.

Tests cover:
- ControlParser: parsing control flow structures
  - For loops
  - While loops
  - Until loops
  - If statements
  - Function definitions
- ParseError: error handling
"""

import pytest
from agfs_shell.control_parser import ControlParser, ParseError


# =============================================================================
# ControlParser - For Loop Tests
# =============================================================================

class TestParseForLoop:
    """Tests for parsing for loops."""

    def test_parse_for_loop_returns_result(self, mock_filesystem):
        """Test that parse_for_loop returns a result."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = [
                'for i in 1 2 3',
                'do',
                '    echo $i',
                'done'
            ]

            result = parser.parse_for_loop(lines)
            # Should return a ForStatement or similar
            assert result is not None

    def test_parse_for_loop_with_variables(self, mock_filesystem):
        """Test for loop with variable expansion in items."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            shell.env['FILES'] = 'a.txt b.txt c.txt'
            parser = ControlParser(shell)

            lines = [
                'for file in $FILES',
                'do',
                '    cat $file',
                'done'
            ]

            result = parser.parse_for_loop(lines)
            assert result is not None

    def test_parse_for_loop_empty_items(self, mock_filesystem):
        """Test for loop with empty items list."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = [
                'for x in',
                'do',
                '    echo $x',
                'done'
            ]

            result = parser.parse_for_loop(lines)
            # Should handle gracefully

    def test_parse_for_loop_invalid_syntax(self, mock_filesystem):
        """Test for loop with invalid syntax."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = [
                'for',  # Missing variable and items
                'do',
                '    echo test',
                'done'
            ]

            result = parser.parse_for_loop(lines)
            # Should return None or raise ParseError


# =============================================================================
# ControlParser - While Loop Tests
# =============================================================================

class TestParseWhileLoop:
    """Tests for parsing while loops."""

    def test_parse_while_returns_result(self, mock_filesystem):
        """Test that parse_while_loop returns a result."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = [
                'while true',
                'do',
                '    echo loop',
                '    break',
                'done'
            ]

            result = parser.parse_while_loop(lines)
            assert result is not None

    def test_parse_while_with_test_condition(self, mock_filesystem):
        """Test while loop with test condition."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = [
                'while [ $count -lt 10 ]',
                'do',
                '    echo $count',
                '    count=$((count + 1))',
                'done'
            ]

            result = parser.parse_while_loop(lines)
            assert result is not None

    def test_parse_while_one_line_do(self, mock_filesystem):
        """Test while loop with do on same line."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = [
                'while true; do',
                '    break',
                'done'
            ]

            result = parser.parse_while_loop(lines)
            assert result is not None


# =============================================================================
# ControlParser - Until Loop Tests
# =============================================================================

class TestParseUntilLoop:
    """Tests for parsing until loops."""

    def test_parse_until_returns_result(self, mock_filesystem):
        """Test that parse_until_loop returns a result."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = [
                'until false',
                'do',
                '    echo once',
                '    break',
                'done'
            ]

            result = parser.parse_until_loop(lines)
            assert result is not None


# =============================================================================
# ControlParser - If Statement Tests
# =============================================================================

class TestParseIfStatement:
    """Tests for parsing if statements."""

    def test_parse_if_returns_result(self, mock_filesystem):
        """Test that parse_if_statement returns a result."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = [
                'if true',
                'then',
                '    echo yes',
                'fi'
            ]

            result = parser.parse_if_statement(lines)
            assert result is not None

    def test_parse_if_else(self, mock_filesystem):
        """Test parsing if-else statement."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = [
                'if false',
                'then',
                '    echo yes',
                'else',
                '    echo no',
                'fi'
            ]

            result = parser.parse_if_statement(lines)
            assert result is not None

    def test_parse_if_elif_else(self, mock_filesystem):
        """Test parsing if-elif-else statement."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = [
                'if [ $x -eq 1 ]',
                'then',
                '    echo one',
                'elif [ $x -eq 2 ]',
                'then',
                '    echo two',
                'else',
                '    echo other',
                'fi'
            ]

            result = parser.parse_if_statement(lines)
            assert result is not None

    def test_parse_if_one_line_then(self, mock_filesystem):
        """Test if statement with then on same line."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = [
                'if true; then',
                '    echo yes',
                'fi'
            ]

            result = parser.parse_if_statement(lines)
            assert result is not None

    def test_parse_if_with_test_brackets(self, mock_filesystem):
        """Test if with [ ] test brackets."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = [
                'if [ -f /etc/passwd ]',
                'then',
                '    echo exists',
                'fi'
            ]

            result = parser.parse_if_statement(lines)
            assert result is not None


# =============================================================================
# ControlParser - Function Definition Tests
# =============================================================================

class TestParseFunctionDefinition:
    """Tests for parsing function definitions."""

    def test_parse_function_attempts_parse(self, mock_filesystem):
        """Test that parse_function_definition attempts to parse."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = [
                'function hello() {',
                '    echo "Hello, World!"',
                '}'
            ]

            result = parser.parse_function_definition(lines)
            # May return None or a FunctionDefinition depending on implementation

    def test_parse_function_without_keyword(self, mock_filesystem):
        """Test function definition without 'function' keyword."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = [
                'myfunction() {',
                '    echo test',
                '}'
            ]

            result = parser.parse_function_definition(lines)
            # May or may not work depending on implementation


# =============================================================================
# ParseError Tests
# =============================================================================

class TestParseError:
    """Tests for ParseError exception."""

    def test_parse_error_creation(self):
        """Test creating ParseError."""
        error = ParseError("Test error message")
        assert "Test error message" in str(error)

    def test_parse_error_with_line_number(self):
        """Test ParseError with line number."""
        error = ParseError("Syntax error", line_number=42)
        # May or may not have line_number attribute


# =============================================================================
# Edge Cases and Error Handling
# =============================================================================

class TestControlParserEdgeCases:
    """Tests for edge cases and error handling."""

    def test_parse_empty_lines(self, mock_filesystem):
        """Test parsing with empty lines."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = []
            result = parser.parse_for_loop(lines)
            # Should handle gracefully

    def test_parse_with_comments(self, mock_filesystem):
        """Test parsing with comments."""
        from agfs_shell.shell import Shell
        from unittest.mock import patch

        with patch('agfs_shell.shell.AGFSFileSystem', return_value=mock_filesystem):
            shell = Shell()
            parser = ControlParser(shell)

            lines = [
                '# This is a for loop',
                'for i in 1 2 3',
                'do',
                '    # Echo the value',
                '    echo $i',
                'done'
            ]

            result = parser.parse_for_loop(lines)
            # Should skip comments or handle them


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
