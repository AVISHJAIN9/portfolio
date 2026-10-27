/**
 * ============================================================================
 * A WINTER'S JOURNEY — CINEMATIC INTERACTIVE PORTFOLIO
 * Complete interactive orchestration engine.
 * Fully table-driven Finite State Machine adhering to Global Rules R1–R5,
 * Tasks 1–9, and User Acceptance Criteria A1–A10.
 * ============================================================================
 */

'use strict';

// Legacy tree overlays flag (three static video panels in Scene 1)
const FOREST_LEGACY_OVERLAYS = false;

/* ============================================================================
   GLOBAL STATE SINGLETON
   ============================================================================ */
const State = {
  currentState:       'S1_FOREST',
  currentRole:        'forest-establishing',
  currentClipId:      'scene-1',
  isInputLocked:      false,
  isTransitioning:    false,
  activeBuffer:       'a',
  quality:            'hq',
  soundEnabled:       false,
  debugMode:          false,
  showOutlines:       true,
  showMasks:          false,
  showTreeOutlines:   false,
  editPoints:         false,
  recentEvents:       [],
  layout: {
    scale: 1,
    offsetX: 0,
    offsetY: 0,
    renderedW: 1280,
    renderedH: 720
  },
  clipsData: null,
  machineData: null,
  hotspotData: null,
  scrollProgress: 0
};

/* Narrative story metadata per state */
const NARRATIVE_MANIFEST = {
  S1_FOREST: {
    title: 'The Snowy Taiga',
    badge: 'SCENE 1',
    type: 'SCROLL',
    desc: 'A lone camera glides into the untouched winter forest.',
    hint: 'Scroll down to travel through the ancient pines'
  },
  S2_PATH: {
    title: 'The Woodland Trail',
    badge: 'SCENE 2',
    type: 'SCROLL',
    desc: 'The forest deepens towards an abandoned logging clearing.',
    hint: 'Continue scrolling towards the old wood stump'
  },
  S3_STUMP: {
    title: 'The Hidden Burrow',
    badge: 'SCENE 3',
    type: 'CLICK GATED',
    desc: 'A curious raccoon dog emerges from behind the snow-capped stump.',
    hint: 'Click the raccoon dog to awaken the journey'
  },
  S7_REINDEER: {
    title: 'The Hollow Lantern Tree',
    badge: 'SCENE 4',
    type: 'CLICK GATED',
    desc: 'A magnificent spirit reindeer rests beneath the glowing amber lantern.',
    hint: 'Click the reindeer to venture into the glacial cave'
  },
  S9_ICE_CAVE: {
    title: 'The Glacial Vault',
    badge: 'SCENE 5',
    type: 'INTERACTIVE',
    desc: 'A luminous cavern ceiling of hanging crystalline icicle formations.',
    hint: 'Click an icicle to inspect engineering works — or click elsewhere to ascend'
  },
  S8_HOLD: {
    title: 'The Alpine Summit',
    badge: 'SCENE 6',
    type: 'INTERACTIVE',
    desc: 'Reindeer and golden eagle stand together overlooking the nine snowy peaks.',
    hint: 'Click the eagle, a mountain peak, or journey toward the cabin'
  },
  S_HOUSE: {
    title: 'The Snowbound Refuge',
    badge: 'SCENE 7',
    type: 'CLICK GATED',
    desc: 'A warm timber cabin stands resilient against the mountain gale.',
    hint: 'Click the cabin door to step inside'
  },
  S12_INTERIOR: {
    title: 'The Craftsman\'s Hearth',
    badge: 'SCENE 8',
    type: 'CLICK GATED',
    desc: 'Warm hearth fire illuminates architectural drafting tables and frost glass.',
    hint: 'Click the blueprint desk or frosted window to gaze into the cosmos'
  },
  S13_FINALE: {
    title: 'Celestial Aurora',
    badge: 'FINALE',
    type: 'HORIZON',
    desc: 'The reindeer stands immortalized beneath constellations and northern lights.',
    hint: 'Journey complete. Click Clubs to view the 2x2 grid or Replay to travel anew'
  },
  S14_CLUBS: {
    title: 'The Clubs Showcase',
    badge: 'SCENE 14',
    type: '2x2 QUADRANTS',
    desc: 'Four club universes looping in synchronized full-viewport harmony.',
    hint: 'Four clubs in 2x2 view. Click Back to return or Replay to restart'
  }
};

/* ============================================================================
   TASK 1: LAYOUT & COVER GEOMETRY ENGINE
   Exact video cover math: sizes hotspot SVG and overlays to match video pixels
   ============================================================================ */
class LayoutEngine {
  static updateLayout() {
    const viewW = window.innerWidth;
    const viewH = window.innerHeight;
    const scale = Math.max(viewW / 1280, viewH / 720);
    const renderedW = 1280 * scale;
    const renderedH = 720 * scale;
    const offsetX = (viewW - renderedW) / 2;
    const offsetY = (viewH - renderedH) / 2;

    State.layout = { scale, offsetX, offsetY, renderedW, renderedH };

    console.log(`[LAYOUT] scale=${scale.toFixed(4)}, offsetX=${offsetX.toFixed(1)}, offsetY=${offsetY.toFixed(1)}, rendered=${renderedW.toFixed(0)}x${renderedH.toFixed(0)}`);

    // Sizing SVG Hotspot layer
    const svg = document.getElementById('hotspot-svg');
    if (svg) {
      svg.style.position = 'absolute';
      svg.style.left = `${offsetX}px`;
      svg.style.top = `${offsetY}px`;
      svg.style.width = `${renderedW}px`;
      svg.style.height = `${renderedH}px`;
    }

    // Sizing tree overlay container
    const ovContainer = document.getElementById('overlay-container');
    if (ovContainer) {
      ovContainer.style.position = 'absolute';
      ovContainer.style.left = `${offsetX}px`;
      ovContainer.style.top = `${offsetY}px`;
      ovContainer.style.width = `${renderedW}px`;
      ovContainer.style.height = `${renderedH}px`;
    }

    // Sizing mask overlay container
    const maskLayer = document.getElementById('mask-overlay-layer');
    if (maskLayer) {
      maskLayer.style.position = 'absolute';
      maskLayer.style.left = `${offsetX}px`;
      maskLayer.style.top = `${offsetY}px`;
      maskLayer.style.width = `${renderedW}px`;
      maskLayer.style.height = `${renderedH}px`;
    }

    // Update debug panel metrics if visible
    if (State.debugMode) {
      const dbgLayout = document.getElementById('dbg-layout');
      if (dbgLayout) {
        dbgLayout.textContent = `${viewW}x${viewH} -> ${renderedW.toFixed(0)}x${renderedH.toFixed(0)} (${scale.toFixed(3)})`;
      }
    }
  }
}

/* ============================================================================
   DUAL-BUFFER VIDEO PLAYER
   Pre-buffers inactive buffer, crossfades on canplaythrough over ~250ms
   Handles ping-pong looping and holdLastFrame
   ============================================================================ */
class DualBufferPlayer {
  constructor() {
    this.bufferA = document.getElementById('video-buffer-a');
    this.bufferB = document.getElementById('video-buffer-b');
    this.houseStill = document.getElementById('house-still-holder');
    this.currentClipRole = null;
    this.currentLoopMode = 'none';
    this.pingPongDirection = 1; // 1 = forward, -1 = reverse
    this.pingPongTimer = null;
    this.activeVideo = this.bufferA;
    this.inactiveVideo = this.bufferB;

    this._setupPingPong(this.bufferA);
    this._setupPingPong(this.bufferB);
  }

  _setupPingPong(videoEl) {
    videoEl.addEventListener('timeupdate', () => {
      if (window.ForestClips && window.ForestClips.blockBufferSwap) return;
      if (this.currentLoopMode === 'pingpong' && videoEl === this.activeVideo) {
        const clip = State.clipsData?.clips[this.currentClipRole];
        const end = (clip && clip.endTime) ? clip.endTime : (videoEl.duration - 0.1);
        const start = (clip && clip.startTime) ? clip.startTime : 0.05;

        if (this.pingPongDirection === 1 && videoEl.currentTime >= end) {
          // Soft reverse: loop back seamlessly
          videoEl.currentTime = start;
          videoEl.play().catch(() => {});
        }
      }
    });
  }

  /**
   * Preload a clip role into the inactive buffer
   */
  preloadRole(role) {
    const clip = State.clipsData?.clips[role];
    if (!clip || !clip.file) return Promise.resolve();

    const video = this.inactiveVideo;
    const file = (State.quality === 'lq' && clip.fallbackFile) ? clip.fallbackFile : clip.file;

    return new Promise((resolve) => {
      let resolved = false;
      const onReady = () => {
        if (!resolved) {
          resolved = true;
          video.removeEventListener('canplaythrough', onReady);
          video.removeEventListener('loadeddata', onReady);
          resolve();
        }
      };

      video.addEventListener('canplaythrough', onReady);
      video.addEventListener('loadeddata', onReady);
      video.src = file;
      video.load();

      // Fallback timeout
      setTimeout(onReady, 1200);
    });
  }

