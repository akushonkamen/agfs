package mountablefs

import (
	"io"
	"sync"
	"testing"

	"github.com/c4pt0r/agfs/agfs-server/pkg/filesystem"
	"github.com/c4pt0r/agfs/agfs-server/pkg/plugin"
	"github.com/c4pt0r/agfs/agfs-server/pkg/plugin/api"
)

// MockPlugin implements plugin.ServicePlugin for testing
type MockPlugin struct {
	name string
}

func (p *MockPlugin) Name() string {
	return p.name
}

func (p *MockPlugin) Validate(cfg map[string]interface{}) error {
	return nil
}

func (p *MockPlugin) Initialize(cfg map[string]interface{}) error {
	return nil
}

func (p *MockPlugin) GetFileSystem() filesystem.FileSystem {
	return nil
}

func (p *MockPlugin) GetReadme() string {
	return "Mock Plugin"
}

func (p *MockPlugin) GetConfigParams() []plugin.ConfigParameter {
	return nil
}

func (p *MockPlugin) Shutdown() error {
	return nil
}

func TestMountableFSRouting(t *testing.T) {
	mfs := NewMountableFS(api.PoolConfig{})

	p1 := &MockPlugin{name: "plugin1"}
	p2 := &MockPlugin{name: "plugin2"}
	pRoot := &MockPlugin{name: "rootPlugin"}

	// Test 1: Basic Mount
	err := mfs.Mount("/data", p1)
	if err != nil {
		t.Fatalf("Failed to mount: %v", err)
	}

	// Test 2: Exact Match
	mount, relPath, found := mfs.findMount("/data")
	if !found {
		t.Errorf("Expected to find mount at /data")
	}
	if mount.Plugin != p1 {
		t.Errorf("Expected plugin1, got %s", mount.Plugin.Name())
	}
	if relPath != "/" {
		t.Errorf("Expected relPath /, got %s", relPath)
	}

	// Test 3: Subpath Match
	mount, relPath, found = mfs.findMount("/data/file.txt")
	if !found {
		t.Errorf("Expected to find mount at /data/file.txt")
	}
	if mount.Plugin != p1 {
		t.Errorf("Expected plugin1, got %s", mount.Plugin.Name())
	}
	if relPath != "/file.txt" {
		t.Errorf("Expected relPath /file.txt, got %s", relPath)
	}

	// Test 4: Partial Match (Should Fail)
	mount, _, found = mfs.findMount("/dataset")
	if found {
		t.Errorf("Should NOT find mount for /dataset (partial match of /data)")
	}

	// Test 5: Nested Mounts / Longest Prefix
	err = mfs.Mount("/data/users", p2)
	if err != nil {
		t.Fatalf("Failed to mount nested: %v", err)
	}

	// /data should still map to p1
	mount, _, found = mfs.findMount("/data/config")
	if !found || mount.Plugin != p1 {
		t.Errorf("Expected /data/config to map to plugin1")
	}

	// /data/users should map to p2
	mount, relPath, found = mfs.findMount("/data/users/alice")
	if !found {
		t.Errorf("Expected to find mount at /data/users/alice")
	}
	if mount.Plugin != p2 {
		t.Errorf("Expected plugin2, got %s", mount.Plugin.Name())
	}
	if relPath != "/alice" {
		t.Errorf("Expected relPath /alice, got %s", relPath)
	}

	// Test 6: Root Mount
	err = mfs.Mount("/", pRoot)
	if err != nil {
		t.Fatalf("Failed to mount root: %v", err)
	}

	// /other should map to root
	mount, relPath, found = mfs.findMount("/other/file")
	if !found {
		t.Errorf("Expected to find mount at /other/file")
	}
	if mount.Plugin != pRoot {
		t.Errorf("Expected rootPlugin, got %s", mount.Plugin.Name())
	}
	if relPath != "/other/file" {
		t.Errorf("Expected relPath /other/file, got %s", relPath)
	}

	// /data/users/alice should still map to p2 (longest match)
	mount, _, found = mfs.findMount("/data/users/alice")
	if !found || mount.Plugin != p2 {
		t.Errorf("Root mount broke specific mount routing")
	}

	// Test 7: Unmount
	err = mfs.Unmount("/data")
	if err != nil {
		t.Fatalf("Failed to unmount: %v", err)
	}

	// /data/file should now fall back to Root because /data is gone
	mount, _, found = mfs.findMount("/data/file")
	if !found {
		t.Errorf("Expected /data/file to be found (fallback to root)")
	}
	if mount.Plugin != pRoot {
		t.Errorf("Expected fallback to rootPlugin, got %s", mount.Plugin.Name())
	}

	// /data/users should still exist
	mount, _, found = mfs.findMount("/data/users/bob")
	if !found || mount.Plugin != p2 {
		t.Errorf("Unmounting parent should not affect child mount")
	}
}

