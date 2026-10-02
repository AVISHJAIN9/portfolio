package singleflight

import "sync"

type call struct {
	wg  sync.WaitGroup
	val []byte
	err error
}

// 09-056: SingleFlight prevents cache stampedes by suppressing duplicate concurrent calls
type Group struct {
	mu sync.Mutex
	m  map[string]*call
}

func NewGroup() *Group {
	return &Group{
		m: make(map[string]*call),
	}
}

func (g *Group) Do(key string, fn func() ([]byte, error)) ([]byte, error, bool) {
	g.mu.Lock()
	if c, ok := g.m[key]; ok {
		g.mu.Unlock()
		c.wg.Wait()
		return c.val, c.err, true // shared execution
	}

	c := new(call)
	c.wg.Add(1)
	g.m[key] = c
	g.mu.Unlock()

	c.val, c.err = fn()
	c.wg.Done()

	g.mu.Lock()
	delete(g.m, key)
	g.mu.Unlock()

	return c.val, c.err, false // primary execution
}