  /**
   * Play a clip role with seamless crossfade
   */
  async playRole(role, options = {}) {
    // Dual-buffer safety: block buffer swap while ForestClips is active
    if (window.ForestClips && window.ForestClips.blockBufferSwap) {
      console.warn('[PLAYER] playRole buffer swap blocked while snow clip is active');
      return;
    }

    const clip = State.clipsData?.clips[role];
    if (!clip) {
      console.warn(`[PLAYER] Unknown clip role: ${role}`);
      return;
    }

    const {
      loop = clip.loop,
      loopType = (loop ? 'seamless' : 'none'),
      holdLastFrame = clip.holdLastFrame,
      onEnded = null,
      crossfadeMs = 250
    } = options;

    this.currentClipRole = role;
    this.currentLoopMode = loopType;
    State.currentRole = role;
    State.currentClipId = clip.id;

    // Special handler for house exterior still
    if (role === 'house-exterior-hold') {
      if (this.houseStill) this.houseStill.style.display = 'block';
      this.activeVideo.style.opacity = '0';
      if (onEnded) setTimeout(onEnded, 500);
      return;
    } else {
      if (this.houseStill) this.houseStill.style.display = 'none';
    }

    const nextVideo = this.inactiveVideo;
    const currentVideo = this.activeVideo;
    const file = (State.quality === 'lq' && clip.fallbackFile) ? clip.fallbackFile : clip.file;

    // Load file if not already loaded
    if (!nextVideo.src.includes(file)) {
      nextVideo.src = file;
      nextVideo.load();
      await new Promise((resolve) => {
        if (nextVideo.readyState >= 2) {
          resolve();
          return;
        }
        let resolved = false;
        const done = () => {
          if (!resolved) {
            resolved = true;
            nextVideo.removeEventListener('loadeddata', done);
            nextVideo.removeEventListener('canplay', done);
            nextVideo.removeEventListener('canplaythrough', done);
            resolve();
          }
        };
        nextVideo.addEventListener('loadeddata', done);
        nextVideo.addEventListener('canplay', done);
        nextVideo.addEventListener('canplaythrough', done);
        setTimeout(done, 4000);
      });
    }

    // Set timestamps & loop attributes
    nextVideo.currentTime = clip.startTime || 0;
    nextVideo.loop = (loopType === 'seamless');

    // On ended handler
    if (!nextVideo.loop) {
      const endedHandler = () => {
        nextVideo.removeEventListener('ended', endedHandler);
        if (holdLastFrame) {
          nextVideo.pause();
        }
        if (onEnded) onEnded();
      };
      nextVideo.addEventListener('ended', endedHandler);
    }

    try {
      const playPromise = nextVideo.play();
      if (playPromise !== undefined) {
        playPromise.catch((e) => {
          console.warn('[PLAYER] Autoplay policy prevented playback:', e);
          const resume = () => {
            ['click', 'touchstart', 'scroll', 'keydown'].forEach(evt => window.removeEventListener(evt, resume));
            if (nextVideo.paused && State.currentState !== 'S8_HOLD' && State.currentState !== 'S_HOUSE') {
              nextVideo.play().catch(() => {});
            }
          };
          ['click', 'touchstart', 'scroll', 'keydown'].forEach(evt => window.addEventListener(evt, resume, { once: true, passive: true }));
        });
      }
    } catch (e) {
      console.warn('[PLAYER] Play invocation error:', e);
    }

    // Perform smooth ~250ms crossfade
    nextVideo.style.transition = `opacity ${crossfadeMs}ms linear`;
    currentVideo.style.transition = `opacity ${crossfadeMs}ms linear`;

    nextVideo.style.opacity = '1';
    currentVideo.style.opacity = '0';
    nextVideo.classList.add('active');
    currentVideo.classList.remove('active');

    // Swap active/inactive buffer references
    this.activeVideo = nextVideo;
    this.inactiveVideo = currentVideo;
    State.activeBuffer = (this.activeVideo === this.bufferA) ? 'a' : 'b';

    await new Promise((r) => setTimeout(r, crossfadeMs));

    currentVideo.pause();

    LayoutEngine.updateLayout();
  }

  /**
   * Scrub active video for scroll scenes
   */
  scrub(time) {
    if (this.activeVideo) {
      if (Math.abs(this.activeVideo.currentTime - time) < 0.04) {
        return;
      }
      console.log('[SCRUB]', 'time:', time, 'id:', this.activeVideo.id, 'before:', this.activeVideo.currentTime);
      this.activeVideo.currentTime = time;
    }
  }
}

/* ============================================================================
   TREE OVERLAY MANAGER
   Plays time-synced swaying tree overlays in Scene 1
   ============================================================================ */
class OverlayManager {
  constructor() {
    this.treeEls = {
      'tree-overlay-left':   document.getElementById('overlay-tree-1-1'),
      'tree-overlay-center': document.getElementById('overlay-tree-1-2'),
      'tree-overlay-right':  document.getElementById('overlay-tree-1-3')
    };
    this.videoEls = {
      'tree-overlay-left':   document.getElementById('video-tree-1-1'),
      'tree-overlay-center': document.getElementById('video-tree-1-2'),
      'tree-overlay-right':  document.getElementById('video-tree-1-3')
    };
  }

  initTreeOverlays() {
    const roles = ['tree-overlay-left', 'tree-overlay-center', 'tree-overlay-right'];
    roles.forEach((r) => {
      const clip = State.clipsData?.clips[r];
      const video = this.videoEls[r];
      if (clip && video) {
        video.src = clip.file;
        video.loop = true;
        video.play().catch(() => {});
      }
    });
  }

  showOverlays() {
    Object.values(this.treeEls).forEach((el) => {
      if (el) el.classList.add('playing');
    });
  }

  hideOverlays() {
    Object.values(this.treeEls).forEach((el) => {
      if (el) el.classList.remove('playing');
    });
  }
}

/* ============================================================================
   WEB AUDIO PROCEDURAL WINTER SOUNDSCAPE
   Synthesizes chilling blizzard wind drone, footsteps, and crystal chimes
   ============================================================================ */
class WinterSoundEngine {
  constructor() {
    this.ctx = null;
    this.masterGain = null;
    this.windGain = null;
    this.initialized = false;
  }

  _init() {
    if (this.initialized) return;
    const AudioContext = window.AudioContext || window.webkitAudioContext;
    if (!AudioContext) return;
    this.ctx = new AudioContext();

    this.masterGain = this.ctx.createGain();
    this.masterGain.gain.setValueAtTime(0.001, this.ctx.currentTime);
    this.masterGain.connect(this.ctx.destination);

    // Procedural Brown Noise generator for ambient arctic wind
    const bufferSize = this.ctx.sampleRate * 3;
    const noiseBuffer = this.ctx.createBuffer(1, bufferSize, this.ctx.sampleRate);
    const output = noiseBuffer.getChannelData(0);
    let lastOut = 0.0;
    for (let i = 0; i < bufferSize; i++) {
      const white = Math.random() * 2 - 1;
      output[i] = (lastOut + (0.02 * white)) / 1.02;
      lastOut = output[i];
      output[i] *= 3.5;
    }

    const whiteNoise = this.ctx.createBufferSource();
    whiteNoise.buffer = noiseBuffer;
    whiteNoise.loop = true;

    // Resonant bandpass filter
    const filter = this.ctx.createBiquadFilter();
    filter.type = 'bandpass';
    filter.frequency.setValueAtTime(320, this.ctx.currentTime);
    filter.Q.setValueAtTime(3.0, this.ctx.currentTime);

    this.windGain = this.ctx.createGain();
    this.windGain.gain.setValueAtTime(0.25, this.ctx.currentTime);

    whiteNoise.connect(filter);
    filter.connect(this.windGain);
    this.windGain.connect(this.masterGain);

    whiteNoise.start();
    this.initialized = true;
  }

  toggle(enable) {
    this._init();
    if (!this.ctx) return;
    if (this.ctx.state === 'suspended') {
      this.ctx.resume();
    }
    const target = enable ? 0.35 : 0.001;
    this.masterGain.gain.setTargetAtTime(target, this.ctx.currentTime, 0.3);
  }

  playWhoosh() {
    if (!this.ctx || !State.soundEnabled) return;
    const osc = this.ctx.createOscillator();
    const g = this.ctx.createGain();
    osc.type = 'sine';
    osc.frequency.setValueAtTime(140, this.ctx.currentTime);
    osc.frequency.exponentialRampToValueAtTime(450, this.ctx.currentTime + 0.3);
    osc.frequency.exponentialRampToValueAtTime(80, this.ctx.currentTime + 0.9);
    g.gain.setValueAtTime(0.18, this.ctx.currentTime);
    g.gain.exponentialRampToValueAtTime(0.001, this.ctx.currentTime + 0.9);
    osc.connect(g);
    g.connect(this.ctx.destination);
    osc.start();
    osc.stop(this.ctx.currentTime + 0.95);
  }

  playChime() {
    if (!this.ctx || !State.soundEnabled) return;
    const freqs = [587.33, 880, 1174.66, 1760];
    freqs.forEach((f, idx) => {
      const osc = this.ctx.createOscillator();
      const g = this.ctx.createGain();
      osc.type = 'sine';
      osc.frequency.setValueAtTime(f, this.ctx.currentTime + (idx * 0.04));
      g.gain.setValueAtTime(0.08, this.ctx.currentTime + (idx * 0.04));
      g.gain.exponentialRampToValueAtTime(0.001, this.ctx.currentTime + 1.2);
      osc.connect(g);
      g.connect(this.ctx.destination);
      osc.start(this.ctx.currentTime + (idx * 0.04));
      osc.stop(this.ctx.currentTime + 1.25);
    });
  }
}

/* ============================================================================
   SNOW PARTICLE CANVAS ENGINE
   ============================================================================ */
class SnowParticleEngine {
  constructor() {
    this.canvas = document.getElementById('snow-canvas');
    this.ctx = this.canvas ? this.canvas.getContext('2d') : null;
    this.flakes = [];
    this.count = 90;
    this.animId = null;

    if (this.canvas) {
      this.resize();
      window.addEventListener('resize', () => this.resize());
      this._initFlakes();
      this.loop();
    }
  }

  resize() {
    this.canvas.width = window.innerWidth;
    this.canvas.height = window.innerHeight;
  }

  _initFlakes() {
    this.flakes = [];
    for (let i = 0; i < this.count; i++) {
      this.flakes.push({
        x: Math.random() * window.innerWidth,
        y: Math.random() * window.innerHeight,
        r: Math.random() * 2.2 + 0.8,
        speed: Math.random() * 1.2 + 0.6,
        drift: Math.random() * 1.4 - 0.7,
        alpha: Math.random() * 0.6 + 0.3
      });
    }
  }