// MockFS implements filesystem.FileSystem for testing symlink functionality
type MockFS struct {
	files map[string]*MockFile // path -> file content
	dirs  map[string]bool      // path -> isDir
	mu    sync.RWMutex
}

type MockFile struct {
	content []byte
	mode    uint32
}

func NewMockFS() *MockFS {
	return &MockFS{
		files: make(map[string]*MockFile),
		dirs:  make(map[string]bool),
	}
}

func (m *MockFS) Create(path string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	path = filesystem.NormalizePath(path)
	if _, exists := m.files[path]; exists {
		return filesystem.NewAlreadyExistsError("create", path)
	}
	m.files[path] = &MockFile{content: []byte{}, mode: 0644}
	return nil
}

func (m *MockFS) Mkdir(path string, perm uint32) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	path = filesystem.NormalizePath(path)
	if m.dirs[path] {
		return filesystem.NewAlreadyExistsError("mkdir", path)
	}
	m.dirs[path] = true
	return nil
}

func (m *MockFS) Remove(path string) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	path = filesystem.NormalizePath(path)
	if _, exists := m.files[path]; exists {
		delete(m.files, path)
		return nil
	}
	if m.dirs[path] {
		delete(m.dirs, path)
		return nil
	}
	return filesystem.NewNotFoundError("remove", path)
}

func (m *MockFS) RemoveAll(path string) error {
	return m.Remove(path)
}

func (m *MockFS) Read(path string, offset int64, size int64) ([]byte, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	path = filesystem.NormalizePath(path)
	file, exists := m.files[path]
	if !exists {
		return nil, filesystem.NewNotFoundError("read", path)
	}
	return file.content, nil
}

func (m *MockFS) Write(path string, data []byte, offset int64, flags filesystem.WriteFlag) (int64, error) {
	m.mu.Lock()
	defer m.mu.Unlock()

	path = filesystem.NormalizePath(path)
	if file, exists := m.files[path]; exists {
		file.content = data
		return int64(len(data)), nil
	}

	if flags&filesystem.WriteFlagCreate != 0 {
		m.files[path] = &MockFile{content: data, mode: 0644}
		return int64(len(data)), nil
	}

	return 0, filesystem.NewNotFoundError("write", path)
}

func (m *MockFS) ReadDir(path string) ([]filesystem.FileInfo, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	path = filesystem.NormalizePath(path)
	if !m.dirs[path] && path != "/" {
		return nil, filesystem.NewNotFoundError("readdir", path)
	}

	var infos []filesystem.FileInfo
	// Add files in this directory
	for p := range m.files {
		dir := filesystem.NormalizePath(p[:lastSlash(p)])
		if dir == path {
			name := p[lastSlash(p)+1:]
			infos = append(infos, filesystem.FileInfo{
				Name:  name,
				Size:  int64(len(m.files[p].content)),
				Mode:  m.files[p].mode,
				IsDir: false,
			})
		}
	}
	// Add subdirectories
	for d := range m.dirs {
		if d == path {
			continue
		}
		dir := filesystem.NormalizePath(d[:lastSlash(d)])
		if dir == path {
			name := d[lastSlash(d)+1:]
			infos = append(infos, filesystem.FileInfo{
				Name:  name,
				Size:  0,
				Mode:  0755,
				IsDir: true,
			})
		}
	}
	return infos, nil
}

func lastSlash(s string) int {
	for i := len(s) - 1; i >= 0; i-- {
		if s[i] == '/' {
			return i
		}
	}
	return -1
}

