"""Unit tests for AliasRegistry component."""

from agfs_shell.alias_registry import AliasRegistry


class TestAliasRegistryCreation:
    """Tests for AliasRegistry initialization."""

    def test_create_empty_registry(self):
        """Test creating empty alias registry."""
        reg = AliasRegistry()
        assert len(reg) == 0
        assert reg.count() == 0
        assert reg.list_all() == []


class TestDefineAlias:
    """Tests for define() method."""

    def test_define_simple_alias(self):
        """Test defining a simple alias."""
        reg = AliasRegistry()
        reg.define("ll", "ls -l")
        assert reg.exists("ll")
        assert reg.get("ll") == "ls -l"

    def test_define_complex_alias(self):
        """Test defining alias with complex command."""
        reg = AliasRegistry()
        reg.define("gst", "git status --short")
        assert reg.get("gst") == "git status --short"

    def test_define_overwrites_existing(self):
        """Test defining same alias overwrites previous."""
        reg = AliasRegistry()
        reg.define("alias", "old value")
        reg.define("alias", "new value")
        assert reg.get("alias") == "new value"


class TestGetAlias:
    """Tests for get() method."""

    def test_get_existing_alias(self):
        """Test getting an existing alias."""
        reg = AliasRegistry()
        reg.define("ll", "ls -l")
        assert reg.get("ll") == "ls -l"

    def test_get_missing_alias(self):
        """Test getting missing alias returns None."""
        reg = AliasRegistry()
        assert reg.get("nonexistent") is None


class TestExistsAlias:
    """Tests for exists() method."""

    def test_exists_true(self):
        """Test exists returns True for existing alias."""
        reg = AliasRegistry()
        reg.define("ll", "ls -l")
        assert reg.exists("ll")

    def test_exists_false(self):
        """Test exists returns False for missing alias."""
        reg = AliasRegistry()
        assert not reg.exists("missing")

    def test_contains_operator(self):
        """Test 'in' operator works."""
        reg = AliasRegistry()
        reg.define("ll", "ls -l")
        assert "ll" in reg
        assert "missing" not in reg


class TestDeleteAlias:
    """Tests for delete() method."""

    def test_delete_existing_alias(self):
        """Test deleting existing alias."""
        reg = AliasRegistry()
        reg.define("ll", "ls -l")
        assert reg.delete("ll")
        assert not reg.exists("ll")

    def test_delete_missing_alias(self):
        """Test deleting missing alias returns False."""
        reg = AliasRegistry()
        assert not reg.delete("nonexistent")

    def test_delete_reduces_count(self):
        """Test delete reduces alias count."""
        reg = AliasRegistry()
        reg.define("alias1", "cmd1")
        reg.define("alias2", "cmd2")
        assert reg.count() == 2
        reg.delete("alias1")
        assert reg.count() == 1


class TestListAliases:
    """Tests for list_all() and get_all() methods."""

    def test_list_all_empty(self):
        """Test listing aliases in empty registry."""
        reg = AliasRegistry()
        assert reg.list_all() == []

    def test_list_all_returns_sorted(self):
        """Test list_all returns sorted alias names."""
        reg = AliasRegistry()
        reg.define("zebra", "z")
        reg.define("alpha", "a")
        reg.define("beta", "b")
        assert reg.list_all() == ["alpha", "beta", "zebra"]

    def test_get_all(self):
        """Test get_all returns all aliases."""
        reg = AliasRegistry()
        reg.define("ll", "ls -l")
        reg.define("la", "ls -la")
        all_aliases = reg.get_all()
        assert all_aliases == {"ll": "ls -l", "la": "ls -la"}


class TestClearRegistry:
    """Tests for clear() method."""

    def test_clear_empty_registry(self):
        """Test clearing empty registry."""
        reg = AliasRegistry()
        reg.clear()
        assert reg.count() == 0

    def test_clear_removes_all_aliases(self):
        """Test clear removes all aliases."""
        reg = AliasRegistry()
        reg.define("alias1", "cmd1")
        reg.define("alias2", "cmd2")
        reg.define("alias3", "cmd3")
        reg.clear()
        assert reg.count() == 0
        assert reg.list_all() == []


class TestCount:
    """Tests for count() and len()."""

    def test_count_empty(self):
        """Test count on empty registry."""
        reg = AliasRegistry()
        assert reg.count() == 0

    def test_count_after_adding(self):
        """Test count increases after adding aliases."""
        reg = AliasRegistry()
        reg.define("alias1", "cmd1")
        assert reg.count() == 1
        reg.define("alias2", "cmd2")
        assert reg.count() == 2

    def test_len_operator(self):
        """Test len() operator."""
        reg = AliasRegistry()
        assert len(reg) == 0
        reg.define("alias1", "cmd1")
        reg.define("alias2", "cmd2")
        assert len(reg) == 2