  loop() {
    if (!this.ctx) return;
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

    this.ctx.fillStyle = '#ffffff';
    for (const f of this.flakes) {
      this.ctx.globalAlpha = f.alpha;
      this.ctx.beginPath();
      this.ctx.arc(f.x, f.y, f.r, 0, Math.PI * 2);
      this.ctx.fill();

      f.y += f.speed;
      f.x += f.drift;

      if (f.y > this.canvas.height) {
        f.y = -5;
        f.x = Math.random() * this.canvas.width;
      }
      if (f.x > this.canvas.width) f.x = 0;
      if (f.x < 0) f.x = this.canvas.width;
    }

    this.animId = requestAnimationFrame(() => this.loop());
  }
}

/* ============================================================================
   EXPLICIT FLOW STATE MACHINE (Section 5)
   States:
     S8_PLAYING, S8_HOLD, PEAK_11_1, S10_LOOP, HOUSE_11_1, HOUSE_11_2,
     HOUSE_11_3, HOUSE_DONE, BIRD_ANIM_A, S9, S9_DONE.
   Transitions (the ONLY allowed ones):
     - S8_PLAYING -> S8_HOLD on Scene 8 end.
     - S8_HOLD -> BIRD_ANIM_A on bird click -> S9 on its end -> S9_DONE.
     - S8_HOLD -> PEAK_11_1 on any of the 9 peak clicks -> S10_LOOP on 11.1 end.
     - S10_LOOP -> S8_HOLD on Back (button labelled "Back to Peak"/"Back").
     - S8_HOLD -> HOUSE_11_1 -> HOUSE_11_2 -> HOUSE_11_3 -> HOUSE_DONE on "Go to House".
   Rules:
     R1. Every transition logged: window.__flowLog.push({t, from, to, source})
     R2. REUSE SAFETY for 11.1: path context ('PEAK' or 'HOUSE') and unique runId.
     R3. While any clip is playing, ignore all hotspot clicks.
     R4. Auto-advance guard: in S8_HOLD, scroll/resize/timers do not advance scene.
     R5. Back returns to Scene 8 END frame instantly (decoded frame shown before hiding S10).
   ============================================================================ */

window.__flowLog = window.__flowLog || [];

class FlowStateMachine {
  constructor() {
    this.validStates = [
      'S8_PLAYING', 'S8_HOLD', 'PEAK_11_1', 'S10_LOOP',
      'HOUSE_11_1', 'HOUSE_11_2', 'HOUSE_11_3', 'HOUSE_DONE',
      'BIRD_ANIM_A', 'S9', 'S9_DONE'
    ];
    this.state = 'S8_PLAYING';
  }

  getState() {
    return this.state;
  }

  setFlowState(newState, source) {
    const oldState = this.state;
    this.state = newState;
    const entry = {
      t: performance.now(),
      from: oldState,
      to: newState,
      source: source || 'direct'
    };
    window.__flowLog.push(entry);
    console.log(`[FLOW-TRANSITION] ${oldState} -> ${newState} via ${source} at ${entry.t.toFixed(1)}ms`);
  }

  transition(from, to, source) {
    if (this.state !== from) {
      console.warn(`[FLOW-TRANSITION] State mismatch: current ${this.state} != expected from ${from}`);
    }
    this.setFlowState(to, source);
  }

  isFlowActive() {
    return this.state !== 'S8_PLAYING' && this.validStates.includes(this.state);
  }
}

class FlowSequencePlayer {
  constructor(canvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d', { alpha: false, desynchronized: true });
    this.fps = 25;
    this.frameDuration = 1000 / this.fps; // 40.0ms
    this.frameInterval = 1000 / this.fps; // 40.0ms
    this.activeRunId = 0;
    this.state = 'IDLE';
    this.animId = null;
    this.cache = new Map();
    this.loading = new Map();
  }

  _getUrls(clipKey) {
    const urls = [];
    if (clipKey === '11.1') {
      for (let i = 1; i <= 172; i++) {
        urls.push('highquality/scene%2011.1/frame_' + String(i).padStart(3, '0') + '.png');
      }
    } else if (clipKey === '11.2') {
      for (let i = 1; i <= 172; i++) {
        urls.push('highquality/scene11.2/untitled%20folder/frame_' + String(i).padStart(3, '0') + '.png');
      }
    } else if (clipKey === '11.3') {
      for (let i = 1; i <= 172; i++) {
        urls.push('highquality/scene%2011.3/frame_' + String(i).padStart(3, '0') + '.png');
      }
    } else if (clipKey === 'scene-10') {
      for (let i = 1; i <= 172; i++) {
        urls.push('highquality/scene%2010%20/' + String(i).padStart(3, '0') + '.png');
      }
    }
    return urls;
  }

  async preloadClip(clipKey) {
    if (this.cache.has(clipKey)) return this.cache.get(clipKey);
    if (this.loading.has(clipKey)) return this.loading.get(clipKey);

    const promise = (async () => {
      const urls = this._getUrls(clipKey);
      const frames = new Array(urls.length);
      const queue = urls.map((url, index) => ({ url, index }));
      const concurrency = 8;
      const workers = Array.from({ length: concurrency }, async () => {
        while (queue.length > 0) {
          const item = queue.shift();
          if (!item) break;
          await new Promise((resolve) => {
            const img = new Image();
            img.onload = () => {
              frames[item.index] = img;
              resolve();
            };
            img.onerror = () => {
              console.error(`[FLOW-PRELOAD] Failed loading frame: ${item.url}`);
              resolve();
            };
            img.src = item.url;
          });
        }
      });
      await Promise.all(workers);
      const validFrames = frames.filter(Boolean);
      console.log(`[FLOW-PRELOAD] Loaded ${clipKey} (${validFrames.length} frames)`);
      this.cache.set(clipKey, validFrames);
      this.loading.delete(clipKey);
      return validFrames;
    })();

    this.loading.set(clipKey, promise);
    return promise;
  }

  freeClip(clipKey) {
    const frames = this.cache.get(clipKey);
    if (frames) {
      frames.forEach(f => {
        if (f) {
          if (f.close) f.close();
          f.src = '';
        }
      });
      this.cache.delete(clipKey);
    }
  }

  freeAll() {
    for (const [key, frames] of this.cache.entries()) {
      frames.forEach(f => {
        if (f) {
          if (f.close) f.close();
          f.src = '';
        }
      });
    }
    this.cache.clear();
  }

  showCanvas() {
    this.canvas.style.display = 'block';
    this.canvas.style.opacity = '1';
    this.canvas.classList.add('active');
  }

  hideCanvas() {
    this.canvas.style.display = 'none';
    this.canvas.style.opacity = '0';
    this.canvas.classList.remove('active');
  }

  stop() {
    if (this.animId) {
      cancelAnimationFrame(this.animId);
      this.animId = null;
    }
    this.state = 'IDLE';
  }

  _playFrameArray(bitmaps, runId) {
    return new Promise((resolve) => {
      let idx = 0;
      let lastTs = performance.now();
      const frameInterval = this.frameInterval || 40.0;

      const loop = (now) => {
        if (this.activeRunId !== runId) {
          resolve();
          return;
        }

        const elapsed = now - lastTs;
        if (elapsed >= frameInterval - 2) {
          lastTs = now - (elapsed % frameInterval);
          if (idx < bitmaps.length) {
            const bmp = bitmaps[idx];
            if (bmp) this.ctx.drawImage(bmp, 0, 0, 1280, 720);
            idx++;
          }
          if (idx >= bitmaps.length) {
            resolve();
            return;
          }
        }
        this.animId = requestAnimationFrame(loop);
      };
      this.animId = requestAnimationFrame(loop);
    });
  }

  async playPeakFlow({ runId, clip11_1, onS10Start }) {
    this.activeRunId = runId;
    this.state = 'PLAYING';
    this.showCanvas();

    const crossfadeFrames = 8;

    // 1. Preload scene-10 in background immediately
    const s10Promise = this.preloadClip('scene-10');

    // 2. Load and play 11.1 up to (length - crossfadeFrames)
    const clipA = clip11_1 || await this.preloadClip('11.1');
    if (this.activeRunId !== runId) return;

    await this._playFrameArray(clipA.slice(0, Math.max(0, clipA.length - crossfadeFrames)), runId);
    if (this.activeRunId !== runId) return;

    // 3. Ensure scene-10 is ready
    const scene10Bitmaps = await s10Promise;
    if (this.activeRunId !== runId) return;

    // 4. Seamless crossfade 11.1 -> scene-10 (8 frames)
    await this._playCrossfade(clipA, scene10Bitmaps, crossfadeFrames, runId);
    if (this.activeRunId !== runId) return;

    // 5. Notify S10 loop start on very next tick
    if (onS10Start) onS10Start();

    // 6. Seamless loop of scene-10
    this.state = 'LOOPING';
    let s10Idx = crossfadeFrames % scene10Bitmaps.length;
    let s10LastTs = performance.now();
    const frameInterval = this.frameInterval || 40.0;

    const s10Loop = (now) => {
      if (this.activeRunId !== runId) return;

      const elapsed = now - s10LastTs;
      if (elapsed >= frameInterval - 2) {
        s10LastTs = now - (elapsed % frameInterval);
        const bmp = scene10Bitmaps[s10Idx];
        if (bmp) this.ctx.drawImage(bmp, 0, 0, 1280, 720);
        s10Idx = (s10Idx + 1) % scene10Bitmaps.length;
      }
      this.animId = requestAnimationFrame(s10Loop);
    };
    this.animId = requestAnimationFrame(s10Loop);
  }