func (m *MockFS) Stat(path string) (*filesystem.FileInfo, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	path = filesystem.NormalizePath(path)

	if path == "/" {
		return &filesystem.FileInfo{
			Name:  "/",
			Size:  0,
			Mode:  0755,
			IsDir: true,
		}, nil
	}

	if file, exists := m.files[path]; exists {
		name := path[lastSlash(path)+1:]
		return &filesystem.FileInfo{
			Name:  name,
			Size:  int64(len(file.content)),
			Mode:  file.mode,
			IsDir: false,
		}, nil
	}

	if m.dirs[path] {
		name := path[lastSlash(path)+1:]
		return &filesystem.FileInfo{
			Name:  name,
			Size:  0,
			Mode:  0755,
			IsDir: true,
		}, nil
	}

	return nil, filesystem.NewNotFoundError("stat", path)
}

func (m *MockFS) Rename(oldPath, newPath string) error {
	return filesystem.NewNotSupportedError("rename", oldPath)
}

func (m *MockFS) Chmod(path string, mode uint32) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	path = filesystem.NormalizePath(path)
	if file, exists := m.files[path]; exists {
		file.mode = mode
		return nil
	}
	return filesystem.NewNotFoundError("chmod", path)
}

func (m *MockFS) Open(path string) (io.ReadCloser, error) {
	return nil, filesystem.NewNotSupportedError("open", path)
}

func (m *MockFS) OpenWrite(path string) (io.WriteCloser, error) {
	return nil, filesystem.NewNotSupportedError("openwrite", path)
}

// MockServicePlugin wraps a MockFS as a ServicePlugin
type MockServicePlugin struct {
	fs   *MockFS
	name string
}

func NewMockServicePlugin(name string) *MockServicePlugin {
	return &MockServicePlugin{
		fs:   NewMockFS(),
		name: name,
	}
}

func (p *MockServicePlugin) Name() string {
	return p.name
}

func (p *MockServicePlugin) Validate(cfg map[string]interface{}) error {
	return nil
}

func (p *MockServicePlugin) Initialize(cfg map[string]interface{}) error {
	return nil
}

func (p *MockServicePlugin) GetFileSystem() filesystem.FileSystem {
	return p.fs
}

func (p *MockServicePlugin) GetReadme() string {
	return "Mock Service Plugin"
}

func (p *MockServicePlugin) GetConfigParams() []plugin.ConfigParameter {
	return nil
}

func (p *MockServicePlugin) Shutdown() error {
	return nil
}

func TestSymlinkBasic(t *testing.T) {
	mfs := NewMountableFS(api.PoolConfig{})

	// Mount a mock filesystem
	mockPlugin := NewMockServicePlugin("mock")
	err := mfs.Mount("/mnt", mockPlugin)
	if err != nil {
		t.Fatalf("Failed to mount: %v", err)
	}

	// Create a test directory and file
	err = mockPlugin.fs.Mkdir("/testdir", 0755)
	if err != nil {
		t.Fatalf("Failed to create directory: %v", err)
	}

	_, err = mockPlugin.fs.Write("/testdir/file.txt", []byte("test content"), 0, filesystem.WriteFlagCreate)
	if err != nil {
		t.Fatalf("Failed to create file: %v", err)
	}

	// Test 1: Create symlink
	err = mfs.Symlink("/mnt/testdir/file.txt", "/mnt/link1")
	if err != nil {
		t.Fatalf("Failed to create symlink: %v", err)
	}

	// Test 2: Read symlink target
	target, err := mfs.Readlink("/mnt/link1")
	if err != nil {
		t.Fatalf("Failed to read symlink: %v", err)
	}
	if target != "/mnt/testdir/file.txt" {
		t.Errorf("Expected target /mnt/testdir/file.txt, got %s", target)
	}

	// Test 3: Access file through symlink
	data, err := mfs.Read("/mnt/link1", 0, -1)
	if err != nil {
		t.Fatalf("Failed to read through symlink: %v", err)
	}
	if string(data) != "test content" {
		t.Errorf("Expected 'test content', got %s", string(data))
	}

	// Test 4: Write through symlink
	_, err = mfs.Write("/mnt/link1", []byte("new content"), 0, filesystem.WriteFlagNone)
	if err != nil {
		t.Fatalf("Failed to write through symlink: %v", err)
	}

	// Verify content changed in original file
	data, err = mfs.Read("/mnt/testdir/file.txt", 0, -1)
	if err != nil {
		t.Fatalf("Failed to read original file: %v", err)
	}
	if string(data) != "new content" {
		t.Errorf("Expected 'new content', got %s", string(data))
	}

	// Test 5: Remove symlink
	err = mfs.Remove("/mnt/link1")
	if err != nil {
		t.Fatalf("Failed to remove symlink: %v", err)
	}

	// Verify symlink is gone
	_, err = mfs.Readlink("/mnt/link1")
	if err == nil {
		t.Errorf("Expected error when reading removed symlink")
	}

	// Verify original file still exists
	data, err = mfs.Read("/mnt/testdir/file.txt", 0, -1)
	if err != nil {
		t.Fatalf("Original file should still exist: %v", err)
	}
	if string(data) != "new content" {
		t.Errorf("Original file content should be unchanged")
	}
}

