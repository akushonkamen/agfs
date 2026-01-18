"""Unit tests for PathManager component."""

from agfs_shell.path_manager import PathManager


class TestPathManagerCreation:
    """Tests for PathManager initialization."""

    def test_default_creation(self):
        """Test creating path manager with defaults."""
        pm = PathManager()
        assert pm.cwd == "/"
        assert pm.chroot_root is None

    def test_creation_with_custom_cwd(self):
        """Test creating with custom initial cwd."""
        pm = PathManager(initial_cwd="/home/user")
        assert pm.cwd == "/home/user"

    def test_creation_with_chroot(self):
        """Test creating with chroot."""
        pm = PathManager(chroot_root="/var/chroot")
        assert pm.chroot_root == "/var/chroot"


class TestPathResolution:
    """Tests for path resolution."""

    def test_resolve_absolute_path(self):
        """Test resolving absolute path without chroot."""
        pm = PathManager(initial_cwd="/home/user")
        assert pm.resolve_path("/etc/config") == "/etc/config"

    def test_resolve_relative_path(self):
        """Test resolving relative path."""
        pm = PathManager(initial_cwd="/home/user")
        assert pm.resolve_path("documents") == "/home/user/documents"

    def test_resolve_parent_directory(self):
        """Test resolving path with parent directory (..)."""
        pm = PathManager(initial_cwd="/home/user/docs")
        assert pm.resolve_path("..") == "/home/user"
        assert pm.resolve_path("../other") == "/home/user/other"

    def test_resolve_current_directory(self):
        """Test resolving current directory (.)."""
        pm = PathManager(initial_cwd="/home/user")
        assert pm.resolve_path(".") == "/home/user"
        assert pm.resolve_path("./file.txt") == "/home/user/file.txt"

    def test_resolve_empty_path(self):
        """Test resolving empty path returns cwd."""
        pm = PathManager(initial_cwd="/home/user")
        assert pm.resolve_path("") == "/home/user"

    def test_resolve_normalizes_path(self):
        """Test path resolution normalizes paths."""
        pm = PathManager(initial_cwd="/home")
        assert pm.resolve_path("/foo//bar/../baz") == "/foo/baz"


class TestChrootPathResolution:
    """Tests for path resolution with chroot."""

    def test_resolve_absolute_with_chroot(self):
        """Test resolving absolute path with chroot."""
        pm = PathManager(chroot_root="/var/chroot")
        assert pm.resolve_path("/etc/config") == "/var/chroot/etc/config"

    def test_resolve_relative_with_chroot(self):
        """Test resolving relative path with chroot."""
        pm = PathManager(initial_cwd="/home", chroot_root="/var/chroot")
        assert pm.resolve_path("user") == "/var/chroot/home/user"

    def test_chroot_prevents_escape(self):
        """Test chroot prevents escaping to parent directories."""
        pm = PathManager(initial_cwd="/home", chroot_root="/var/chroot")
        # Try to escape with ../.. should stay at chroot root
        result = pm.resolve_path("../../etc")
        assert result == "/var/chroot/etc"

    def test_chroot_normalizes_to_root(self):
        """Test excessive parent refs normalize to chroot root."""
        pm = PathManager(chroot_root="/var/chroot")
        # Going up from / should stay at /
        result = pm.resolve_path("/../../../etc")
        assert result == "/var/chroot/etc"


class TestChangeDirectory:
    """Tests for change_directory."""

    def test_change_to_absolute_path(self):
        """Test changing to absolute path."""
        pm = PathManager(initial_cwd="/home/user")
        pm.change_directory("/etc")
        assert pm.cwd == "/etc"

    def test_change_to_relative_path(self):
        """Test changing to relative path."""
        pm = PathManager(initial_cwd="/home/user")
        pm.change_directory("documents")
        assert pm.cwd == "/home/user/documents"

    def test_change_to_parent_directory(self):
        """Test changing to parent directory."""
        pm = PathManager(initial_cwd="/home/user/docs")
        pm.change_directory("..")
        assert pm.cwd == "/home/user"

    def test_change_normalizes_path(self):
        """Test change_directory normalizes paths."""
        pm = PathManager(initial_cwd="/home")
        pm.change_directory("user/../other")
        assert pm.cwd == "/home/other"

    def test_change_with_chroot(self):
        """Test changing directory with chroot."""
        pm = PathManager(chroot_root="/var/chroot")
        pm.change_directory("/home/user")
        assert pm.cwd == "/home/user"  # Virtual path

    def test_change_with_chroot_prevents_escape(self):
        """Test changing directory with chroot prevents escape."""
        pm = PathManager(initial_cwd="/home", chroot_root="/var/chroot")
        pm.change_directory("../../etc")
        # Should be normalized to /etc (can't escape chroot)
        assert pm.cwd == "/etc"


