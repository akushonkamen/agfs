package fusefs

import (
	"testing"

	agfs "github.com/c4pt0r/agfs/agfs-sdk/go"
)

func TestHandleManagerBasicOperations(t *testing.T) {
	// Note: This is a unit test that doesn't require a running server
	// We're testing the handle manager's mapping logic

	client := agfs.NewClient("http://localhost:8080")
	hm := NewHandleManager(client)

	// Test initial state
	if count := hm.Count(); count != 0 {
		t.Errorf("Expected 0 handles, got %d", count)
	}

	// Note: We can't actually test Open/Close without a running server
	// Those would be integration tests
}

func TestHandleManagerConcurrency(t *testing.T) {
	client := agfs.NewClient("http://localhost:8080")
	hm := NewHandleManager(client)

	// Test concurrent access to handle map (shouldn't panic)
	done := make(chan bool, 2)

	go func() {
		for i := 0; i < 100; i++ {
			hm.Count()
		}
		done <- true
	}()

	go func() {
		for i := 0; i < 100; i++ {
			hm.Count()
		}
		done <- true
	}()

	<-done
	<-done

	// If we got here without panic, concurrency is safe
}
