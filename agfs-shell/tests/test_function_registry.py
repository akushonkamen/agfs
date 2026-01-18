"""Unit tests for FunctionRegistry component."""

from agfs_shell.function_registry import FunctionRegistry, FunctionDefinition


class TestFunctionDefinition:
    """Tests for FunctionDefinition dataclass."""

    def test_create_function_definition(self):
        """Test creating a function definition."""
        func = FunctionDefinition(
            name="greet", params=["name"], body=["echo Hello, $name"]
        )
        assert func.name == "greet"
        assert func.params == ["name"]
        assert func.body == ["echo Hello, $name"]

    def test_function_definition_defaults(self):
        """Test function definition with defaults."""
        func = FunctionDefinition(name="simple")
        assert func.name == "simple"
        assert func.params == []
        assert func.body == []

    def test_function_definition_repr(self):
        """Test function definition string representation."""
        func = FunctionDefinition(name="test", params=[], body=["cmd1", "cmd2"])
        repr_str = repr(func)
        assert "FunctionDefinition" in repr_str
        assert "name='test'" in repr_str
        assert "2 lines" in repr_str


class TestFunctionRegistryCreation:
    """Tests for FunctionRegistry initialization."""

    def test_create_empty_registry(self):
        """Test creating empty function registry."""
        reg = FunctionRegistry()
        assert len(reg) == 0
        assert reg.count() == 0
        assert reg.list_all() == []


class TestDefineFunction:
    """Tests for define() method."""

    def test_define_simple_function(self):
        """Test defining a simple function."""
        reg = FunctionRegistry()
        reg.define("greet", params=[], body=["echo Hello"])
        assert reg.exists("greet")
        assert reg.count() == 1

    def test_define_function_with_params(self):
        """Test defining function with parameters."""
        reg = FunctionRegistry()
        reg.define("add", params=["a", "b"], body=["echo $(($a + $b))"])
        func = reg.get("add")
        assert func.params == ["a", "b"]

    def test_define_overwrites_existing(self):
        """Test defining same function overwrites previous."""
        reg = FunctionRegistry()
        reg.define("func", body=["old"])
        reg.define("func", body=["new"])
        func = reg.get("func")
        assert func.body == ["new"]

    def test_define_from_dict(self):
        """Test defining function from dictionary."""
        reg = FunctionRegistry()
        reg.define_from_dict("test", {"params": ["x"], "body": ["echo $x"]})
        func = reg.get("test")
        assert func.params == ["x"]
        assert func.body == ["echo $x"]

    def test_define_from_dict_with_missing_keys(self):
        """Test defining from dict with missing keys uses defaults."""
        reg = FunctionRegistry()
        reg.define_from_dict("minimal", {})
        func = reg.get("minimal")
        assert func.params == []
        assert func.body == []


class TestGetFunction:
    """Tests for get() and get_as_dict() methods."""

    def test_get_existing_function(self):
        """Test getting an existing function."""
        reg = FunctionRegistry()
        reg.define("test", body=["echo test"])
        func = reg.get("test")
        assert isinstance(func, FunctionDefinition)
        assert func.name == "test"

    def test_get_missing_function(self):
        """Test getting missing function returns None."""
        reg = FunctionRegistry()
        assert reg.get("nonexistent") is None

    def test_get_as_dict(self):
        """Test getting function as dictionary."""
        reg = FunctionRegistry()
        reg.define("test", params=["a"], body=["echo $a"])
        func_dict = reg.get_as_dict("test")
        assert func_dict == {"params": ["a"], "body": ["echo $a"], "is_ast": False}

    def test_get_as_dict_missing(self):
        """Test getting missing function as dict returns None."""
        reg = FunctionRegistry()
        assert reg.get_as_dict("missing") is None


class TestExistsFunction:
    """Tests for exists() method."""

    def test_exists_true(self):
        """Test exists returns True for existing function."""
        reg = FunctionRegistry()
        reg.define("test")
        assert reg.exists("test")

    def test_exists_false(self):
        """Test exists returns False for missing function."""
        reg = FunctionRegistry()
        assert not reg.exists("missing")

    def test_contains_operator(self):
        """Test 'in' operator works."""
        reg = FunctionRegistry()
        reg.define("test")
        assert "test" in reg
        assert "missing" not in reg