  _playCrossfade(clipA, clipB, crossfadeFrames, runId) {
    return new Promise((resolve) => {
      let k = 0;
      let lastTs = performance.now();
      const frameInterval = this.frameInterval || 40.0;
      const N = crossfadeFrames;
      const offsetA = clipA.length - N;

      const loop = (now) => {
        if (this.activeRunId !== runId) {
          resolve();
          return;
        }

        const elapsed = now - lastTs;
        if (elapsed >= frameInterval - 2) {
          lastTs = now - (elapsed % frameInterval);
          if (k < N) {
            const bmpA = clipA[offsetA + k];
            const bmpB = clipB[k];
            const alpha = (k + 1.0) / (N + 1.0);

            if (bmpA) {
              this.ctx.globalAlpha = 1.0;
              this.ctx.drawImage(bmpA, 0, 0, 1280, 720);
            }
            if (bmpB) {
              this.ctx.globalAlpha = alpha;
              this.ctx.drawImage(bmpB, 0, 0, 1280, 720);
            }
            this.ctx.globalAlpha = 1.0;
            k++;
          }
          if (k >= N) {
            resolve();
            return;
          }
        }
        this.animId = requestAnimationFrame(loop);
      };
      this.animId = requestAnimationFrame(loop);
    });
  }

  async playHouseFlow({ runId, clip11_1, onTransition, onDone }) {
    this.activeRunId = runId;
    this.state = 'PLAYING';
    this.showCanvas();

    const crossfadeFrames = 8;

    // 1. Preload 11.2 and 11.3 concurrently
    const p11_2 = this.preloadClip('11.2');
    const p11_3 = this.preloadClip('11.3');

    // 2. Play 11.1 up to (length - crossfadeFrames)
    const clipA = clip11_1 || await this.preloadClip('11.1');
    if (this.activeRunId !== runId) return;

    await this._playFrameArray(clipA.slice(0, clipA.length - crossfadeFrames), runId);
    if (this.activeRunId !== runId) return;

    // Await 11.2 readiness before crossfade
    const clip11_2 = await p11_2;
    if (this.activeRunId !== runId) return;

    // 3. Seamless crossfade 11.1 -> 11.2 (8 frames)
    if (onTransition) onTransition('HOUSE_11_1', 'HOUSE_11_2', 'clip-11.1-ended');
    await this._playCrossfade(clipA, clip11_2, crossfadeFrames, runId);
    if (this.activeRunId !== runId) return;

    // Play 11.2 from crossfadeFrames up to (length - crossfadeFrames)
    await this._playFrameArray(clip11_2.slice(crossfadeFrames, clip11_2.length - crossfadeFrames), runId);
    if (this.activeRunId !== runId) return;

    // Await 11.3 readiness before crossfade
    const clip11_3 = await p11_3;
    if (this.activeRunId !== runId) return;

    // 4. Seamless crossfade 11.2 -> 11.3 (8 frames)
    if (onTransition) onTransition('HOUSE_11_2', 'HOUSE_11_3', 'clip-11.2-ended');
    await this._playCrossfade(clip11_2, clip11_3, crossfadeFrames, runId);
    if (this.activeRunId !== runId) return;

    // Play 11.3 from crossfadeFrames to end (holds last frame)
    await this._playFrameArray(clip11_3.slice(crossfadeFrames), runId);
    if (this.activeRunId !== runId) return;

    if (onDone) onDone();
  }
}

/* ============================================================================
   MASTER STORY CONTROLLER
   Orchestrates State Machine, Hotspot Delegation, Input Locking, and UI
   ============================================================================ */
class StoryController {
  constructor() {
    this.player = new DualBufferPlayer();
    this.overlays = new OverlayManager();
    this.sound = new WinterSoundEngine();
    this.snow = new SnowParticleEngine();

    this.narrativeCard = document.getElementById('narrative-card');
    this.sceneBadge    = document.getElementById('scene-badge');
    this.sceneTypeBadge= document.getElementById('scene-type-badge');
    this.sceneTitle    = document.getElementById('scene-title');
    this.sceneDesc     = document.getElementById('scene-desc');
    this.hintText      = document.getElementById('hint-text');
    this.timelineFill  = document.getElementById('timeline-fill');
    this.scrollPrompt  = document.getElementById('scroll-prompt');
    this.whiteFlash    = document.getElementById('white-flash-dip');
    this.replayBtn     = document.getElementById('btn-replay');
    this.clubsBtn      = document.getElementById('btn-clubs');
    this.scene14BackBtn= document.getElementById('btn-scene14-back');
    this.stageClubsBtn = document.getElementById('btn-stage-clubs');
    this.cardClubsBtn  = document.getElementById('btn-card-clubs');
    this.scene14Container = document.getElementById('scene-14-stage');
    this.stageS13Actions  = document.getElementById('stage-s13-actions');
    this.scene13CardActions = document.getElementById('scene13-actions');
    this.quadrantVideos   = document.querySelectorAll('.scene-14-video');

    // Modals & Flow Elements
    this.iciclePanel   = document.getElementById('icicle-panel');
    this.flowCanvas    = document.getElementById('flow-canvas');
    this.s8EndImg      = document.getElementById('s8-end-frame-img');
    this.flowPlayer    = this.flowCanvas ? new FlowSequencePlayer(this.flowCanvas) : null;
    this.flowStateMachine = new FlowStateMachine();
    this.flowContext   = null;
    this.flowRunId     = 0;
  }

  async init() {
    // 1. Fetch registries
    const [clipsRes, machineRes, hotspotRes] = await Promise.all([
      fetch('assets/clips.json'),
      fetch('assets/story-machine.json'),
      fetch('hotspot-data.json')
    ]);

    State.clipsData   = await clipsRes.json();
    State.machineData = await machineRes.json();
    State.hotspotData = await hotspotRes.json();

    // Check debug mode from URL (?debug=1 or ?debug=hotspots)
    const params = new URLSearchParams(window.location.search);
    State.debugMode = (params.get('debug') === '1');
    if (params.get('debug') === 'hotspots' || params.get('debug') === '1') {
      document.body.classList.add('debug-hotspots');
    }

    // 2. Setup layout and listeners
    LayoutEngine.updateLayout();
    window.addEventListener('resize', () => LayoutEngine.updateLayout());

    // 3. Setup interaction listeners
    this._setupHotspotDelegation();
    this._setupScrollScrubber();
    this._setupHUD();
    this._setupModals();

    if (State.debugMode) {
      this._setupDebugConsole();
    }

    // 4. Initialize snow-shed clip engine (ForestClips loaded from forestClips.js)
    if (window.ForestClips) {
      try {
        await window.ForestClips.init(() => this.player.activeVideo);
        this._logEvent('[FOREST] Snow-clip engine ready');
      } catch (err) {
        console.warn('[FOREST] ForestClips init error:', err);
      }
    }

    // 5. Start at S1_FOREST
    await this.enterState('S1_FOREST');

    if (window.GlassPortfolio) {
      window.GlassPortfolio.init().catch(err => console.warn('[GLASS] init error:', err));
    }

    document.body.classList.remove('loading-state');
    this._logEvent('System initialized in S1_FOREST');
  }

  _logEvent(msg) {
    const timestamp = new Date().toLocaleTimeString();
    const entry = `[${timestamp}] ${msg}`;
    State.recentEvents.unshift(entry);
    if (State.recentEvents.length > 10) State.recentEvents.pop();

    if (State.debugMode) {
      const list = document.getElementById('dbg-event-log');
      if (list) {
        list.innerHTML = State.recentEvents.map((e) => `<li>${e}</li>`).join('');
      }
    }
  }

  /* --------------------------------------------------------------------------
     INPUT LOCKING (Global Rule R3)
     -------------------------------------------------------------------------- */
  lockInput(reason = '') {
    State.isInputLocked = true;
    State.isTransitioning = true;
    this._logEvent(`[INPUT LOCK] ${reason}`);
    this._updateDebugUI();
  }

  unlockInput(reason = '') {
    State.isInputLocked = false;
    State.isTransitioning = false;
    this._logEvent(`[INPUT UNLOCK] ${reason}`);
    this._updateDebugUI();
  }

  _updateDebugUI() {
    if (!State.debugMode) return;
    const lockPill = document.getElementById('dbg-input-lock-pill');
    if (lockPill) {
      if (State.isInputLocked) {
        lockPill.textContent = 'LOCKED';
        lockPill.className = 'badge-pill lock';
      } else {
        lockPill.textContent = 'UNLOCKED';
        lockPill.className = 'badge-pill unlock';
      }
    }
    const dbgState = document.getElementById('dbg-state');
    if (dbgState) dbgState.textContent = State.currentState;

    const dbgClip = document.getElementById('dbg-clip');
    if (dbgClip) dbgClip.textContent = State.currentClipId;

    const dbgRole = document.getElementById('dbg-role');
    if (dbgRole) dbgRole.textContent = State.currentRole;
  }

  _showChapter6Buttons() {
    const stageGroup = document.getElementById('stage-ch6-buttons');
    if (stageGroup) stageGroup.style.display = 'flex';
    const stageMtn = document.getElementById('btn-stage-travel-mountains');
    if (stageMtn) stageMtn.style.display = 'inline-flex';
    const stageHome = document.getElementById('btn-stage-travel-home');
    if (stageHome) stageHome.style.display = 'inline-flex';

    const hudMtn = document.getElementById('btn-hud-travel-mountains');
    if (hudMtn) hudMtn.style.display = 'inline-flex';
    const hudHome = document.getElementById('btn-hud-travel-home');
    if (hudHome) hudHome.style.display = 'inline-flex';

    const cardActions = document.getElementById('chapter6-actions');
    if (cardActions) cardActions.style.display = 'flex';
  }

  _hideChapter6Buttons() {
    const stageGroup = document.getElementById('stage-ch6-buttons');
    if (stageGroup) stageGroup.style.display = 'none';
    const hudMtn = document.getElementById('btn-hud-travel-mountains');
    if (hudMtn) hudMtn.style.display = 'none';
    const hudHome = document.getElementById('btn-hud-travel-home');
    if (hudHome) hudHome.style.display = 'none';
    const cardActions = document.getElementById('chapter6-actions');
    if (cardActions) cardActions.style.display = 'none';
  }

