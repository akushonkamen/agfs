package api

import (
	"context"
	"fmt"
	"sync"
	"time"

	"github.com/c4pt0r/agfs/agfs-server/pkg/filesystem"
	log "github.com/sirupsen/logrus"
	"github.com/tetratelabs/wazero"
	wazeroapi "github.com/tetratelabs/wazero/api"
)

// PoolConfig contains configuration for the instance pool
type PoolConfig struct {
	MaxInstances        int           // Maximum number of concurrent instances
	InstanceMaxLifetime time.Duration // Maximum instance lifetime (0 = unlimited)
	InstanceMaxRequests int64         // Maximum requests per instance (0 = unlimited)
	HealthCheckInterval time.Duration // Health check interval (0 = disabled)
	EnableStatistics    bool          // Enable statistics collection
}

// WASMInstancePool manages a pool of WASM module instances for concurrent access
type WASMInstancePool struct {
	ctx              context.Context
	runtime          wazero.Runtime
	compiledModule   wazero.CompiledModule
	hostFS           filesystem.FileSystem
	pluginName       string
	config           PoolConfig
	instances        chan *WASMModuleInstance
	currentInstances int
	mu               sync.Mutex
	stats            PoolStats
	closed           bool
}

// PoolStats tracks pool usage statistics
type PoolStats struct {
	TotalCreated   int64
	TotalDestroyed int64
	CurrentActive  int64
	TotalWaits     int64
	TotalRequests  int64
	FailedRequests int64
	mu             sync.Mutex
}

// WASMModuleInstance represents a single WASM module instance
type WASMModuleInstance struct {
	module       wazeroapi.Module
	fileSystem   *WASMFileSystem
	createdAt    time.Time
	requestCount int64 // Number of requests handled by this instance
	mu           sync.Mutex
}

// NewWASMInstancePool creates a new WASM instance pool with configuration
func NewWASMInstancePool(ctx context.Context, runtime wazero.Runtime, compiledModule wazero.CompiledModule,
	pluginName string, config PoolConfig, hostFS filesystem.FileSystem) *WASMInstancePool {

	// Apply defaults
	if config.MaxInstances <= 0 {
		config.MaxInstances = 10 // default to 10 concurrent instances
	}

	pool := &WASMInstancePool{
		ctx:            ctx,
		runtime:        runtime,
		compiledModule: compiledModule,
		hostFS:         hostFS,
		pluginName:     pluginName,
		config:         config,
		instances:      make(chan *WASMModuleInstance, config.MaxInstances),
	}

	log.Infof("Created WASM instance pool for %s (max_instances=%d, max_lifetime=%v, max_requests=%d)",
		pluginName, config.MaxInstances, config.InstanceMaxLifetime, config.InstanceMaxRequests)

	// Start health check goroutine if enabled
	if config.HealthCheckInterval > 0 {
		go pool.healthCheckLoop()
	}

	return pool
}

// healthCheckLoop periodically checks instance health
func (p *WASMInstancePool) healthCheckLoop() {
	ticker := time.NewTicker(p.config.HealthCheckInterval)
	defer ticker.Stop()

	for {
		select {
		case <-p.ctx.Done():
			return
		case <-ticker.C:
			p.performHealthCheck()
		}
	}
}

// performHealthCheck checks the health of instances in the pool
func (p *WASMInstancePool) performHealthCheck() {
	p.mu.Lock()
	closed := p.closed
	p.mu.Unlock()

	if closed {
		return
	}

	log.Debugf("[Pool %s] Health check: active instances=%d/%d",
		p.pluginName, p.currentInstances, p.config.MaxInstances)
}

// Acquire gets an instance from the pool or creates a new one if available
func (p *WASMInstancePool) Acquire() (*WASMModuleInstance, error) {
	// Check if pool is closed
	p.mu.Lock()
	if p.closed {
		p.mu.Unlock()
		return nil, fmt.Errorf("instance pool is closed")
	}
	p.mu.Unlock()

	// Increment request counter if statistics enabled
	if p.config.EnableStatistics {
		p.stats.mu.Lock()
		p.stats.TotalRequests++
		p.stats.mu.Unlock()
	}

	// Try to get an existing instance from the pool
	select {
	case instance := <-p.instances:
		// Check if instance needs to be recycled
		if p.shouldRecycleInstance(instance) {
			log.Debugf("Recycling expired WASM instance for %s", p.pluginName)
			p.destroyInstance(instance)

			p.mu.Lock()
			p.currentInstances--
			p.mu.Unlock()

			// Create a new instance to replace the recycled one
			return p.Acquire()
		}

		log.Debugf("Reusing WASM instance from pool for %s", p.pluginName)
		return instance, nil
	default:
		// No available instance, try to create a new one
		p.mu.Lock()
		canCreate := p.currentInstances < p.config.MaxInstances
		if canCreate {
			p.currentInstances++
		}
		p.mu.Unlock()

		if canCreate {
			instance, err := p.createInstance()
			if err != nil {
				p.mu.Lock()
				p.currentInstances--
				p.mu.Unlock()

				if p.config.EnableStatistics {
					p.stats.mu.Lock()
					p.stats.FailedRequests++
					p.stats.mu.Unlock()
				}
				return nil, err
			}

			if p.config.EnableStatistics {
				p.stats.mu.Lock()
				p.stats.TotalCreated++
				p.stats.CurrentActive++
				p.stats.mu.Unlock()
			}

			log.Debugf("Created new WASM instance for %s (total: %d/%d)",
				p.pluginName, p.currentInstances, p.config.MaxInstances)
			return instance, nil
		}

		// Pool is full, wait for an available instance
		log.Debugf("WASM pool full for %s, waiting for available instance...", p.pluginName)
		if p.config.EnableStatistics {
			p.stats.mu.Lock()
			p.stats.TotalWaits++
			p.stats.mu.Unlock()
		}

		instance := <-p.instances

		// Check if instance needs to be recycled
		if p.shouldRecycleInstance(instance) {
			log.Debugf("Recycling expired WASM instance for %s", p.pluginName)
			p.destroyInstance(instance)

			p.mu.Lock()
			p.currentInstances--
			p.mu.Unlock()

			// Create a new instance to replace the recycled one
			return p.Acquire()
		}

		return instance, nil
	}
}