func TestSymlinkRelativePath(t *testing.T) {
	mfs := NewMountableFS(api.PoolConfig{})

	mockPlugin := NewMockServicePlugin("mock")
	err := mfs.Mount("/mnt", mockPlugin)
	if err != nil {
		t.Fatalf("Failed to mount: %v", err)
	}

	// Create directory structure
	err = mockPlugin.fs.Mkdir("/dir1", 0755)
	if err != nil {
		t.Fatalf("Failed to create dir1: %v", err)
	}

	err = mockPlugin.fs.Mkdir("/dir2", 0755)
	if err != nil {
		t.Fatalf("Failed to create dir2: %v", err)
	}

	_, err = mockPlugin.fs.Write("/dir1/file.txt", []byte("content"), 0, filesystem.WriteFlagCreate)
	if err != nil {
		t.Fatalf("Failed to create file: %v", err)
	}

	// Create relative symlink from /mnt/dir2/link to ../dir1/file.txt
	err = mfs.Symlink("../dir1/file.txt", "/mnt/dir2/link")
	if err != nil {
		t.Fatalf("Failed to create relative symlink: %v", err)
	}

	// Read through relative symlink
	data, err := mfs.Read("/mnt/dir2/link", 0, -1)
	if err != nil {
		t.Fatalf("Failed to read through relative symlink: %v", err)
	}
	if string(data) != "content" {
		t.Errorf("Expected 'content', got %s", string(data))
	}

	// Verify the target stored is relative
	target, err := mfs.Readlink("/mnt/dir2/link")
	if err != nil {
		t.Fatalf("Failed to read symlink target: %v", err)
	}
	if target != "../dir1/file.txt" {
		t.Errorf("Expected relative target '../dir1/file.txt', got %s", target)
	}
}

func TestSymlinkErrors(t *testing.T) {
	mfs := NewMountableFS(api.PoolConfig{})

	mockPlugin := NewMockServicePlugin("mock")
	err := mfs.Mount("/mnt", mockPlugin)
	if err != nil {
		t.Fatalf("Failed to mount: %v", err)
	}

	// Create a test file
	_, err = mockPlugin.fs.Write("/file.txt", []byte("content"), 0, filesystem.WriteFlagCreate)
	if err != nil {
		t.Fatalf("Failed to create file: %v", err)
	}

	// Test 1: Create symlink where target file exists
	err = mfs.Symlink("/mnt/target", "/mnt/file.txt")
	if err == nil {
		t.Errorf("Expected error when creating symlink at existing file path")
	}

	// Test 2: Create symlink in non-existent parent directory
	err = mfs.Symlink("/mnt/target", "/mnt/nonexistent/link")
	if err == nil {
		t.Errorf("Expected error when parent directory doesn't exist")
	}

	// Test 3: Create duplicate symlink
	err = mockPlugin.fs.Mkdir("/dir", 0755)
	if err != nil {
		t.Fatalf("Failed to create directory: %v", err)
	}

	err = mfs.Symlink("/mnt/file.txt", "/mnt/dir/link1")
	if err != nil {
		t.Fatalf("Failed to create first symlink: %v", err)
	}

	err = mfs.Symlink("/mnt/file.txt", "/mnt/dir/link1")
	if err == nil {
		t.Errorf("Expected error when creating duplicate symlink")
	}

	// Test 4: Read non-existent symlink
	_, err = mfs.Readlink("/mnt/nonexistent")
	if err == nil {
		t.Errorf("Expected error when reading non-existent symlink")
	}
}