  /* --------------------------------------------------------------------------
     FINITE STATE TRANSITION ORCHESTRATOR
     -------------------------------------------------------------------------- */
  async enterState(stateName) {
    const stateConfig = State.machineData?.states[stateName];
    if (!stateConfig) {
      console.error(`[FSM] Unknown state: ${stateName}`);
      return;
    }

    State.currentState = stateName;
    console.log(`[STATE] Entering state -> ${stateName}`);
    this._logEvent(`[STATE] Entered ${stateName}`);

    if (window.GlassPortfolio && stateName !== 'S12_INTERIOR') {
      window.GlassPortfolio.onStateChange(stateName);
    }

    // Update Hotspot Visibility
    this._activateHotspotGroup(stateName);

    // Update Narrative Card
    this._updateNarrativeCard(stateName);

    // Update Replay Button visibility on finale or scene 14
    if (this.replayBtn) {
      this.replayBtn.style.display = (stateName === 'S13_FINALE' || stateName === 'S14_CLUBS') ? 'flex' : 'none';
    }

    // Scene 13 "Clubs" Buttons visibility
    const isS13 = (stateName === 'S13_FINALE');
    if (this.clubsBtn) this.clubsBtn.style.display = isS13 ? 'inline-flex' : 'none';
    if (this.stageS13Actions) this.stageS13Actions.style.display = isS13 ? 'flex' : 'none';
    if (this.scene13CardActions) this.scene13CardActions.style.display = isS13 ? 'flex' : 'none';

    // Scene 14 visibility & synchronized playback
    const isS14 = (stateName === 'S14_CLUBS');
    const hudBottom = document.getElementById('hud-bottom');
    if (hudBottom) {
      hudBottom.style.display = isS14 ? 'none' : 'flex';
    }

    if (this.scene14BackBtn) this.scene14BackBtn.style.display = isS14 ? 'inline-flex' : 'none';
    if (this.scene14Container) {
      if (isS14) {
        this.scene14Container.style.display = 'flex';
        requestAnimationFrame(() => {
          this.scene14Container.classList.add('visible');
        });
        this.startScene14VideosInSync();
      } else {
        this.scene14Container.classList.remove('visible');
        const vids = document.querySelectorAll('.scene-14-video');
        vids.forEach(v => {
          try { v.pause(); } catch (e) {}
        });
        setTimeout(() => {
          if (State.currentState !== 'S14_CLUBS') {
            this.scene14Container.style.display = 'none';
          }
        }, 650);
      }
    }

    // Hide scene-12 end-frame overlay whenever we leave that state
    if (stateName !== 'S12_INTERIOR') {
      const s12End = document.getElementById('s12-end-frame-img');
      if (s12End) s12End.style.display = 'none';
    }

    // Handle overlays (Scene 1 pine trees) — behind flag per specification
    if (FOREST_LEGACY_OVERLAYS) {
      if (stateName === 'S1_FOREST') {
        this.overlays.initTreeOverlays();
        this.overlays.showOverlays();
      } else {
        this.overlays.hideOverlays();
      }
    } else {
      this.overlays.hideOverlays();
    }

    // Activate / deactivate snow-shed hotspots
    if (window.ForestClips) {
      if (stateName === 'S1_FOREST') {
        window.ForestClips.activate();
      } else {
        window.ForestClips.deactivate();
      }
    }

    if (stateName === 'S8_HOLD') {
      State.currentState = 'S8_HOLD';
      this.flowStateMachine.setFlowState('S8_HOLD', 'scene-8-ended');
      this._activateHotspotGroup('S8_HOLD');

      // Pause background buffers so main scene video never bleeds through
      if (this.player.bufferA) this.player.bufferA.pause();
      if (this.player.bufferB) this.player.bufferB.pause();

      if (this.flowPlayer) {
        this.flowPlayer.stop();
        this.flowPlayer.hideCanvas();
        // Priority preload 11.1 first so Go to House triggers with 0ms latency
        this.flowPlayer.preloadClip('11.1').then(() => {
          this.flowPlayer.preloadClip('11.2');
          this.flowPlayer.preloadClip('11.3');
        });
        this.flowPlayer.preloadClip('scene-10');
      }

      // Show Chapter 6 buttons (Travel to Mountains & Travel to Home)
      this._showChapter6Buttons();

      // Keep Scene 8 END frame decoded & visible (Rule R5)
      const s8EndImg = document.getElementById('s8-end-frame-img');
      if (s8EndImg) {
        s8EndImg.style.display = 'block';
      }

      this.unlockInput('Arrived in S8_HOLD (options live)');
      return;
    }

    // Hide Chapter 6 buttons for all other states
    this._hideChapter6Buttons();

    // When entering S_HOUSE, stop the flow canvas and hide both video
    // buffers so the house-still image shows cleanly above everything
    if (stateName === 'S_HOUSE') {
      if (this.flowPlayer) {
        this.flowPlayer.stop();
        this.flowPlayer.hideCanvas();
      }
      // Force both buffers to opacity 0 so no previous frame bleeds through
      if (this.player.bufferA) this.player.bufferA.style.opacity = '0';
      if (this.player.bufferB) this.player.bufferB.style.opacity = '0';
    }

    // S14_CLUBS: The 2x2 grid is active. Main stage buffer is idle.
    if (stateName === 'S14_CLUBS') {
      if (this.player.bufferA) this.player.bufferA.pause();
      if (this.player.bufferB) this.player.bufferB.pause();
      this.unlockInput('Arrived in S14_CLUBS');
      return;
    }

    // Play state idle clip
    const idleRole = stateConfig.idleRole;
    const loopType = stateConfig.loopType || 'seamless';
    const holdLastFrame = (stateConfig.loopType === 'hold_last_frame');

    await this.player.playRole(idleRole, {
      loopType,
      holdLastFrame
    });

    // After buffer swap keep ForestClips pointed at the new active video
    if (window.ForestClips && stateName === 'S1_FOREST') {
      window.ForestClips.setMainVideo(this.player.activeVideo);
    }

    // S12_INTERIOR: cabin video walks inside to the workbench (frame 172 at 5.70s).
    // Lock on frame 172 (the blueprint workbench) without looping or repeating.
    if (stateName === 'S12_INTERIOR') {
      const vid = this.player.activeVideo;
      const s12End = document.getElementById('s12-end-frame-img');
      const targetTime = 5.70; // Exact timestamp of frame 172 workbench closeup

      const showEndFrame = () => {
        try {
          if (!vid.paused) vid.pause();
          vid.currentTime = targetTime;
        } catch (e) {}
        if (s12End) {
          s12End.style.display = 'block';
          s12End.style.opacity = '1';
        }
        if (window.GlassPortfolio) {
          window.GlassPortfolio.onStateChange('S12_INTERIOR');
        }
      };

      // Guard: only lock once
      let locked = false;
      const lock = () => {
        if (locked) return;
        locked = true;
        clearInterval(pollId);
        vid.removeEventListener('ended', lock);
        vid.removeEventListener('timeupdate', onTimeUpdate);
        showEndFrame();
      };

      // 1. Listen for the native ended event
      vid.addEventListener('ended', lock);

      // 2. timeupdate: lock as soon as currentTime reaches target 5.70s (frame 172)
      const onTimeUpdate = () => {
        if (vid.currentTime >= targetTime || (vid.duration && vid.currentTime >= vid.duration - 0.15)) {
          lock();
        }
      };
      vid.addEventListener('timeupdate', onTimeUpdate);

      // 3. Fallback poll every 50ms (never lock if currentTime < 2.0)
      const pollId = setInterval(() => {
        if (vid.currentTime >= targetTime || (vid.currentTime > 2.0 && vid.paused) || (vid.duration && vid.currentTime >= vid.duration - 0.15)) {
          lock();
        }
      }, 50);
    }

    this.unlockInput(`Arrived in ${stateName} idle`);
  }

  async runTransitionSequence(steps, nextState) {
    if (State.isInputLocked) return;
    this.lockInput(`Transitioning via: ${steps.join(' -> ')}`);

    for (const stepRole of steps) {
      // Check for CAVE_EXIT connector placeholder (Task 8: dip through white)
      if (stepRole === 'cave-exit-connector') {
        this._logEvent('[CONNECTOR] Executing CAVE_EXIT white glare dip (700ms)');
        if (this.whiteFlash) this.whiteFlash.classList.add('active');
        await new Promise((r) => setTimeout(r, 350));
        if (this.whiteFlash) this.whiteFlash.classList.remove('active');
        await new Promise((r) => setTimeout(r, 350));
        continue;
      }

      // Normal clip transition
      this._logEvent(`[PLAY TRANSITION] ${stepRole}`);
      if (stepRole === 'reindeer-cliff-run') {
        this.flowStateMachine.setFlowState('S8_PLAYING', 'reindeer-cliff-run-start');
        if (this.flowPlayer) this.flowPlayer.preloadClip('11.1');
      }
      await new Promise((resolve) => {
        this.player.playRole(stepRole, {
          loop: false,
          loopType: 'none',
          holdLastFrame: (stepRole === 'reindeer-cliff-run'),
          onEnded: resolve
        });
      });
    }

    if (nextState) {
      if (nextState === 'S8_HOLD') {
        this.flowStateMachine.transition('S8_PLAYING', 'S8_HOLD', 'scene-8-ended');
      }
      await this.enterState(nextState);
    } else {
      this.unlockInput('Transition complete');
    }
  }