// shouldRecycleInstance checks if an instance should be recycled
func (p *WASMInstancePool) shouldRecycleInstance(instance *WASMModuleInstance) bool {
	instance.mu.Lock()
	defer instance.mu.Unlock()

	// Check max lifetime
	if p.config.InstanceMaxLifetime > 0 {
		age := time.Since(instance.createdAt)
		if age > p.config.InstanceMaxLifetime {
			log.Debugf("Instance exceeded max lifetime: %v > %v", age, p.config.InstanceMaxLifetime)
			return true
		}
	}

	// Check max requests
	if p.config.InstanceMaxRequests > 0 && instance.requestCount >= p.config.InstanceMaxRequests {
		log.Debugf("Instance exceeded max requests: %d >= %d", instance.requestCount, p.config.InstanceMaxRequests)
		return true
	}

	return false
}

// Release returns an instance to the pool
func (p *WASMInstancePool) Release(instance *WASMModuleInstance) {
	if instance == nil {
		return
	}

	// Try to return to pool, if pool is full, destroy the instance
	select {
	case p.instances <- instance:
		log.Debugf("Returned WASM instance to pool for %s", p.pluginName)
	default:
		// Pool is full, destroy this instance
		log.Debugf("Pool full, destroying excess WASM instance for %s", p.pluginName)
		p.destroyInstance(instance)

		p.mu.Lock()
		p.currentInstances--
		p.mu.Unlock()

		p.stats.mu.Lock()
		p.stats.TotalDestroyed++
		p.stats.CurrentActive--
		p.stats.mu.Unlock()
	}
}

// createInstance creates a new WASM module instance
func (p *WASMInstancePool) createInstance() (*WASMModuleInstance, error) {
	// Instantiate the compiled module
	module, err := p.runtime.InstantiateModule(p.ctx, p.compiledModule, wazero.NewModuleConfig())
	if err != nil {
		return nil, fmt.Errorf("failed to instantiate WASM module: %w", err)
	}

	// Call plugin_new to initialize
	if newFunc := module.ExportedFunction("plugin_new"); newFunc != nil {
		if _, err := newFunc.Call(p.ctx); err != nil {
			module.Close(p.ctx)
			return nil, fmt.Errorf("failed to call plugin_new: %w", err)
		}
	}

	instance := &WASMModuleInstance{
		module:    module,
		createdAt: time.Now(),
		fileSystem: &WASMFileSystem{
			ctx:    p.ctx,
			module: module,
			mu:     nil, // No mutex needed - each instance is single-threaded
		},
	}

	return instance, nil
}

// destroyInstance destroys a WASM module instance
func (p *WASMInstancePool) destroyInstance(instance *WASMModuleInstance) {
	if instance == nil || instance.module == nil {
		return
	}

	// Call plugin shutdown if available
	if shutdownFunc := instance.module.ExportedFunction("plugin_shutdown"); shutdownFunc != nil {
		shutdownFunc.Call(p.ctx)
	}

	// Close the module
	instance.module.Close(p.ctx)
}

// Close closes the pool and destroys all instances
func (p *WASMInstancePool) Close() error {
	p.mu.Lock()
	defer p.mu.Unlock()

	// Close all instances in the pool
	close(p.instances)
	for instance := range p.instances {
		p.destroyInstance(instance)
	}

	log.Infof("Closed WASM instance pool for %s", p.pluginName)
	return nil
}

// GetStats returns the current pool statistics
func (p *WASMInstancePool) GetStats() PoolStats {
	p.stats.mu.Lock()
	defer p.stats.mu.Unlock()
	return p.stats
}

// Execute executes a function with an instance from the pool
// This is a convenience method that handles acquire/release automatically
func (p *WASMInstancePool) Execute(fn func(*WASMModuleInstance) error) error {
	instance, err := p.Acquire()
	if err != nil {
		return err
	}
	defer p.Release(instance)

	return fn(instance)
}

// ExecuteFS executes a filesystem operation with an instance from the pool
func (p *WASMInstancePool) ExecuteFS(fn func(filesystem.FileSystem) error) error {
	instance, err := p.Acquire()
	if err != nil {
		return err
	}
	defer p.Release(instance)

	return fn(instance.fileSystem)
}
