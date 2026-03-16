//! CtxFS Python Bindings using PyO3
//!
//! This module provides Python bindings for the CtxFS client library.
//!
//! # Example
//!
//! ```python
//! import ctxfs
//!
//! # Create a client
//! client = ctxfs.Client("http://localhost:8080")
//!
//! # Write a file
//! client.write("/memfs/test.txt", b"Hello, World!")
//!
//! # Read a file
//! content = client.read("/memfs/test.txt")
//!
//! # List directory
//! files = client.list("/memfs")
//! ```

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyBytes};
use std::sync::Arc;
use tokio::runtime::Runtime;

use ctxfs_sdk::Client;

/// CtxFS Python Client
///
/// A Python wrapper around the CtxFS HTTP client that provides
/// file system operations through a simple API.
#[pyclass(name = "Client")]
pub struct PyClient {
    /// The underlying CtxFS client
    client: Arc<Client>,
    /// Tokio runtime for async operations
    rt: Arc<Runtime>,
}

impl PyClient {
    /// Create a new PyClient with the given base URL
    fn new_with_url(base_url: String) -> PyResult<(Arc<Client>, Arc<Runtime>)> {
        let rt = Arc::new(
            tokio::runtime::Runtime::new()
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to create runtime: {}", e)
                ))?
        );

        // Client::new is synchronous, so we can call it directly
        let client = Client::new(&base_url)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to create client: {}", e)
            ))?;

        Ok((Arc::new(client), rt))
    }
}