  /* --------------------------------------------------------------------------
     HOTSPOT HIT-TESTING & DELEGATION (Task 1 & 2)
     -------------------------------------------------------------------------- */
  _setupHotspotDelegation() {
    const svg = document.getElementById('hotspot-svg');
    if (!svg) return;

    svg.addEventListener('click', (e) => {
      // Find closest hotspot element
      const target = e.target.closest('.hs-zone');
      if (!target) return;

      // Rule R3: Input lock check
      if (State.isInputLocked) {
        console.log('[INPUT] Ignored click — input locked');
        return;
      }

      const action = target.getAttribute('data-action');
      const hotspotId = target.getAttribute('data-hotspot-id');

      this._logEvent(`[CLICK] Hotspot: ${hotspotId || target.id} (action: ${action})`);

      // Dispatch action
      this._handleAction(action, target);
    });

    // Direct binding for Travel to Mountains buttons (HTML triggers 11.1 -> Scene 10)
    ['btn-stage-travel-mountains', 'btn-hud-travel-mountains', 'btn-card-travel-mountains'].forEach((btnId) => {
      const el = document.getElementById(btnId);
      if (el) {
        const trigger = (e) => {
          e.stopPropagation();
          e.preventDefault();
          this.handlePeakClick('1');
        };
        el.addEventListener('click', trigger);
        el.addEventListener('keydown', (e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            trigger(e);
          }
        });
      }
    });