func TestSymlinkChain(t *testing.T) {
	mfs := NewMountableFS(api.PoolConfig{})

	mockPlugin := NewMockServicePlugin("mock")
	err := mfs.Mount("/mnt", mockPlugin)
	if err != nil {
		t.Fatalf("Failed to mount: %v", err)
	}

	// Create a file
	_, err = mockPlugin.fs.Write("/file.txt", []byte("original"), 0, filesystem.WriteFlagCreate)
	if err != nil {
		t.Fatalf("Failed to create file: %v", err)
	}

	// Create a chain of symlinks: link1 -> link2 -> link3 -> file.txt
	err = mfs.Symlink("/mnt/file.txt", "/mnt/link3")
	if err != nil {
		t.Fatalf("Failed to create link3: %v", err)
	}

	err = mfs.Symlink("/mnt/link3", "/mnt/link2")
	if err != nil {
		t.Fatalf("Failed to create link2: %v", err)
	}

	err = mfs.Symlink("/mnt/link2", "/mnt/link1")
	if err != nil {
		t.Fatalf("Failed to create link1: %v", err)
	}

	// Read through the chain
	data, err := mfs.Read("/mnt/link1", 0, -1)
	if err != nil {
		t.Fatalf("Failed to read through symlink chain: %v", err)
	}
	if string(data) != "original" {
		t.Errorf("Expected 'original', got %s", string(data))
	}

	// Test circular symlink detection
	// Remove link3 and recreate it pointing to link1 to form a cycle
	err = mfs.Remove("/mnt/link3")
	if err != nil {
		t.Fatalf("Failed to remove link3: %v", err)
	}

	err = mfs.Symlink("/mnt/link1", "/mnt/link3")
	if err != nil {
		t.Fatalf("Failed to create circular symlink: %v", err)
	}

	// Now we have a cycle: link1 -> link2 -> link3 -> link1
	// Reading should fail
	_, err = mfs.Read("/mnt/link1", 0, -1)
	if err == nil {
		t.Errorf("Expected error when reading circular symlink")
	}
	if err != nil && err.Error() != "too many levels of symbolic links" {
		t.Logf("Got expected error: %v", err)
	}
}

func TestSymlinkCrossMount(t *testing.T) {
	mfs := NewMountableFS(api.PoolConfig{})

	// Mount two different filesystems
	plugin1 := NewMockServicePlugin("plugin1")
	plugin2 := NewMockServicePlugin("plugin2")

	err := mfs.Mount("/mnt1", plugin1)
	if err != nil {
		t.Fatalf("Failed to mount plugin1: %v", err)
	}

	err = mfs.Mount("/mnt2", plugin2)
	if err != nil {
		t.Fatalf("Failed to mount plugin2: %v", err)
	}

	// Create a file in plugin1
	_, err = plugin1.fs.Write("/file.txt", []byte("from plugin1"), 0, filesystem.WriteFlagCreate)
	if err != nil {
		t.Fatalf("Failed to create file in plugin1: %v", err)
	}

	// Create directory in plugin2
	err = plugin2.fs.Mkdir("/links", 0755)
	if err != nil {
		t.Fatalf("Failed to create directory in plugin2: %v", err)
	}

	// Create symlink in plugin2 pointing to file in plugin1
	err = mfs.Symlink("/mnt1/file.txt", "/mnt2/links/cross_link")
	if err != nil {
		t.Fatalf("Failed to create cross-mount symlink: %v", err)
	}

	// Read through cross-mount symlink
	data, err := mfs.Read("/mnt2/links/cross_link", 0, -1)
	if err != nil {
		t.Fatalf("Failed to read through cross-mount symlink: %v", err)
	}
	if string(data) != "from plugin1" {
		t.Errorf("Expected 'from plugin1', got %s", string(data))
	}
}