class TestGetCwd:
    """Tests for get_cwd."""

    def test_get_cwd(self):
        """Test getting current working directory."""
        pm = PathManager(initial_cwd="/home/user")
        assert pm.get_cwd() == "/home/user"

    def test_get_cwd_after_change(self):
        """Test getting cwd after changing directory."""
        pm = PathManager()
        pm.change_directory("/tmp")
        assert pm.get_cwd() == "/tmp"


class TestGetRealPath:
    """Tests for get_real_path."""

    def test_get_real_path_without_chroot(self):
        """Test getting real path without chroot."""
        pm = PathManager()
        assert pm.get_real_path("/etc/config") == "/etc/config"

    def test_get_real_path_with_chroot(self):
        """Test getting real path with chroot."""
        pm = PathManager(chroot_root="/var/chroot")
        assert pm.get_real_path("/etc/config") == "/var/chroot/etc/config"

    def test_get_real_path_relative(self):
        """Test getting real path for relative path."""
        pm = PathManager(initial_cwd="/home", chroot_root="/var/chroot")
        assert pm.get_real_path("user") == "/var/chroot/home/user"


class TestChrootManagement:
    """Tests for chroot management."""

    def test_set_chroot(self):
        """Test setting chroot."""
        pm = PathManager()
        pm.set_chroot("/var/chroot")
        assert pm.chroot_root == "/var/chroot"
        assert pm.cwd == "/"  # Should reset to root

    def test_clear_chroot(self):
        """Test clearing chroot."""
        pm = PathManager(chroot_root="/var/chroot")
        pm.set_chroot(None)
        assert pm.chroot_root is None

    def test_is_chrooted(self):
        """Test checking if chrooted."""
        pm = PathManager()
        assert not pm.is_chrooted()

        pm.set_chroot("/var/chroot")
        assert pm.is_chrooted()

        pm.set_chroot(None)
        assert not pm.is_chrooted()


class TestNormalizePath:
    """Tests for normalize_path."""

    def test_normalize_simple_path(self):
        """Test normalizing simple path."""
        pm = PathManager()
        assert pm.normalize_path("/foo/bar") == "/foo/bar"

    def test_normalize_path_with_dots(self):
        """Test normalizing path with . and ..."""
        pm = PathManager()
        assert pm.normalize_path("/foo/./bar/../baz") == "/foo/baz"

    def test_normalize_preserves_leading_slash(self):
        """Test normalize preserves leading slash for absolute paths."""
        pm = PathManager()
        # normpath might remove leading / in some cases, we ensure it's there
        result = pm.normalize_path("/foo")
        assert result.startswith("/")

    def test_normalize_relative_path(self):
        """Test normalizing relative path."""
        pm = PathManager()
        assert pm.normalize_path("foo/bar") == "foo/bar"


class TestEdgeCases:
    """Tests for edge cases."""

    def test_multiple_slashes(self):
        """Test handling multiple consecutive slashes."""
        pm = PathManager()
        assert pm.resolve_path("/foo//bar///baz") == "/foo/bar/baz"

    def test_trailing_slash(self):
        """Test handling trailing slash."""
        pm = PathManager()
        # normpath removes trailing slashes
        result = pm.resolve_path("/foo/bar/")
        assert result == "/foo/bar"

    def test_root_directory(self):
        """Test handling root directory."""
        pm = PathManager()
        assert pm.resolve_path("/") == "/"

    def test_change_to_root(self):
        """Test changing to root directory."""
        pm = PathManager(initial_cwd="/home/user")
        pm.change_directory("/")
        assert pm.cwd == "/"
