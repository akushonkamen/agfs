package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/dongxuny/agfs-fuse/pkg/fusefs"
	"github.com/dongxuny/agfs-fuse/pkg/version"
	"github.com/hanwen/go-fuse/v2/fs"
	"github.com/hanwen/go-fuse/v2/fuse"
)

func main() {
	var (
		showVersion  = flag.Bool("version", false, "Show version information")
		cacheTTL     = flag.Duration("cache-ttl", 5*time.Second, "Cache TTL duration")
		writeWorkers = flag.Int("write-workers", 8, "Number of concurrent write workers")
		debug        = flag.Bool("debug", false, "Enable debug output")
		allowOther   = flag.Bool("allow-other", false, "Allow other users to access the mount")
	)

	flag.Usage = func() {
		fmt.Fprintf(os.Stderr, "Usage: %s [options] <server-url> <mountpoint>\n\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "Mount AGFS server as a FUSE filesystem.\n\n")
		fmt.Fprintf(os.Stderr, "Options:\n")
		flag.PrintDefaults()
		fmt.Fprintf(os.Stderr, "\nExamples:\n")
		fmt.Fprintf(os.Stderr, "  %s http://localhost:8080 /mnt/agfs\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "  %s --cache-ttl=10s http://localhost:8080 /mnt/agfs\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "  %s --debug http://localhost:8080 /mnt/agfs\n", os.Args[0])
	}

	flag.Parse()

	// Show version
	if *showVersion {
		fmt.Printf("agfs-fuse %s\n", version.GetFullVersion())
		os.Exit(0)
	}

	// Check arguments
	if flag.NArg() != 2 {
		flag.Usage()
		os.Exit(1)
	}

	serverURL := flag.Arg(0)
	mountpoint := flag.Arg(1)

	// Create filesystem
	root := fusefs.NewAGFSFS(fusefs.Config{
		ServerURL:    serverURL,
		CacheTTL:     *cacheTTL,
		WriteWorkers: *writeWorkers,
		Debug:        *debug,
	})

	// Setup FUSE mount options
	opts := &fs.Options{
		AttrTimeout:  cacheTTL,
		EntryTimeout: cacheTTL,
		MountOptions: fuse.MountOptions{
			Name:          "agfs",
			FsName:        "agfs",
			DisableXAttrs: true,
			Debug:         *debug,
		},
	}

	if *allowOther {
		opts.MountOptions.AllowOther = true
	}

	// Mount the filesystem
	server, err := fs.Mount(mountpoint, root, opts)
	if err != nil {
		log.Fatalf("Mount failed: %v", err)
	}

	fmt.Printf("AGFS mounted at %s\n", mountpoint)
	fmt.Printf("Server: %s\n", serverURL)
	fmt.Printf("Cache TTL: %v\n", *cacheTTL)
	fmt.Printf("Write workers: %d\n", *writeWorkers)

	if !*debug {
		fmt.Println("Press Ctrl+C to unmount")
	}

	// Handle graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		<-sigChan
		fmt.Println("\nUnmounting...")

		// Unmount
		if err := server.Unmount(); err != nil {
			log.Printf("Unmount failed: %v", err)
		}

		// Close filesystem
		if err := root.Close(); err != nil {
			log.Printf("Close filesystem failed: %v", err)
		}
	}()

	// Wait for the filesystem to be unmounted
	server.Wait()

	fmt.Println("AGFS unmounted successfully")
}