    // Direct binding for Travel to Home buttons (HTML triggers 11.1 -> 11.2 -> 11.3)
    ['btn-stage-travel-home', 'btn-hud-travel-home', 'btn-card-travel-home', 'btn-stage-goto-house', 'btn-hud-goto-house'].forEach((btnId) => {
      const el = document.getElementById(btnId);
      if (el) {
        const trigger = (e) => {
          e.stopPropagation();
          e.preventDefault();
          this.handleHouseClick();
        };
        el.addEventListener('click', trigger);
        el.addEventListener('keydown', (e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            trigger(e);
          }
        });
      }
    });

    // House button handler (for regression suite / direct SVG trigger)
    const houseBtn = document.getElementById('hs-btn-goto-house');
    if (houseBtn) {
      houseBtn.addEventListener('click', (e) => {
        e.stopPropagation();
        e.preventDefault();
        this.handleHouseClick();
      });
    }
  }

  _activateHotspotGroup(stateName) {
    // Hide all groups
    document.querySelectorAll('.hs-group').forEach((g) => {
      g.style.display = 'none';
      g.classList.remove('active');
    });

    // Show active group
    const activeGroup = document.getElementById(`hs-group-${stateName}`);
    if (activeGroup) {
      activeGroup.style.display = 'block';
      activeGroup.classList.add('active');
    }

    // Update mask image in debug mode
    if (State.showMasks) {
      this._updateMaskOverlay(stateName);
    }
  }

  _handleAction(action, target) {
    this.sound.playWhoosh();

    switch (action) {
      case 'advance-to-chase':
        // S3 Raccoon Dog click -> vortex (3.1), meeting (5) -> S7_REINDEER
        this.runTransitionSequence(['portal-vortex', 'creatures-meeting'], 'S7_REINDEER');
        break;

      case 'advance-to-ice-cave':
        // S7 Reindeer click -> reindeer-run-to-cave (6) -> S9_ICE_CAVE
        this.runTransitionSequence(['reindeer-run-to-cave'], 'S9_ICE_CAVE');
        break;

      case 'open-icicle-panel': {
        // S9 Icicle click -> open detail modal (no advance)
        const iid = target.getAttribute('data-icicle-id');
        this._openIciclePanel(iid);
        break;
      }

      case 'advance-from-cave':
        // S9 backdrop click -> CAVE_EXIT connector (white dip), cliff run (8) -> S8_HOLD
        this.runTransitionSequence(['cave-exit-connector', 'reindeer-cliff-run'], 'S8_HOLD');
        break;

      case 'flow-bird-click':
        this.handleBirdClick();
        break;

      case 'flow-peak-click': {
        const peakId = target.getAttribute('data-peak-id') || '1';
        this.handlePeakClick(peakId);
        break;
      }

      case 'flow-house-click':
        this.handleHouseClick();
        break;

      case 'enter-house':
        // S_HOUSE door click -> white-dip connector -> S12_INTERIOR
        // (enter-house-transition clip is a placeholder with null file;
        //  use the cave-exit-connector white-dip to bridge the scenes cleanly)
        this.runTransitionSequence(['cave-exit-connector'], 'S12_INTERIOR');
        break;

      case 'advance-to-finale':
        // S12 Window OR Table click -> blueprint-transition (13a), celestial-finale (13b) -> S13_FINALE
        // Hide the scene-12 end-frame overlay before the transition plays
        {
          const s12End = document.getElementById('s12-end-frame-img');
          if (s12End) s12End.style.display = 'none';
        }
        this.runTransitionSequence(['blueprint-transition', 'celestial-finale'], 'S13_FINALE');
        break;

      default:
        console.log(`[ACTION] Unhandled action: ${action}`);
    }
  }

  /* --------------------------------------------------------------------------
     FLOW INTERACTION HANDLERS (Section 5)
     -------------------------------------------------------------------------- */
  async handleBirdClick() {
    if (State.isInputLocked) return;
    if (this.flowStateMachine.getState() !== 'S8_HOLD') return;

    this.lockInput('Bird taking flight');
    this.flowStateMachine.transition('S8_HOLD', 'BIRD_ANIM_A', 'bird-hotspot-click');

    const s8End = document.getElementById('s8-end-frame-img');
    if (s8End) s8End.style.display = 'none';
    this._activateHotspotGroup(null);

    // Play BIRD_ANIM_A (scene-9.1.webm)
    await new Promise((resolve) => {
      this.player.playRole('bird-anim-a', {
        loop: false,
        loopType: 'none',
        holdLastFrame: false,
        onEnded: resolve
      });
    });

    // Transition BIRD_ANIM_A -> S9
    this.flowStateMachine.transition('BIRD_ANIM_A', 'S9', 'bird-anim-a-ended');

    // Play Scene 9 (scene-9.webm)
    await new Promise((resolve) => {
      this.player.playRole('bird-scene-9', {
        loop: false,
        loopType: 'none',
        holdLastFrame: true,
        onEnded: resolve
      });
    });

    // Transition S9 -> S9_DONE
    this.flowStateMachine.transition('S9', 'S9_DONE', 'scene-9-ended');
    this.unlockInput('Bird flight complete');
  }

  async handlePeakClick(peakId) {
    if (State.isInputLocked) return;
    if (this.flowStateMachine.getState() !== 'S8_HOLD') {
      if (State.currentState === 'S8_HOLD') {
        this.flowStateMachine.setFlowState('S8_HOLD', 'reset-for-peak-click');
      } else {
        return;
      }
    }

    // Pause background buffers so main scene video never bleeds through
    if (this.player.bufferA) this.player.bufferA.pause();
    if (this.player.bufferB) this.player.bufferB.pause();

    const runId = Date.now();
    this.flowRunId = runId;
    this.flowContext = 'PEAK';
    this.lockInput(`Flying to Peak ${peakId}`);

    State.currentState = 'PEAK_11_1';
    this.flowStateMachine.transition('S8_HOLD', 'PEAK_11_1', `peak-${peakId}-click`);

    this._activateHotspotGroup(null);
    this._hideChapter6Buttons();

    // Show canvas with opacity 1
    this.flowPlayer.showCanvas();

    // Preload 11.1
    const clip11_1 = await this.flowPlayer.preloadClip('11.1');
    if (this.flowRunId !== runId) return;

    if (clip11_1 && clip11_1[0]) {
      this.flowPlayer.ctx.drawImage(clip11_1[0], 0, 0, 1280, 720);
    }
    const s8End = document.getElementById('s8-end-frame-img');
    if (s8End) s8End.style.display = 'none';

    await this.flowPlayer.playPeakFlow({
      runId,
      clip11_1,
      onS10Start: () => {
        if (this.flowRunId !== runId) return;
        State.currentState = 'S10_LOOP';
        this.flowStateMachine.transition('PEAK_11_1', 'S10_LOOP', 'clip-11.1-ended');
        const backBtn = document.getElementById('btn-scene10-back');
        if (backBtn) backBtn.style.display = 'inline-flex';
        this.unlockInput('Looping at peak summit');
        if (window.GlassPortfolio) {
          window.GlassPortfolio.onStateChange('S10_LOOP');
        }
      }
    });
  }

  handleScene10Back() {
    if (this.flowStateMachine.getState() !== 'S10_LOOP') return;

    this.flowStateMachine.transition('S10_LOOP', 'S8_HOLD', 'back-button-click');

    const backBtn = document.getElementById('btn-scene10-back');
    if (backBtn) {
      backBtn.blur();
      backBtn.style.display = 'none';
    }

    if (window.GlassPortfolio) {
      window.GlassPortfolio.onStateChange('S8_HOLD');
    }

    // Rule R5: Show Scene 8 END frame BEFORE hiding Scene 10 canvas
    const s8End = document.getElementById('s8-end-frame-img');
    if (s8End) s8End.style.display = 'block';

    this.flowPlayer.stop();
    this.flowPlayer.hideCanvas();
    // Keep decoded in memory for instant reuse if another peak is clicked

    this.flowContext = null;
    State.currentState = 'S8_HOLD';
    this._activateHotspotGroup('S8_HOLD');
    this._updateNarrativeCard('S8_HOLD');
    this._showChapter6Buttons();
    this.unlockInput('Returned to S8_HOLD');
  }

  async handleHouseClick() {
    if (State.isInputLocked) {
      console.log('[HOUSE-CLICK] Ignored: input is locked');
      return;
    }
    if (this.flowStateMachine.getState() !== 'S8_HOLD') {
      if (State.currentState === 'S8_HOLD') {
        this.flowStateMachine.setFlowState('S8_HOLD', 'reset-for-house-click');
      } else {
        console.warn(`[HOUSE-CLICK] Not in S8_HOLD: flow=${this.flowStateMachine.getState()}, current=${State.currentState}`);
        return;
      }
    }

    // Pause background video buffers
    if (this.player.bufferA) this.player.bufferA.pause();
    if (this.player.bufferB) this.player.bufferB.pause();

    const runId = Date.now();
    this.flowRunId = runId;
    this.flowContext = 'HOUSE';
    this.lockInput('Traveling to cabin');

    // Rule R1: Transition immediately upon click so state is active
    State.currentState = 'HOUSE_11_1';
    this.flowStateMachine.transition('S8_HOLD', 'HOUSE_11_1', 'goto-house-click');
    this._activateHotspotGroup(null);
    this._hideChapter6Buttons();

    // Show canvas with opacity 1
    this.flowPlayer.showCanvas();

    // Preload 11.1 if not already cached
    const clip11_1 = await this.flowPlayer.preloadClip('11.1');
    if (this.flowRunId !== runId) return;

    // Paint frame 0 immediately onto canvas before hiding s8End
    if (clip11_1 && clip11_1[0]) {
      this.flowPlayer.ctx.drawImage(clip11_1[0], 0, 0, 1280, 720);
    }

    const s8End = document.getElementById('s8-end-frame-img');
    if (s8End) s8End.style.display = 'none';

    await this.flowPlayer.playHouseFlow({
      runId,
      clip11_1,
      onTransition: (from, to, source) => {
        if (this.flowRunId !== runId) return;
        State.currentState = to;
        this.flowStateMachine.transition(from, to, source);
      },
      onDone: () => {
        if (this.flowRunId !== runId) return;
        this.flowStateMachine.transition('HOUSE_11_3', 'HOUSE_DONE', 'clip-11.3-ended');
        // Enter S_HOUSE so the door hotspot becomes visible and clickable
        this.enterState('S_HOUSE');
      }
    });
  }

  /* --------------------------------------------------------------------------
     ICICLE PANEL MODAL
     -------------------------------------------------------------------------- */
  _openIciclePanel(icicleId) {
    const details = State.hotspotData?.icicleDetails[icicleId];
    if (!details || !this.iciclePanel) return;

    this.sound.playChime();

    const titleEl = document.getElementById('icicle-panel-title');
    const subEl   = document.getElementById('icicle-panel-subtitle');
    const bodyEl  = document.getElementById('icicle-panel-body');
    const tagsEl  = document.getElementById('icicle-panel-tags');

    if (titleEl) titleEl.textContent = details.title;
    if (subEl)   subEl.textContent   = details.subtitle;
    if (bodyEl)  bodyEl.textContent  = details.desc;
    if (tagsEl) {
      tagsEl.innerHTML = details.tags.map((t) => `<span class="tag-badge">${t}</span>`).join('');
    }

    this.iciclePanel.classList.add('open');
    this.iciclePanel.setAttribute('aria-hidden', 'false');
    this._logEvent(`[PANEL] Opened icicle detail: ${details.title}`);
  }

  _closeIciclePanel() {
    if (!this.iciclePanel) return;
    this.iciclePanel.classList.remove('open');
    this.iciclePanel.setAttribute('aria-hidden', 'true');
  }

  _setupModals() {
    const icClose = document.getElementById('icicle-panel-close');
    if (icClose) {
      icClose.addEventListener('click', () => this._closeIciclePanel());
    }

    const scene10Back = document.getElementById('btn-scene10-back');
    if (scene10Back) {
      scene10Back.addEventListener('click', () => this.handleScene10Back());
    }
  }

  /* --------------------------------------------------------------------------
     SCROLL SCRUBBER (Scenes 1-3)
     -------------------------------------------------------------------------- */
  _setupScrollScrubber() {
    const scrollDriver = document.getElementById('scroll-driver');
    if (scrollDriver) {
      // Set scroll range
      scrollDriver.style.height = `${window.innerHeight * 4}px`;
    }

    window.addEventListener('scroll', () => {
      // ONLY allow scroll scrubbing during S1_FOREST and S2_PATH — strictly ignore in all other scenes!
      if (State.currentState !== 'S1_FOREST' && State.currentState !== 'S2_PATH') {
        return;
      }

      // Ignore scroll scrubbing while snow clip is actively playing or taking over timeline
      if (window.ForestClips && (window.ForestClips.blockBufferSwap || window.ForestClips.isTakingOverTimeline)) return;

      // Rule R4: Auto-advance guard: in S8_HOLD and flow states, scroll must NOT advance the scene
      if (this.flowStateMachine && this.flowStateMachine.isFlowActive()) return;

      const maxScroll = (document.documentElement.scrollHeight - window.innerHeight) || 1;
      const progress = Math.min(Math.max(window.scrollY / maxScroll, 0), 1);
      State.scrollProgress = progress;

      if (progress < 0.45) {
        if (State.currentState !== 'S1_FOREST') {
          this.enterState('S1_FOREST');
        }
        const clip = State.clipsData?.clips['forest-establishing'];
        const duration = clip ? clip.endTime : 7.0;
        this.player.scrub((progress / 0.45) * duration);
      } else if (progress < 0.90) {
        if (State.currentState !== 'S2_PATH') {
          this.enterState('S2_PATH');
        }
        const clip = State.clipsData?.clips['clearing-path'];
        const duration = clip ? clip.endTime : 6.4;
        this.player.scrub(((progress - 0.45) / 0.45) * duration);
      } else {
        // Reached stump clearing -> advance into click-gated S3_STUMP
        if (State.currentState !== 'S3_STUMP') {
          this.enterState('S3_STUMP');
        }
      }
    }, { passive: true });
  }

  /* --------------------------------------------------------------------------
     HUD & CHAPTERS
     -------------------------------------------------------------------------- */
  _setupHUD() {
    // Quality toggle
    const qualityBtn = document.getElementById('btn-quality');
    const qualityLbl = document.getElementById('quality-label');
    if (qualityBtn && qualityLbl) {
      qualityBtn.addEventListener('click', () => {
        State.quality = (State.quality === 'hq') ? 'lq' : 'hq';
        qualityLbl.textContent = State.quality.toUpperCase();
        this._logEvent(`[QUALITY] Toggled to ${State.quality.toUpperCase()}`);
      });
    }

    // Audio toggle
    const audioBtn = document.getElementById('btn-audio');
    const audioIcon = document.getElementById('audio-icon');
    if (audioBtn && audioIcon) {
      audioBtn.addEventListener('click', () => {
        State.soundEnabled = !State.soundEnabled;
        audioIcon.textContent = State.soundEnabled ? '🔊' : '🔇';
        this.sound.toggle(State.soundEnabled);
        this._logEvent(`[AUDIO] Ambient audio ${State.soundEnabled ? 'enabled' : 'disabled'}`);
      });
    }

    // Chapters Drawer
    const chapBtn   = document.getElementById('btn-chapters');
    const drawer    = document.getElementById('chapters-drawer');
    const closeBtn  = document.getElementById('drawer-close');
    const chapList  = document.getElementById('chapters-list');

    if (chapBtn && drawer) {
      chapBtn.addEventListener('click', () => {
        drawer.classList.add('open');
        drawer.setAttribute('aria-hidden', 'false');
      });
    }
    if (closeBtn && drawer) {
      closeBtn.addEventListener('click', () => {
        drawer.classList.remove('open');
        drawer.setAttribute('aria-hidden', 'true');
      });
    }

    // Future Projects HUD Button
    const hudFutureBtn = document.getElementById('btn-future-projects');
    if (hudFutureBtn) {
      hudFutureBtn.addEventListener('click', () => {
        if (window.GlassPortfolio) {
          window.GlassPortfolio._openSubPanel('panel-future-projects');
        }
      });
    }

    // Populate chapters
    if (chapList) {
      const states = Object.keys(NARRATIVE_MANIFEST);
      const chapterItemsHtml = states.map((st, idx) => {
        const meta = NARRATIVE_MANIFEST[st];
        return `
          <li class="chapter-item" data-jump-state="${st}">
            <span class="chapter-num">CHAPTER ${idx + 1}</span>
            <div class="chapter-name">${meta.title}</div>
          </li>
        `;
      }).join('');

      const futureChapterHtml = `
        <li class="chapter-item future-chapter-item" data-action="open-future-projects">
          <span class="chapter-num" style="color:#f59e0b;">FINALE SPECIAL</span>
          <div class="chapter-name">🚧 Future Projects / Coming Soon</div>
        </li>
      `;

      chapList.innerHTML = chapterItemsHtml + futureChapterHtml;

      chapList.addEventListener('click', (e) => {
        const futureItem = e.target.closest('[data-action="open-future-projects"]');
        if (futureItem) {
          drawer.classList.remove('open');
          if (window.GlassPortfolio) {
            window.GlassPortfolio._openSubPanel('panel-future-projects');
          }
          return;
        }
        const item = e.target.closest('.chapter-item');
        if (!item) return;
        const targetState = item.getAttribute('data-jump-state');
        if (targetState) {
          drawer.classList.remove('open');
          this.enterState(targetState);
        }
      });
    }

    // Replay Button
    if (this.replayBtn) {
      this.replayBtn.addEventListener('click', () => {
        if (State.currentState === 'S14_CLUBS') {
          this.replayScene14Videos();
          return;
        }
        this._logEvent('[REPLAY] Restarting journey from S1_FOREST');
        window.scrollTo({ top: 0, behavior: 'smooth' });
        this.enterState('S1_FOREST');
      });
    }

    // Clubs Button click handlers -> Transition to Scene 14
    const onClubsClicked = () => {
      this._logEvent('[CLUBS] Button clicked -> Transitioning to Scene 14');
      this.transitionToScene14();
    };
    if (this.clubsBtn)      this.clubsBtn.addEventListener('click', onClubsClicked);
    if (this.stageClubsBtn)  this.stageClubsBtn.addEventListener('click', onClubsClicked);
    if (this.cardClubsBtn)   this.cardClubsBtn.addEventListener('click', onClubsClicked);

    // Back from Scene 14 to Scene 13
    if (this.scene14BackBtn) {
      this.scene14BackBtn.addEventListener('click', () => {
        this._logEvent('[CLUBS] Back button clicked -> Returning to Scene 13');
        this.transitionBackToScene13();
      });
    }
  }

  async transitionToScene14() {
    if (State.isInputLocked) return;
    this.lockInput('Transitioning to Scene 14 Clubs Showcase');
    await this.enterState('S14_CLUBS');
    this.unlockInput('Transition to Scene 14 complete');
  }

  async transitionBackToScene13() {
    if (State.isInputLocked) return;
    this.lockInput('Returning to Scene 13 Celestial Finale');
    await this.enterState('S13_FINALE');
    this.unlockInput('Returned to Scene 13');
  }

  async startScene14VideosInSync() {
    this._logEvent('[CLUBS] Preparing 4 videos for synchronized playback');
    const vids = Array.from(document.querySelectorAll('.scene-14-video'));
    if (!vids.length) return;

    // Ensure all 4 are muted, looping, and reset
    vids.forEach(v => {
      v.muted = true;
      v.loop = true;
      try { v.currentTime = 0; } catch (e) {}
    });

    // Wait until all 4 have loaded (canplaythrough or loadeddata)
    const loadPromises = vids.map(v => {
      if (v.readyState >= 3) {
        return Promise.resolve();
      }
      return new Promise(resolve => {
        let done = false;
        const finish = () => {
          if (!done) {
            done = true;
            v.removeEventListener('canplaythrough', finish);
            v.removeEventListener('loadeddata', finish);
            resolve();
          }
        };
        v.addEventListener('canplaythrough', finish, { once: true });
        v.addEventListener('loadeddata', finish, { once: true });
        setTimeout(finish, 1200); // Failsafe timeout
      });
    });

    await Promise.all(loadPromises);

    if (State.currentState !== 'S14_CLUBS') return;

    // Call play() on all 4 together at the exact same moment from 0:00
    vids.forEach(v => {
      try {
        v.currentTime = 0;
        v.play().catch(e => console.warn('[CLUBS] Play warning:', e));
      } catch (e) {}
    });
  }

  replayScene14Videos() {
    this._logEvent('[CLUBS] Replay: Restarting all four videos from 0:00 together');
    const vids = Array.from(document.querySelectorAll('.scene-14-video'));
    vids.forEach(v => {
      try {
        v.muted = true;
        v.currentTime = 0;
        v.play().catch(e => console.warn('[CLUBS] Play warning:', e));
      } catch (e) {}
    });
  }

  _updateNarrativeCard(stateName) {
    const meta = NARRATIVE_MANIFEST[stateName];
    if (!meta) return;

    if (this.sceneBadge)     this.sceneBadge.textContent     = meta.badge;
    if (this.sceneTypeBadge) this.sceneTypeBadge.textContent = meta.type;
    if (this.sceneTitle)     this.sceneTitle.textContent     = meta.title;
    if (this.sceneDesc)      this.sceneDesc.textContent      = meta.desc;
    if (this.hintText)       this.hintText.textContent       = meta.hint;

    // Timeline bar calculation
    const states = Object.keys(NARRATIVE_MANIFEST);
    const idx = states.indexOf(stateName);
    const pct = Math.round(((idx + 1) / states.length) * 100);
    if (this.timelineFill) {
      this.timelineFill.style.width = `${pct}%`;
    }

    // Hide scroll cue after Scene 2
    if (this.scrollPrompt) {
      this.scrollPrompt.style.opacity = (stateName === 'S1_FOREST' || stateName === 'S2_PATH') ? '1' : '0';
    }
  }

  /* --------------------------------------------------------------------------
     DEBUG CONSOLE (?debug=1) (Task 2.10 & Task 3)
     -------------------------------------------------------------------------- */
  _setupDebugConsole() {
    const panel = document.getElementById('debug-panel');
    if (!panel) return;
    panel.style.display = 'flex';
    document.body.classList.add('debug-outlines');

    // Populate Jump Select
    const jumpSelect = document.getElementById('dbg-jump-select');
    if (jumpSelect) {
      const states = Object.keys(State.machineData?.states || {});
      jumpSelect.innerHTML = states.map((s) => `<option value="${s}">${s}</option>`).join('');
      jumpSelect.addEventListener('change', (e) => {
        this.enterState(e.target.value);
      });
    }

    // Populate Transition Select
    const transSelect = document.getElementById('dbg-trans-select');
    const playTransBtn = document.getElementById('dbg-btn-play-trans');
    if (transSelect && playTransBtn) {
      const clips = Object.keys(State.clipsData?.clips || {});
      transSelect.innerHTML = clips.map((c) => `<option value="${c}">${c}</option>`).join('');
      playTransBtn.addEventListener('click', () => {
        const role = transSelect.value;
        this._logEvent(`[DEBUG] Playing single clip for inspection: ${role}`);
        this.player.playRole(role, { loop: false, loopType: 'none' });
      });
    }

    // Toggles
    const chkOutlines = document.getElementById('dbg-chk-outlines');
    if (chkOutlines) {
      chkOutlines.addEventListener('change', (e) => {
        State.showOutlines = e.target.checked;
        document.body.classList.toggle('debug-outlines', e.target.checked);
      });
    }

    const chkMasks = document.getElementById('dbg-chk-masks');
    if (chkMasks) {
      chkMasks.addEventListener('change', (e) => {
        State.showMasks = e.target.checked;
        const maskLayer = document.getElementById('mask-overlay-layer');
        if (maskLayer) maskLayer.style.display = e.target.checked ? 'block' : 'none';
        if (e.target.checked) this._updateMaskOverlay(State.currentState);
      });
    }

    const chkTrees = document.getElementById('dbg-chk-trees');
    if (chkTrees) {
      chkTrees.addEventListener('change', (e) => {
        State.showTreeOutlines = e.target.checked;
        document.body.classList.toggle('debug-tree-outlines', e.target.checked);
      });
    }

    const chkEdit = document.getElementById('dbg-chk-edit');
    if (chkEdit) {
      chkEdit.addEventListener('change', (e) => {
        State.editPoints = e.target.checked;
        this._renderPointHandles(e.target.checked);
      });
    }

    // Export button
    const exportBtn = document.getElementById('dbg-btn-export');
    if (exportBtn) {
      exportBtn.addEventListener('click', () => {
        const dataStr = 'data:text/json;charset=utf-8,' + encodeURIComponent(JSON.stringify(State.hotspotData, null, 2));
        const downloadAnchor = document.createElement('a');
        downloadAnchor.setAttribute('href', dataStr);
        downloadAnchor.setAttribute('download', 'hotspot-data.json');
        document.body.appendChild(downloadAnchor);
        downloadAnchor.click();
        downloadAnchor.remove();
        this._logEvent('[DEBUG] Exported hotspot-data.json download');
      });
    }
  }

  _updateMaskOverlay(stateName) {
    const maskImg = document.getElementById('active-mask-img');
    if (!maskImg) return;

    // Pick primary mask for active state
    const maskMap = {
      S3_STUMP: 'assets/masks/raccoon-dog.png',
      S7_REINDEER: 'assets/masks/reindeer.png',
      S9_ICE_CAVE: 'assets/masks/icicle-1.png',
      S8_HOLD: 'assets/masks/eagle.png',
      S_HOUSE: 'assets/masks/door.png',
      S12_INTERIOR: 'assets/masks/window.png'
    };

    const maskFile = maskMap[stateName];
    if (maskFile) {
      maskImg.src = maskFile;
      maskImg.style.display = 'block';
    } else {
      maskImg.style.display = 'none';
    }
  }

  _renderPointHandles(enable) {
    const group = document.getElementById('debug-points-group');
    if (!group) return;
    group.innerHTML = '';
    if (!enable) return;

    // Render draggable circles for current active hotspot path
    const activeHotspots = document.querySelectorAll(`.hs-group[data-state="${State.currentState}"] .hs-traced`);
    activeHotspots.forEach((pathEl) => {
      const d = pathEl.getAttribute('d');
      if (!d) return;

      const coords = [];
      const parts = d.replace(/[MLZ]/g, '').trim().split(/\s+/);
      for (let i = 0; i < parts.length; i += 2) {
        if (parts[i] && parts[i + 1]) {
          coords.push({ x: parseFloat(parts[i]), y: parseFloat(parts[i + 1]) });
        }
      }

      coords.forEach((pt, ptIdx) => {
        const circle = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
        circle.setAttribute('cx', pt.x);
        circle.setAttribute('cy', pt.y);
        circle.setAttribute('r', '6');
        circle.setAttribute('class', 'debug-point-handle');
        circle.setAttribute('data-point-index', ptIdx);

        // Simple drag listener
        let isDragging = false;
        circle.addEventListener('mousedown', (e) => {
          isDragging = true;
          e.stopPropagation();
        });

        window.addEventListener('mousemove', (e) => {
          if (!isDragging) return;
          const svg = document.getElementById('hotspot-svg');
          const rect = svg.getBoundingClientRect();
          const svgX = ((e.clientX - rect.left) / rect.width) * 1280;
          const svgY = ((e.clientY - rect.top) / rect.height) * 720;

          coords[ptIdx] = { x: Math.round(svgX), y: Math.round(svgY) };
          circle.setAttribute('cx', coords[ptIdx].x);
          circle.setAttribute('cy', coords[ptIdx].y);

          // Rebuild d
          const newD = coords.map((c, i) => `${i === 0 ? 'M' : 'L'} ${c.x} ${c.y}`).join(' ') + ' Z';
          pathEl.setAttribute('d', newD);

          const hsId = pathEl.getAttribute('data-hotspot-id');
          if (State.hotspotData?.hotspots[hsId]) {
            State.hotspotData.hotspots[hsId].d = newD;
          }
        });

        window.addEventListener('mouseup', () => {
          isDragging = false;
        });

        group.appendChild(circle);
      });
    });
  }
}

/* ============================================================================
   BOOTSTRAP
   ============================================================================ */
document.addEventListener('DOMContentLoaded', () => {
  window.State = State;
  const controller = new StoryController();
  window.PortfolioApp = controller;
  window.storyController = controller;
  controller.init().catch((err) => {
    console.error('[BOOTSTRAP] Fatal initialization error:', err);
  });
});