class TestDeleteFunction:
    """Tests for delete() method."""

    def test_delete_existing_function(self):
        """Test deleting existing function."""
        reg = FunctionRegistry()
        reg.define("test")
        assert reg.delete("test")
        assert not reg.exists("test")

    def test_delete_missing_function(self):
        """Test deleting missing function returns False."""
        reg = FunctionRegistry()
        assert not reg.delete("nonexistent")

    def test_delete_reduces_count(self):
        """Test delete reduces function count."""
        reg = FunctionRegistry()
        reg.define("func1")
        reg.define("func2")
        assert reg.count() == 2
        reg.delete("func1")
        assert reg.count() == 1


class TestListFunctions:
    """Tests for list_all() and related methods."""

    def test_list_all_empty(self):
        """Test listing functions in empty registry."""
        reg = FunctionRegistry()
        assert reg.list_all() == []

    def test_list_all_returns_sorted(self):
        """Test list_all returns sorted function names."""
        reg = FunctionRegistry()
        reg.define("zebra")
        reg.define("alpha")
        reg.define("beta")
        assert reg.list_all() == ["alpha", "beta", "zebra"]

    def test_get_all(self):
        """Test get_all returns all function definitions."""
        reg = FunctionRegistry()
        reg.define("func1", body=["cmd1"])
        reg.define("func2", body=["cmd2"])
        all_funcs = reg.get_all()
        assert len(all_funcs) == 2
        assert isinstance(all_funcs["func1"], FunctionDefinition)
        assert isinstance(all_funcs["func2"], FunctionDefinition)

    def test_get_all_as_dict(self):
        """Test get_all_as_dict returns dictionary format."""
        reg = FunctionRegistry()
        reg.define("func1", params=["a"], body=["echo $a"])
        reg.define("func2", params=[], body=["ls"])
        all_dicts = reg.get_all_as_dict()
        assert all_dicts["func1"] == {"params": ["a"], "body": ["echo $a"], "is_ast": False}
        assert all_dicts["func2"] == {"params": [], "body": ["ls"], "is_ast": False}


class TestClearRegistry:
    """Tests for clear() method."""

    def test_clear_empty_registry(self):
        """Test clearing empty registry."""
        reg = FunctionRegistry()
        reg.clear()
        assert reg.count() == 0

    def test_clear_removes_all_functions(self):
        """Test clear removes all functions."""
        reg = FunctionRegistry()
        reg.define("func1")
        reg.define("func2")
        reg.define("func3")
        reg.clear()
        assert reg.count() == 0
        assert reg.list_all() == []


class TestCount:
    """Tests for count() and len()."""

    def test_count_empty(self):
        """Test count on empty registry."""
        reg = FunctionRegistry()
        assert reg.count() == 0

    def test_count_after_adding(self):
        """Test count increases after adding functions."""
        reg = FunctionRegistry()
        reg.define("func1")
        assert reg.count() == 1
        reg.define("func2")
        assert reg.count() == 2

    def test_len_operator(self):
        """Test len() operator."""
        reg = FunctionRegistry()
        assert len(reg) == 0
        reg.define("func1")
        reg.define("func2")
        assert len(reg) == 2


class TestRepr:
    """Tests for __repr__."""

    def test_repr_empty(self):
        """Test repr of empty registry."""
        reg = FunctionRegistry()
        repr_str = repr(reg)
        assert "FunctionRegistry" in repr_str
        assert "0 functions" in repr_str

    def test_repr_with_functions(self):
        """Test repr with functions."""
        reg = FunctionRegistry()
        reg.define("alpha")
        reg.define("beta")
        repr_str = repr(reg)
        assert "FunctionRegistry" in repr_str
        assert "2 functions" in repr_str
        assert "alpha" in repr_str
        assert "beta" in repr_str


class TestComplexScenarios:
    """Tests for complex usage scenarios."""

    def test_multiple_operations(self):
        """Test multiple registry operations."""
        reg = FunctionRegistry()

        # Define several functions
        reg.define("greet", params=["name"], body=["echo Hello, $name"])
        reg.define("add", params=["a", "b"], body=["echo $(($a + $b))"])
        reg.define("list", body=["ls -la"])

        assert reg.count() == 3

        # Get one as dict
        greet_dict = reg.get_as_dict("greet")
        assert greet_dict["params"] == ["name"]

        # Delete one
        reg.delete("add")
        assert reg.count() == 2
        assert "add" not in reg

        # List remaining
        remaining = reg.list_all()
        assert remaining == ["greet", "list"]

    def test_redefine_function(self):
        """Test redefining function updates definition."""
        reg = FunctionRegistry()
        reg.define("func", body=["old command"])
        assert reg.get("func").body == ["old command"]

        reg.define("func", body=["new command"])
        assert reg.get("func").body == ["new command"]
        assert reg.count() == 1  # Still just one function