#[pymethods]
impl PyClient {
    /// Create a new CtxFS client
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL of the CtxFS server (default: "http://localhost:8080")
    ///
    /// # Returns
    ///
    /// A new Client instance
    ///
    /// # Example
    ///
    /// ```python
    /// client = ctxfs.Client("http://localhost:8080")
    /// ```
    #[new]
    #[pyo3(signature = (base_url = "http://localhost:8080"))]
    pub fn py_new(base_url: &str) -> PyResult<Self> {
        Self::new_with_url(base_url.to_string())
            .map(|(client, rt)| PyClient { client, rt })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e))
    }

    /// Get the base URL of this client
    ///
    /// # Returns
    ///
    /// The base URL string
    ///
    /// # Example
    ///
    /// ```python
    /// client = ctxfs.Client("http://localhost:8080")
    /// print(client.base_url())  # "http://localhost:8080/api/v1"
    /// ```
    #[getter]
    pub fn base_url(&self) -> String {
        self.client.base_url().to_string()
    }

    /// Read a file
    ///
    /// # Arguments
    ///
    /// * `path` - The file path to read
    /// * `offset` - Starting position in bytes (default: 0)
    /// * `size` - Number of bytes to read, -1 for all (default: -1)
    ///
    /// # Returns
    ///
    /// The file content as bytes
    ///
    /// # Example
    ///
    /// ```python
    /// content = client.read("/memfs/test.txt")
    /// # or with offset
    /// content = client.read("/memfs/test.txt", offset=100, size=50)
    /// ```
    #[pyo3(signature = (path, offset=0, size=-1))]
    pub fn read<'py>(
        &'py self,
        py: Python<'py>,
        path: &str,
        offset: i64,
        size: i64,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let path = path.to_string();

        // First do the async operation outside of GIL
        let data = py.allow_threads(|| {
            rt.block_on(async move {
                client.read(&path, offset, size).await
            })
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

        // Then create PyBytes with GIL held
        Ok(PyBytes::new_bound(py, &data))
    }

    /// Write to a file
    ///
    /// # Arguments
    ///
    /// * `path` - The file path to write to
    /// * `data` - The data to write (bytes)
    /// * `append` - Whether to append to the file (default: False)
    ///
    /// # Returns
    ///
    /// The number of bytes written
    ///
    /// # Example
    ///
    /// ```python
    /// client.write("/memfs/test.txt", b"Hello, World!")
    /// # or append
    /// client.write("/memfs/test.txt", b"More data", append=True)
    /// ```
    #[pyo3(signature = (path, data, append=false))]
    pub fn write(&self, py: Python, path: &str, data: &[u8], append: bool) -> PyResult<usize> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let path = path.to_string();
        let data = data.to_vec();
        let data_len = data.len();

        py.allow_threads(|| {
            rt.block_on(async move {
                if append {
                    // For append, we use write_with_flags with APPEND flag
                    client.write_with_flags(&path, &data, 0, ctxfs_sdk::WriteFlag::APPEND | ctxfs_sdk::WriteFlag::CREATE).await
                } else {
                    // Default write (creates or truncates)
                    client.write(&path, &data).await
                }
            })
            .map(|_| data_len)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
        })
    }

    /// Create a new empty file
    ///
    /// # Arguments
    ///
    /// * `path` - The path where the file should be created
    ///
    /// # Example
    ///
    /// ```python
    /// client.create("/memfs/newfile.txt")
    /// ```
    pub fn create(&self, py: Python, path: &str) -> PyResult<()> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let path = path.to_string();

        py.allow_threads(|| {
            rt.block_on(async move {
                client.create(&path).await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
        })
    }

    /// Create a directory
    ///
    /// # Arguments
    ///
    /// * `path` - The directory path to create
    /// * `perm` - Permission mode (e.g., 0o755 for rwxr-xr-x, default: 0o755)
    ///
    /// # Example
    ///
    /// ```python
    /// client.mkdir("/memfs/subdir", perm=0o755)
    /// ```
    #[pyo3(signature = (path, perm=0o755))]
    pub fn mkdir(&self, py: Python, path: &str, perm: u32) -> PyResult<()> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let path = path.to_string();

        py.allow_threads(|| {
            rt.block_on(async move {
                client.mkdir(&path, perm).await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
        })
    }

    /// Remove a file or empty directory
    ///
    /// # Arguments
    ///
    /// * `path` - The path to remove
    ///
    /// # Example
    ///
    /// ```python
    /// client.remove("/memfs/file.txt")
    /// ```
    pub fn remove(&self, py: Python, path: &str) -> PyResult<()> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let path = path.to_string();

        py.allow_threads(|| {
            rt.block_on(async move {
                client.remove_one(&path).await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
        })
    }

    /// Remove a file or directory recursively
    ///
    /// # Arguments
    ///
    /// * `path` - The path to remove
    ///
    /// # Example
    ///
    /// ```python
    /// client.remove_all("/memfs/directory")
    /// ```
    pub fn remove_all(&self, py: Python, path: &str) -> PyResult<()> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let path = path.to_string();

        py.allow_threads(|| {
            rt.block_on(async move {
                client.remove_all(&path).await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
        })
    }

    /// List directory contents
    ///
    /// # Arguments
    ///
    /// * `path` - The directory path to list
    ///
    /// # Returns
    ///
    /// A list of dictionaries, each containing:
    /// - name: str - The file/directory name
    /// - size: int - The size in bytes
    /// - is_dir: bool - Whether it's a directory
    /// - is_symlink: bool - Whether it's a symbolic link
    /// - mode: int - Permission mode
    /// - mod_time: str - Modification time (RFC3339)
    ///
    /// # Example
    ///
    /// ```python
    /// files = client.list("/memfs")
    /// for f in files:
    ///     print(f"{f['name']} - {f['size']} bytes")
    /// ```
    pub fn list<'py>(&'py self, py: Python<'py>, path: &str) -> PyResult<Bound<'py, PyAny>> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let path = path.to_string();

        // First do the async operation outside of GIL
        let files = py.allow_threads(|| {
            rt.block_on(async move {
                client.read_dir(&path).await
            })
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

        // Then build the Python objects with GIL held
        let list = PyList::empty_bound(py);
        for file in files {
            let dict = PyDict::new_bound(py);
            dict.set_item("name", file.name)?;
            dict.set_item("size", file.size)?;
            dict.set_item("is_dir", file.is_dir)?;
            dict.set_item("is_symlink", file.is_symlink)?;
            dict.set_item("mode", file.mode)?;
            dict.set_item("mod_time", file.mod_time.to_rfc3339())?;
            list.append(dict)?;
        }
        Ok(list.into_any())
    }

    /// Get file statistics
    ///
    /// # Arguments
    ///
    /// * `path` - The file/directory path
    ///
    /// # Returns
    ///
    /// A dictionary containing:
    /// - name: str - The file/directory name
    /// - size: int - The size in bytes
    /// - is_dir: bool - Whether it's a directory
    /// - is_symlink: bool - Whether it's a symbolic link
    /// - mode: int - Permission mode
    /// - mod_time: str - Modification time (RFC3339)
    ///
    /// # Example
    ///
    /// ```python
    /// info = client.stat("/memfs/file.txt")
    /// print(f"Size: {info['size']} bytes")
    /// ```
    pub fn stat<'py>(&'py self, py: Python<'py>, path: &str) -> PyResult<Bound<'py, PyDict>> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let path = path.to_string();

        // First do the async operation outside of GIL
        let file = py.allow_threads(|| {
            rt.block_on(async move {
                client.stat(&path).await
            })
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

        // Then build the Python dict with GIL held
        let dict = PyDict::new_bound(py);
        dict.set_item("name", file.name)?;
        dict.set_item("size", file.size)?;
        dict.set_item("is_dir", file.is_dir)?;
        dict.set_item("is_symlink", file.is_symlink)?;
        dict.set_item("mode", file.mode)?;
        dict.set_item("mod_time", file.mod_time.to_rfc3339())?;
        Ok(dict)
    }

    /// Rename/move a file or directory
    ///
    /// # Arguments
    ///
    /// * `old_path` - The current path
    /// * `new_path` - The new path
    ///
    /// # Example
    ///
    /// ```python
    /// client.rename("/memfs/old.txt", "/memfs/new.txt")
    /// ```
    pub fn rename(&self, py: Python, old_path: &str, new_path: &str) -> PyResult<()> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let old_path = old_path.to_string();
        let new_path = new_path.to_string();

        py.allow_threads(|| {
            rt.block_on(async move {
                client.rename(&old_path, &new_path).await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
        })
    }

    /// Search for files using a pattern
    ///
    /// # Arguments
    ///
    /// * `path` - The directory path to search in
    /// * `pattern` - The regex pattern to search for
    /// * `recursive` - Whether to search recursively (default: True)
    /// * `case_insensitive` - Whether to perform case-insensitive matching (default: False)
    ///
    /// # Returns
    ///
    /// A dictionary with:
    /// - matches: list of dicts with 'file', 'line', 'content' keys
    /// - count: total number of matches
    ///
    /// # Example
    ///
    /// ```python
    /// result = client.grep("/memfs", r"TODO", recursive=True)
    /// print(f"Found {result['count']} matches")
    /// for match in result['matches']:
    ///     print(f"{match['file']}:{match['line']}: {match['content']}")
    /// ```
    #[pyo3(signature = (path, pattern, recursive=true, case_insensitive=false))]
    pub fn grep<'py>(
        &'py self,
        py: Python<'py>,
        path: &str,
        pattern: &str,
        recursive: bool,
        case_insensitive: bool,
    ) -> PyResult<Bound<'py, PyDict>> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let path = path.to_string();
        let pattern = pattern.to_string();

        // First do the async operation outside of GIL
        let response = py.allow_threads(|| {
            rt.block_on(async move {
                client.grep(&path, &pattern, recursive, case_insensitive).await
            })
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

        // Then build the Python dict with GIL held
        let dict = PyDict::new_bound(py);
        let matches_list = PyList::empty_bound(py);

        for m in response.matches {
            let match_dict = PyDict::new_bound(py);
            match_dict.set_item("file", m.file)?;
            match_dict.set_item("line", m.line)?;
            match_dict.set_item("content", m.content)?;
            matches_list.append(match_dict)?;
        }

        dict.set_item("matches", matches_list)?;
        dict.set_item("count", response.count)?;
        Ok(dict)
    }

    /// Calculate file digest/hash
    ///
    /// # Arguments
    ///
    /// * `path` - The file path
    /// * `algorithm` - Hash algorithm: "xxh3" (default) or "md5"
    ///
    /// # Returns
    ///
    /// A dictionary with:
    /// - algorithm: str - The algorithm used
    /// - path: str - The file path
    /// - digest: str - Hex-encoded digest
    ///
    /// # Example
    ///
    /// ```python
    /// result = client.digest("/memfs/file.txt", algorithm="xxh3")
    /// print(f"Hash: {result['digest']}")
    /// ```
    #[pyo3(signature = (path, algorithm="xxh3"))]
    pub fn digest<'py>(
        &'py self,
        py: Python<'py>,
        path: &str,
        algorithm: &str,
    ) -> PyResult<Bound<'py, PyDict>> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let path = path.to_string();

        // First do the async operation outside of GIL
        let response = py.allow_threads(|| {
            rt.block_on(async move {
                client.digest(&path, algorithm).await
            })
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

        // Then build the Python dict with GIL held
        let dict = PyDict::new_bound(py);
        dict.set_item("algorithm", response.algorithm)?;
        dict.set_item("path", response.path)?;
        dict.set_item("digest", response.digest)?;
        Ok(dict)
    }

    /// Create a symbolic link
    ///
    /// # Arguments
    ///
    /// * `target` - The target path the link will point to
    /// * `link` - The link path to create
    ///
    /// # Example
    ///
    /// ```python
    /// client.symlink("/memfs/target.txt", "/memfs/link.txt")
    /// ```
    pub fn symlink(&self, py: Python, target: &str, link: &str) -> PyResult<()> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let target = target.to_string();
        let link = link.to_string();

        py.allow_threads(|| {
            rt.block_on(async move {
                client.symlink(&target, &link).await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
        })
    }

    /// Read symbolic link target
    ///
    /// # Arguments
    ///
    /// * `link` - The symbolic link path
    ///
    /// # Returns
    ///
    /// The target path the symlink points to
    ///
    /// # Example
    ///
    /// ```python
    /// target = client.readlink("/memfs/link.txt")
    /// print(f"Link points to: {target}")
    /// ```
    pub fn readlink(&self, py: Python, link: &str) -> PyResult<String> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let link = link.to_string();

        py.allow_threads(|| {
            rt.block_on(async move {
                client.readlink(&link).await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
        })
    }

    /// Change file permissions
    ///
    /// # Arguments
    ///
    /// * `path` - The file/directory path
    /// * `mode` - Permission mode (e.g., 0o644)
    ///
    /// # Example
    ///
    /// ```python
    /// client.chmod("/memfs/file.txt", 0o644)
    /// ```
    pub fn chmod(&self, py: Python, path: &str, mode: u32) -> PyResult<()> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let path = path.to_string();

        py.allow_threads(|| {
            rt.block_on(async move {
                client.chmod(&path, mode).await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
        })
    }

    /// Touch a file (update modification time)
    ///
    /// # Arguments
    ///
    /// * `path` - The file path
    ///
    /// # Example
    ///
    /// ```python
    /// client.touch("/memfs/file.txt")
    /// ```
    pub fn touch(&self, py: Python, path: &str) -> PyResult<()> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let path = path.to_string();

        py.allow_threads(|| {
            rt.block_on(async move {
                client.touch(&path).await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
        })
    }

    /// Truncate a file to a specific size
    ///
    /// # Arguments
    ///
    /// * `path` - The file path
    /// * `size` - The new size in bytes
    ///
    /// # Example
    ///
    /// ```python
    /// client.truncate("/memfs/file.txt", 1024)
    /// ```
    pub fn truncate(&self, py: Python, path: &str, size: i64) -> PyResult<()> {
        let client = self.client.clone();
        let rt = self.rt.clone();
        let path = path.to_string();

        py.allow_threads(|| {
            rt.block_on(async move {
                client.truncate(&path, size).await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
        })
    }

    /// Health check
    ///
    /// Checks if the CtxFS server is healthy and responding.
    ///
    /// # Raises
    ///
    /// PyIOError if the server is not healthy
    ///
    /// # Example
    ///
    /// ```python
    /// client.health()
    /// print("Server is healthy!")
    /// ```
    pub fn health(&self, py: Python) -> PyResult<()> {
        let client = self.client.clone();
        let rt = self.rt.clone();

        py.allow_threads(|| {
            rt.block_on(async move {
                client.health().await
            })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))
        })
    }

    /// Get server capabilities
    ///
    /// # Returns
    ///
    /// A dictionary with:
    /// - version: str - Server version
    /// - features: list of str - Supported features
    ///
    /// # Example
    ///
    /// ```python
    /// caps = client.get_capabilities()
    /// print(f"Server version: {caps['version']}")
    /// ```
    pub fn get_capabilities<'py>(&'py self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let client = self.client.clone();
        let rt = self.rt.clone();

        // First do the async operation outside of GIL
        let response = py.allow_threads(|| {
            rt.block_on(async move {
                client.get_capabilities().await
            })
        }).map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

        // Then build the Python dict with GIL held
        let dict = PyDict::new_bound(py);
        dict.set_item("version", response.version)?;
        dict.set_item("features", response.features)?;
        Ok(dict)
    }

    /// String representation of the client
    ///
    /// # Example
    ///
    /// ```python
    /// client = ctxfs.Client()
    /// print(client)  # <ctxfs.Client base_url=http://localhost:8080/api/v1>
    /// ```
    fn __repr__(&self) -> String {
        format!("<ctxfs.Client base_url={}>", self.client.base_url())
    }

    /// String representation of the client
    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// CtxFS Python module
///
/// This module provides the CtxFS client for Python.
#[pymodule]
fn ctxfs(m: &Bound<PyModule>) -> PyResult<()> {
    m.add_class::<PyClient>()?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__author__", "CtxFS Contributors")?;
    Ok(())
}
