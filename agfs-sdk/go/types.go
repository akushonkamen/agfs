package agfs

import "time"

// MetaData represents structured metadata for files and directories
type MetaData struct {
	Name    string            // Plugin name or identifier
	Type    string            // Type classification of the file/directory
	Content map[string]string // Additional extensible metadata
}

// FileInfo represents file metadata similar to os.FileInfo
type FileInfo struct {
	Name    string
	Size    int64
	Mode    uint32
	ModTime time.Time
	IsDir   bool
	Meta    MetaData // Structured metadata for additional information
}
