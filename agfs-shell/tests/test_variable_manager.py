"""Unit tests for VariableManager component."""

import pytest
from agfs_shell.variable_manager import VariableManager


class TestVariableManagerCreation:
    """Tests for VariableManager initialization."""

    def test_default_creation(self):
        """Test creating variable manager with defaults."""
        vm = VariableManager()
        assert vm.env["?"] == "0"
        assert "HISTFILE" in vm.env
        assert vm.local_scopes == []

    def test_creation_with_initial_env(self):
        """Test creating variable manager with initial environment."""
        initial = {"FOO": "bar", "BAZ": "qux"}
        vm = VariableManager(initial_env=initial)
        assert vm.env["FOO"] == "bar"
        assert vm.env["BAZ"] == "qux"
        assert vm.env["?"] == "0"  # Special vars still set


class TestVariableGetSet:
    """Tests for variable get/set operations."""

    def test_get_existing_variable(self):
        """Test getting an existing variable."""
        vm = VariableManager()
        vm.env["TEST"] = "value"
        assert vm.get("TEST") == "value"

    def test_get_missing_variable(self):
        """Test getting a missing variable returns empty string."""
        vm = VariableManager()
        assert vm.get("NONEXISTENT") == ""

    def test_get_with_default(self):
        """Test getting missing variable with custom default."""
        vm = VariableManager()
        assert vm.get("MISSING", "default_val") == "default_val"

    def test_set_variable(self):
        """Test setting a variable."""
        vm = VariableManager()
        vm.set("NEW_VAR", "new_value")
        assert vm.env["NEW_VAR"] == "new_value"

    def test_set_overwrites_existing(self):
        """Test setting overwrites existing value."""
        vm = VariableManager()
        vm.env["VAR"] = "old"
        vm.set("VAR", "new")
        assert vm.env["VAR"] == "new"


class TestLocalScopes:
    """Tests for local variable scopes."""

    def test_push_scope(self):
        """Test pushing a new local scope."""
        vm = VariableManager()
        assert len(vm.local_scopes) == 0
        vm.push_scope()
        assert len(vm.local_scopes) == 1
        assert vm.local_scopes[0] == {}

    def test_pop_scope(self):
        """Test popping a local scope."""
        vm = VariableManager()
        vm.push_scope()
        vm.local_scopes[0]["LOCAL_VAR"] = "value"
        scope = vm.pop_scope()
        assert scope == {"LOCAL_VAR": "value"}
        assert len(vm.local_scopes) == 0

    def test_pop_empty_scopes_raises(self):
        """Test popping when no scopes raises IndexError."""
        vm = VariableManager()
        with pytest.raises(IndexError):
            vm.pop_scope()

    def test_set_local_variable(self):
        """Test setting a local variable."""
        vm = VariableManager()
        vm.push_scope()
        vm.set("LOCAL", "value", local=True)
        assert vm.local_scopes[0]["LOCAL"] == "value"

    def test_local_variable_shadows_global(self):
        """Test local variable shadows global."""
        vm = VariableManager()
        vm.env["VAR"] = "global"
        vm.push_scope()
        vm.set("VAR", "local", local=True)
        assert vm.get("VAR") == "local"
        vm.pop_scope()
        assert vm.get("VAR") == "global"

    def test_nested_scopes(self):
        """Test nested local scopes."""
        vm = VariableManager()
        vm.env["VAR"] = "global"

        vm.push_scope()
        vm.set("VAR", "scope1", local=True)
        assert vm.get("VAR") == "scope1"

        vm.push_scope()
        vm.set("VAR", "scope2", local=True)
        assert vm.get("VAR") == "scope2"

        vm.pop_scope()
        assert vm.get("VAR") == "scope1"

        vm.pop_scope()
        assert vm.get("VAR") == "global"


