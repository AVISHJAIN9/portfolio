package ttl

import (
	"sync"
	"time"
)

// 09-031..09-040: Circular Timing Wheel for Expiration
type WheelEntry struct {
	Key       string
	ExpiresAt time.Time
}

type TimingWheel struct {
	mu        sync.Mutex
	interval  time.Duration
	slots     int
	buckets   [][]WheelEntry
	cursor    int
	onExpired func(key string)
	stopCh    chan struct{}
}

func NewTimingWheel(interval time.Duration, slots int, onExpired func(key string)) *TimingWheel {
	buckets := make([][]WheelEntry, slots)
	for i := range buckets {
		buckets[i] = make([]WheelEntry, 0)
	}
	return &TimingWheel{
		interval:  interval,
		slots:     slots,
		buckets:   buckets,
		cursor:    0,
		onExpired: onExpired,
		stopCh:    make(chan struct{}),
	}
}

func (w *TimingWheel) Schedule(key string, expireIn time.Duration) {
	w.mu.Lock()
	defer w.mu.Unlock()

	ticks := int(expireIn / w.interval)
	if ticks <= 0 {
		ticks = 1
	}
	slot := (w.cursor + ticks) % w.slots
	w.buckets[slot] = append(w.buckets[slot], WheelEntry{
		Key:       key,
		ExpiresAt: time.Now().Add(expireIn),
	})
}

// 09-035..09-037: Active sweeper step
func (w *TimingWheel) Advance() []string {
	w.mu.Lock()
	defer w.mu.Unlock()

	w.cursor = (w.cursor + 1) % w.slots
	entries := w.buckets[w.cursor]
	w.buckets[w.cursor] = make([]WheelEntry, 0)

	now := time.Now()
	var expiredKeys []string
	var remaining []WheelEntry

	for _, entry := range entries {
		if now.After(entry.ExpiresAt) || now.Equal(entry.ExpiresAt) {
			expiredKeys = append(expiredKeys, entry.Key)
			if w.onExpired != nil {
				w.onExpired(entry.Key)
			}
		} else {
			remaining = append(remaining, entry)
		}
	}
	w.buckets[w.cursor] = remaining
	return expiredKeys
}

func (w *TimingWheel) Start() {
	ticker := time.NewTicker(w.interval)
	go func() {
		for {
			select {
			case <-ticker.C:
				w.Advance()
			case <-w.stopCh:
				ticker.Stop()
				return
			}
		}
	}()
}

func (w *TimingWheel) Stop() {
	select {
	case <-w.stopCh:
	default:
		close(w.stopCh)
	}
}