class TestExpandAlias:
    """Tests for expand() method."""

    def test_expand_simple_alias(self):
        """Test expanding simple alias."""
        reg = AliasRegistry()
        reg.define("ll", "ls -l")
        assert reg.expand("ll") == "ls -l"

    def test_expand_with_arguments(self):
        """Test expanding alias preserves arguments."""
        reg = AliasRegistry()
        reg.define("ll", "ls -l")
        assert reg.expand("ll /tmp") == "ls -l /tmp"

    def test_expand_non_alias_unchanged(self):
        """Test expanding non-alias command returns unchanged."""
        reg = AliasRegistry()
        assert reg.expand("ls -l") == "ls -l"

    def test_expand_recursive(self):
        """Test recursive alias expansion."""
        reg = AliasRegistry()
        reg.define("l", "ls")
        reg.define("ll", "l -l")
        assert reg.expand("ll") == "ls -l"

    def test_expand_recursive_with_args(self):
        """Test recursive expansion preserves arguments."""
        reg = AliasRegistry()
        reg.define("l", "ls")
        reg.define("ll", "l -l")
        assert reg.expand("ll /tmp") == "ls -l /tmp"

    def test_expand_prevents_cycles(self):
        """Test expansion prevents infinite cycles."""
        reg = AliasRegistry()
        reg.define("a", "b")
        reg.define("b", "a")
        # Should stop expanding when cycle detected
        result = reg.expand("a")
        # Exact result depends on implementation, but shouldn't hang
        assert result in ["a", "b"]

    def test_expand_max_depth(self):
        """Test expansion respects max depth."""
        reg = AliasRegistry()
        reg.define("a", "b")
        reg.define("b", "c")
        reg.define("c", "d")
        # Default max_depth should handle this
        result = reg.expand("a")
        assert "d" in result or "c" in result

    def test_expand_non_recursive(self):
        """Test non-recursive expansion."""
        reg = AliasRegistry()
        reg.define("l", "ls")
        reg.define("ll", "l -l")
        # Non-recursive: only expand once
        result = reg.expand("ll", recursive=False)
        assert result == "l -l"

    def test_expand_empty_command(self):
        """Test expanding empty command."""
        reg = AliasRegistry()
        assert reg.expand("") == ""

    def test_expand_whitespace_only(self):
        """Test expanding whitespace-only command."""
        reg = AliasRegistry()
        assert reg.expand("   ") == "   "


class TestRepr:
    """Tests for __repr__."""

    def test_repr_empty(self):
        """Test repr of empty registry."""
        reg = AliasRegistry()
        repr_str = repr(reg)
        assert "AliasRegistry" in repr_str
        assert "0 aliases" in repr_str

    def test_repr_with_aliases(self):
        """Test repr with aliases."""
        reg = AliasRegistry()
        reg.define("ll", "ls -l")
        reg.define("la", "ls -la")
        repr_str = repr(reg)
        assert "AliasRegistry" in repr_str
        assert "2 aliases" in repr_str


class TestComplexScenarios:
    """Tests for complex usage scenarios."""

    def test_multiple_operations(self):
        """Test multiple registry operations."""
        reg = AliasRegistry()

        # Define several aliases
        reg.define("ll", "ls -l")
        reg.define("la", "ls -la")
        reg.define("gst", "git status")

        assert reg.count() == 3

        # Expand one
        assert reg.expand("ll /tmp") == "ls -l /tmp"

        # Delete one
        reg.delete("la")
        assert reg.count() == 2

        # List remaining
        remaining = reg.list_all()
        assert "ll" in remaining
        assert "gst" in remaining
        assert "la" not in remaining

    def test_alias_chains(self):
        """Test chained alias expansions."""
        reg = AliasRegistry()
        reg.define("a", "b")
        reg.define("b", "c")
        reg.define("c", "echo done")

        result = reg.expand("a")
        assert "echo done" in result

    def test_alias_with_special_chars(self):
        """Test aliases with special characters."""
        reg = AliasRegistry()
        reg.define("..", "cd ..")
        reg.define("...", "cd ../..")

        assert reg.get("..") == "cd .."
        assert reg.get("...") == "cd ../.."

    def test_redefine_alias(self):
        """Test redefining alias updates expansion."""
        reg = AliasRegistry()
        reg.define("ll", "ls -l")
        assert reg.get("ll") == "ls -l"

        reg.define("ll", "ls -la")
        assert reg.get("ll") == "ls -la"
        assert reg.count() == 1  # Still just one alias