class TestExitCode:
    """Tests for exit code management."""

    def test_default_exit_code(self):
        """Test default exit code is 0."""
        vm = VariableManager()
        assert vm.get_exit_code() == 0

    def test_set_exit_code(self):
        """Test setting exit code."""
        vm = VariableManager()
        vm.set_exit_code(42)
        assert vm.get_exit_code() == 42
        assert vm.env["?"] == "42"

    def test_exit_code_survives_invalid_value(self):
        """Test get_exit_code handles invalid values."""
        vm = VariableManager()
        vm.env["?"] = "not_a_number"
        assert vm.get_exit_code() == 0


class TestExport:
    """Tests for variable export."""

    def test_export_with_value(self):
        """Test exporting a variable with value."""
        vm = VariableManager()
        vm.export("EXPORTED", "value")
        assert vm.env["EXPORTED"] == "value"

    def test_export_without_value(self):
        """Test exporting without value sets empty string."""
        vm = VariableManager()
        vm.export("EMPTY_EXPORT")
        assert vm.env["EMPTY_EXPORT"] == ""

    def test_export_existing_variable(self):
        """Test exporting existing variable doesn't change value."""
        vm = VariableManager()
        vm.env["EXISTING"] = "value"
        vm.export("EXISTING")
        assert vm.env["EXISTING"] == "value"


class TestUnset:
    """Tests for variable unset."""

    def test_unset_global_variable(self):
        """Test unsetting a global variable."""
        vm = VariableManager()
        vm.env["VAR"] = "value"
        vm.unset("VAR")
        assert "VAR" not in vm.env

    def test_unset_local_variable(self):
        """Test unsetting a local variable."""
        vm = VariableManager()
        vm.push_scope()
        vm.set("LOCAL", "value", local=True)
        vm.unset("LOCAL")
        assert "LOCAL" not in vm.local_scopes[0]

    def test_unset_removes_local_prefix(self):
        """Test unsetting removes _local_ prefixed version."""
        vm = VariableManager()
        vm.push_scope()
        vm.set("VAR", "value", local=True)
        assert "_local_VAR" in vm.env
        vm.unset("VAR")
        assert "_local_VAR" not in vm.env


class TestHasVariable:
    """Tests for has_variable check."""

    def test_has_global_variable(self):
        """Test checking for global variable."""
        vm = VariableManager()
        vm.env["GLOBAL"] = "value"
        assert vm.has_variable("GLOBAL")

    def test_has_local_variable(self):
        """Test checking for local variable."""
        vm = VariableManager()
        vm.push_scope()
        vm.set("LOCAL", "value", local=True)
        assert vm.has_variable("LOCAL")

    def test_missing_variable(self):
        """Test checking for missing variable."""
        vm = VariableManager()
        assert not vm.has_variable("MISSING")


class TestGetAllVariables:
    """Tests for get_all_variables."""

    def test_get_all_returns_env(self):
        """Test get_all_variables returns env dict."""
        vm = VariableManager()
        vm.env["VAR1"] = "val1"
        vm.env["VAR2"] = "val2"
        all_vars = vm.get_all_variables()
        assert "VAR1" in all_vars
        assert "VAR2" in all_vars

    def test_get_all_includes_local_scopes(self):
        """Test get_all_variables includes local scope variables."""
        vm = VariableManager()
        vm.env["GLOBAL"] = "global_val"
        vm.push_scope()
        vm.set("LOCAL", "local_val", local=True)
        all_vars = vm.get_all_variables()
        assert all_vars["GLOBAL"] == "global_val"
        assert all_vars["LOCAL"] == "local_val"


class TestClearLocalScopes:
    """Tests for clear_local_scopes."""

    def test_clear_scopes(self):
        """Test clearing all local scopes."""
        vm = VariableManager()
        vm.push_scope()
        vm.push_scope()
        vm.set("VAR1", "val1", local=True)
        vm.clear_local_scopes()
        assert len(vm.local_scopes) == 0

    def test_clear_removes_local_prefixed_vars(self):
        """Test clearing scopes removes _local_ prefixed variables."""
        vm = VariableManager()
        vm.push_scope()
        vm.set("VAR", "value", local=True)
        assert "_local_VAR" in vm.env
        vm.clear_local_scopes()
        assert "_local_VAR" not in vm.env