func TestSymlinkVisibility(t *testing.T) {
	mfs := NewMountableFS(api.PoolConfig{})

	mockPlugin := NewMockServicePlugin("mock")
	err := mfs.Mount("/mnt", mockPlugin)
	if err != nil {
		t.Fatalf("Failed to mount: %v", err)
	}

	// Create directory and file
	err = mockPlugin.fs.Mkdir("/dir", 0755)
	if err != nil {
		t.Fatalf("Failed to create directory: %v", err)
	}

	_, err = mockPlugin.fs.Write("/dir/file.txt", []byte("content"), 0, filesystem.WriteFlagCreate)
	if err != nil {
		t.Fatalf("Failed to create file: %v", err)
	}

	// Create symlink
	err = mfs.Symlink("/mnt/dir/file.txt", "/mnt/dir/link")
	if err != nil {
		t.Fatalf("Failed to create symlink: %v", err)
	}

	// Test 1: ReadDir should show the symlink
	infos, err := mfs.ReadDir("/mnt/dir")
	if err != nil {
		t.Fatalf("Failed to read directory: %v", err)
	}

	foundFile := false
	foundLink := false
	for _, info := range infos {
		if info.Name == "file.txt" {
			foundFile = true
			if info.Meta.Type == "symlink" {
				t.Errorf("file.txt should not be a symlink")
			}
		}
		if info.Name == "link" {
			foundLink = true
			if info.Meta.Type != "symlink" {
				t.Errorf("link should be a symlink, got type: %s", info.Meta.Type)
			}
		}
	}

	if !foundFile {
		t.Errorf("file.txt not found in directory listing")
	}
	if !foundLink {
		t.Errorf("link not found in directory listing")
	}

	// Test 2: Stat on symlink should return symlink info
	linkStat, err := mfs.Stat("/mnt/dir/link")
	if err != nil {
		t.Fatalf("Failed to stat symlink: %v", err)
	}

	if linkStat.Meta.Type != "symlink" {
		t.Errorf("Stat should identify link as symlink, got type: %s", linkStat.Meta.Type)
	}
	if linkStat.Name != "link" {
		t.Errorf("Expected name 'link', got %s", linkStat.Name)
	}

	// Test 3: Read through symlink should work
	data, err := mfs.Read("/mnt/dir/link", 0, -1)
	if err != nil {
		t.Fatalf("Failed to read through symlink: %v", err)
	}
	if string(data) != "content" {
		t.Errorf("Expected 'content', got %s", string(data))
	}

	// Test 4: Readlink should work
	target, err := mfs.Readlink("/mnt/dir/link")
	if err != nil {
		t.Fatalf("Failed to readlink: %v", err)
	}
	if target != "/mnt/dir/file.txt" {
		t.Errorf("Expected target '/mnt/dir/file.txt', got %s", target)
	}
}

func TestSymlinkToDirectory(t *testing.T) {
	mfs := NewMountableFS(api.PoolConfig{})

	mockPlugin := NewMockServicePlugin("mock")
	err := mfs.Mount("/mnt", mockPlugin)
	if err != nil {
		t.Fatalf("Failed to mount: %v", err)
	}

	// Create directory with a file inside
	err = mockPlugin.fs.Mkdir("/realdir", 0755)
	if err != nil {
		t.Fatalf("Failed to create directory: %v", err)
	}

	_, err = mockPlugin.fs.Write("/realdir/file.txt", []byte("test"), 0, filesystem.WriteFlagCreate)
	if err != nil {
		t.Fatalf("Failed to create file: %v", err)
	}

	// Create symlink to directory
	err = mfs.Symlink("/mnt/realdir", "/mnt/linkdir")
	if err != nil {
		t.Fatalf("Failed to create symlink to directory: %v", err)
	}

	// Test 1: Stat should show it as a directory symlink
	linkStat, err := mfs.Stat("/mnt/linkdir")
	if err != nil {
		t.Fatalf("Failed to stat symlink: %v", err)
	}

	if linkStat.Meta.Type != "symlink" {
		t.Errorf("Should be identified as symlink, got: %s", linkStat.Meta.Type)
	}
	if !linkStat.IsDir {
		t.Errorf("Symlink to directory should report IsDir=true")
	}

	// Test 2: ReadDir through symlink should work
	infos, err := mfs.ReadDir("/mnt/linkdir")
	if err != nil {
		t.Fatalf("Failed to read through symlink directory: %v", err)
	}

	found := false
	for _, info := range infos {
		if info.Name == "file.txt" {
			found = true
		}
	}
	if !found {
		t.Errorf("file.txt not found when reading through symlink directory")
	}

	// Test 3: Access file through symlink directory
	data, err := mfs.Read("/mnt/linkdir/file.txt", 0, -1)
	if err != nil {
		t.Fatalf("Failed to read file through symlink directory: %v", err)
	}
	if string(data) != "test" {
		t.Errorf("Expected 'test', got %s", string(data))
	}
}
