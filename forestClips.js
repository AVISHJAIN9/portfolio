/**
 * ============================================================================
 * FOREST SNOW-SHED CLIP ENGINE v3  — forestClips.js
 * ============================================================================
 *
 * Implements:
 *   1. Per-frame alignment mapping from clipAlignment.json (t_k, A_k affine)
 *   2. Timeline Takeover playback model (main video plays 1x, locked via rVFC)
 *   3. Strict mutual-exclusion dot windows (exactly one dot at a time, first 40%)
 *   4. Tight feathered mask around tree region (outside region = 100% transparent)
 *   5. Clean IDLE canvas (zero pixels drawn at IDLE)
 *   6. ?clips=off kill switch
 *   7. Re-sync scroll position after hand-back
 * ============================================================================
 */

'use strict';

/* ============================================================================
   UTILITY FUNCTIONS
   ============================================================================ */
function smoothstep(a, b, t) {
  t = Math.max(0, Math.min(1, (t - a) / (b - a)));
  return t * t * (3 - 2 * t);
}

function lerp(a, b, t) { return a + (b - a) * t; }

function rVFC(video, cb) {
  if (video && video.requestVideoFrameCallback) {
    return video.requestVideoFrameCallback(cb);
  }
  return requestAnimationFrame(() => cb(performance.now(), { mediaTime: video ? video.currentTime : 0 }));
}

function cancelRVFC(video, id) {
  if (video && video.cancelVideoFrameCallback) {
    video.cancelVideoFrameCallback(id);
  } else {
    cancelAnimationFrame(id);
  }
}

/* ============================================================================
   CLIP CONFIG & DERIVED WINDOWS
   Windows: non-overlapping, first ~40% of mapped range, clamped to next start
   ============================================================================ */
var FOREST_CLIPS_CONFIG = {
  'tree-snow-left': {
    sceneKey:     '1.1',
    keyframes:    [{ t: 0.00, x: 0.35, y: 0.35 }],
    treeRegion:   { x: 0.25, y: 0.00, w: 0.60, h: 0.85 }, // normalized {x,y,w,h}
    activeRange:  [0.00, 2.72],
    dotWindow:    [0.00, 1.08], // first 40% of 2.72s, clamped before tree 2
    clipDuration: 3.0,
    speed:        1.0,
    label:        'Left pine (Tree 1)',
    ariaLabel:    'Left pine — click to shed snow'
  },
  'tree-snow-center': {
    sceneKey:     '1.2',
    keyframes:    [{ t: 1.10, x: 0.50, y: 0.35 }],
    treeRegion:   { x: 0.20, y: 0.00, w: 0.60, h: 0.90 },
    activeRange:  [0.08, 2.76],
    dotWindow:    [1.10, 1.76], // clamped start after tree 1 dot ends, first 40%
    clipDuration: 3.25,
    speed:        1.0,
    label:        'Center pine (Tree 2)',
    ariaLabel:    'Center pine — click to shed snow'
  },
  'tree-snow-right': {
    sceneKey:     '1.3',
    keyframes:    [{ t: 6.32, x: 0.86, y: 0.45 }],
    treeRegion:   { x: 0.65, y: 0.05, w: 0.32, h: 0.88 },
    activeRange:  [6.32, 7.24],
    dotWindow:    [6.32, 6.68], // first 40% of [6.32, 7.24]
    clipDuration: 1.0,
    speed:        0.5,
    label:        'Right pine (Tree 3)',
    ariaLabel:    'Right pine — click to shed snow'
  }
};

var SRC_DIMS = {
  'tree-snow-left':   { w: 1920, h: 1080 },
  'tree-snow-center': { w: 1920, h: 1080 },
  'tree-snow-right':  { w: 1280, h: 720  }
};

function getCoverTransform(srcW, srcH, boxW, boxH) {
  var scale = Math.max(boxW / srcW, boxH / srcH);
  var drawW = srcW * scale;
  var drawH = srcH * scale;
  return {
    scale: scale,
    drawX: (boxW - drawW) / 2,
    drawY: (boxH - drawH) / 2,
    drawW: drawW,
    drawH: drawH
  };
}

function sourceNormToViewport(nx, ny, srcW, srcH) {
  var t = getCoverTransform(srcW, srcH, window.innerWidth, window.innerHeight);
  return { x: t.drawX + nx * t.drawW, y: t.drawY + ny * t.drawH };
}

function interpolateKeyframes(keyframes, t) {
  if (!keyframes || keyframes.length === 0) return { x: 0.5, y: 0.5 };
  if (keyframes.length === 1 || t <= keyframes[0].t) {
    return { x: keyframes[0].x, y: keyframes[0].y };
  }
  var last = keyframes[keyframes.length - 1];
  if (t >= last.t) return { x: last.x, y: last.y };

  for (var i = 0; i < keyframes.length - 1; i++) {
    var a = keyframes[i], b = keyframes[i + 1];
    if (t >= a.t && t <= b.t) {
      var u = smoothstep(a.t, b.t, t);
      return { x: lerp(a.x, b.x, u), y: lerp(a.y, b.y, u) };
    }
  }
  return { x: last.x, y: last.y };
}

/* ============================================================================
   FRAME SEQUENCE — Cache blobs, lazily decode ImageBitmaps
   ============================================================================ */
class FrameSequence {
  constructor(urls, fps) {
    this.urls         = urls;
    this.fps          = fps;
    this.blobs        = [];
    this.bitmaps      = [];
    this.blobsReady   = false;
    this.bitmapsReady = false;
    this._fetchP      = null;
    this._decodeP     = null;
    this._aborted     = false;
  }

  get frameCount() { return this.urls.length; }

  async fetchBlobs() {
    if (this.blobsReady) return;
    if (this._fetchP) return this._fetchP;

    this._fetchP = (async () => {
      var results = await Promise.allSettled(
        this.urls.map(url => fetch(url).then(r => {
          if (!r.ok) throw new Error(r.status);
          return r.blob();
        }).catch(() => null))
      );
      if (this._aborted) return;
      this.blobs = results.map(r => (r.status === 'fulfilled' ? r.value : null));
      this.blobsReady = true;
    })();

    return this._fetchP;
  }

  async decodeBitmaps() {
    if (this.bitmapsReady) return;
    if (this._decodeP) return this._decodeP;

    if (!this.blobsReady) await this.fetchBlobs();
    if (this._aborted) return;

    this._decodeP = (async () => {
      var results = await Promise.allSettled(
        this.blobs.map(blob => blob ? createImageBitmap(blob) : Promise.resolve(null))
      );
      if (this._aborted) { results.forEach(r => r.value && r.value.close()); return; }
      this.bitmaps = results.map(r => (r.status === 'fulfilled' ? r.value : null));
      this.bitmapsReady = true;
      this._decodeP = null;
    })();

    return this._decodeP;
  }

  freeBitmaps() {
    this.bitmaps.forEach(b => b && b.close());
    this.bitmaps = [];
    this.bitmapsReady = false;
    this._decodeP = null;
  }

  dispose() {
    this._aborted = true;
    this.freeBitmaps();
    this.blobs = [];
    this.blobsReady = false;
    this._fetchP = null;
  }
}

/* ============================================================================
   CANVAS CLIP PLAYER — Tight Feathered Mask & Timeline Takeover
   ============================================================================ */
var REDUCED_MOTION = window.matchMedia &&
                     window.matchMedia('(prefers-reduced-motion: reduce)').matches;

class CanvasClipPlayer {
  constructor(canvas, getMainVideo) {
    this.canvas        = canvas;
    this.ctx           = canvas.getContext('2d', { alpha: true });
    this.getMainVideo  = getMainVideo;

    this._state        = 'IDLE';
    this._clipConfig   = null;
    this._seq          = null;
    this._alignment    = null; // alignment data for current clip
    this._opacity      = 0;
    this._rVFCId       = null;
    this._fadeStartTs  = null;
    this._fadeMode     = null; // 'in' or 'out'
    this._fadeDuration = 100;
    this._onComplete   = null;

    // Offscreen canvas for tight feathered mask compositing
    this._offCanvas    = document.createElement('canvas');
    this._offCtx       = this._offCanvas.getContext('2d');

    this._boundResize  = () => this._onResize();
    window.addEventListener('resize', this._boundResize, { passive: true });
    this._onResize();
  }

  _onResize() {
    var dpr = window.devicePixelRatio || 1;
    this.canvas.width  = Math.round(window.innerWidth  * dpr);
    this.canvas.height = Math.round(window.innerHeight * dpr);
    this.canvas.style.width  = window.innerWidth  + 'px';
    this.canvas.style.height = window.innerHeight + 'px';
    
    // Clear on resize
    if (this._state === 'IDLE') {
      this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    }
  }

  get state() { return this._state; }

  /**
   * Start timeline takeover playback
   */
  startTakeover(clipId, seq, clipConfig, alignmentData, onComplete) {
    if (this._state !== 'IDLE') return false;

    this._clipConfig = clipConfig;
    this._seq = seq;
    this._alignment = alignmentData;
    this._onComplete = onComplete;
    this._opacity = 0;
    this._fadeMode = 'in';
    this._fadeDuration = REDUCED_MOTION ? 60 : 100;
    this._fadeStartTs = performance.now();
    this._setState('PLAYING');

    var video = this.getMainVideo();
    if (video) {
      if (video.paused) {
        video.play().catch(() => {});
      }
      this._startTracking(video);
    }
    return true;
  }

  _setState(s) {
    this._state = s;
    this.canvas.setAttribute('data-clip-state', s);
  }

  _startTracking(video) {
    var self = this;

    function frameCallback(now, metadata) {
      if (self._state === 'IDLE') return;

      var mediaTime = metadata ? metadata.mediaTime : video.currentTime;
      self._onVideoFrame(mediaTime, now);

      if (self._state !== 'IDLE') {
        self._rVFCId = rVFC(video, frameCallback);
      }
    }

    this._rVFCId = rVFC(video, frameCallback);
  }

  _onVideoFrame(mediaTime, now) {
    var cfg = this._clipConfig;
    var align = this._alignment;
    if (!cfg || !align || !this._seq || !this._seq.bitmapsReady) return;

    // Check if active range has ended
    var activeEnd = cfg.activeRange ? cfg.activeRange[1] : 7.28;
    if (mediaTime >= activeEnd) {
      this._startFadeOut();
    }

    // Handle fade transitions
    if (this._fadeMode === 'in') {
      var elapsed = now - this._fadeStartTs;
      this._opacity = Math.min(1.0, elapsed / this._fadeDuration);
      if (this._opacity >= 1.0) {
        this._opacity = 1.0;
        this._fadeMode = null;
      }
    } else if (this._fadeMode === 'out') {
      var elapsedOut = now - this._fadeStartTs;
      this._opacity = Math.max(0.0, 1.0 - (elapsedOut / this._fadeDuration));
      if (this._opacity <= 0.0) {
        this._opacity = 0.0;
        this._finish();
        return;
      }
    }

    // Look up frame mapping for mediaTime
    var mapping = this._getFrameMapping(mediaTime);
    if (!mapping) {
      this._startFadeOut();
      return;
    }

    // Render tight feathered mask
    this._renderMaskedFrame(mapping.k, mapping.A, this._opacity);
  }

  _getFrameMapping(mediaTime) {
    var align = this._alignment;
    if (!align || !align.frames || !align.frames.length) return null;
    var frames = align.frames;

    // Clamp before first frame
    if (mediaTime <= frames[0].t_k) {
      return { k: frames[0].k, A: frames[0].A_k };
    }
    var last = frames[frames.length - 1];
    if (mediaTime >= last.t_k) {
      return { k: last.k, A: last.A_k };
    }

    // Search interval
    for (var i = 0; i < frames.length - 1; i++) {
      if (mediaTime >= frames[i].t_k && mediaTime <= frames[i + 1].t_k) {
        return { k: frames[i].k, A: frames[i].A_k };
      }
    }
    return { k: last.k, A: last.A_k };
  }

  renderFrame(seq, frameIdx, opacity, cfg, A) {
    this._seq = seq;
    this._clipConfig = cfg;
    this._renderMaskedFrame(frameIdx, A, opacity);
  }

  _renderMaskedFrame(frameIdx, A, opacity) {
    var seq = this._seq;
    var cfg = this._clipConfig;
    if (!seq || !seq.bitmapsReady || opacity <= 0) return;

    var bitmap = seq.bitmaps[Math.max(0, Math.min(frameIdx, seq.frameCount - 1))];
    if (!bitmap) return;

    var dpr  = window.devicePixelRatio || 1;
    var W    = this.canvas.width;
    var H    = this.canvas.height;
    var dims = SRC_DIMS[cfg._id] || { w: 1920, h: 1080 };
    var tr   = getCoverTransform(dims.w, dims.h, window.innerWidth, window.innerHeight);

    // Tree region in screen viewport pixels
    var treeNorm = cfg.treeRegion || { x: 0.25, y: 0.0, w: 0.55, h: 0.85 };
    var tx = tr.drawX + treeNorm.x * tr.drawW;
    var ty = tr.drawY + treeNorm.y * tr.drawH;
    var tw = treeNorm.w * tr.drawW;
    var th = treeNorm.h * tr.drawH;

    // Size offscreen canvas
    var offW = Math.max(1, Math.round(tw));
    var offH = Math.max(1, Math.round(th));
    if (this._offCanvas.width !== offW || this._offCanvas.height !== offH) {
      this._offCanvas.width = offW;
      this._offCanvas.height = offH;
    }

    var offCtx = this._offCtx;
    offCtx.clearRect(0, 0, offW, offH);

    // 1. Draw warped clip frame into offscreen canvas
    offCtx.save();
    offCtx.translate(-tx, -ty);

    // Apply affine transform A from 320x180 downscaled space
    var sx = tr.drawW / 320.0;
    var sy = tr.drawH / 180.0;
    var dx = (A && A[0] && A[0][2] != null) ? A[0][2] * sx : 0;
    var dy = (A && A[1] && A[1][2] != null) ? A[1][2] * sy : 0;

    offCtx.imageSmoothingEnabled = true;
    offCtx.imageSmoothingQuality = 'high';
    offCtx.drawImage(bitmap, tr.drawX + dx, tr.drawY + dy, tr.drawW, tr.drawH);
    offCtx.restore();

    // 2. Apply tight feathered radial mask around tree
    offCtx.save();
    offCtx.globalCompositeOperation = 'destination-in';
    var cx = offW / 2;
    var cy = offH / 2;
    var rx = offW / 2;
    var ry = offH / 2;
    var grad = offCtx.createRadialGradient(cx, cy, Math.min(rx, ry) * 0.45, cx, cy, Math.max(rx, ry));
    grad.addColorStop(0.00, 'rgba(0,0,0,1)');
    grad.addColorStop(0.70, 'rgba(0,0,0,1)');
    grad.addColorStop(0.95, 'rgba(0,0,0,0.15)');
    grad.addColorStop(1.00, 'rgba(0,0,0,0)');
    offCtx.fillStyle = grad;
    offCtx.fillRect(0, 0, offW, offH);
    offCtx.restore();

    // 3. Composite onto stage canvas — zero pixels drawn outside (tx, ty, tw, th)
    this.ctx.clearRect(0, 0, W, H);
    this.ctx.globalAlpha = opacity;
    this.ctx.drawImage(
      this._offCanvas,
      Math.round(tx * dpr),
      Math.round(ty * dpr),
      Math.round(tw * dpr),
      Math.round(th * dpr)
    );
    this.ctx.globalAlpha = 1.0;
  }

  _startFadeOut() {
    if (this._fadeMode === 'out') return;
    this._fadeMode = 'out';
    this._fadeDuration = REDUCED_MOTION ? 60 : 250;
    this._fadeStartTs = performance.now();
  }

  _finish() {
    var video = this.getMainVideo();
    if (video && this._rVFCId) {
      cancelRVFC(video, this._rVFCId);
    }
    this._rVFCId = null;
    this._setState('IDLE');

    // Clean canvas completely
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

    // Free bitmaps
    if (this._seq) {
      this._seq.freeBitmaps();
    }

    var cb = this._onComplete;
    this._onComplete = null;
    if (cb) cb();
  }

  stopImmediate() {
    if (this._state === 'IDLE') return;
    this._startFadeOut();
  }

  /**
   * Force-stop, resolve any pending completion promise, and go to IDLE immediately.
   * Used by rapid-trigger tests and engine cleanup.
   */
  abortImmediate() {
    if (this._state === 'IDLE') return;
    if (this._rVFCId) {
      cancelRVFC(this.getMainVideo ? this.getMainVideo() : null, this._rVFCId);
      this._rVFCId = null;
    }
    this._opacity = 0;
    this._fadeMode = null;
    this._setState('IDLE');
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    if (this._seq) this._seq.freeBitmaps();
    var cb = this._onComplete;
    this._onComplete = null;
    if (cb) cb();
  }
}

/* ============================================================================
   SNOW HOTSPOT LAYER — Single Dot, Non-Overlapping Windows, Mutual Exclusion
   ============================================================================ */
class SnowHotspotLayer {
  constructor(container, clipsConfig, engine) {
    this.container  = container;
    this.config     = clipsConfig;
    this.engine     = engine;
    this.buttons    = {};
    this._visible   = false;
    this._rVFCid    = null;
    this._mainVideo = null;
    this._build();
  }

  _build() {
    var self = this;
    Object.entries(this.config).forEach(function(entry) {
      var clipId = entry[0], cfg = entry[1];
      var btn = document.createElement('button');
      btn.id        = 'snow-hs-' + clipId;
      btn.className = 'snow-hotspot-btn';
      btn.setAttribute('aria-label', cfg.ariaLabel);
      btn.setAttribute('data-clip-id', clipId);
      btn.setAttribute('tabindex', '0');
      btn.title     = cfg.label;
      btn.innerHTML =
        '<span class="snow-hs-dot" aria-hidden="true"></span>' +
        '<span class="snow-hs-tooltip">' + cfg.label + '</span>';
      self.container.appendChild(btn);
      self.buttons[clipId] = btn;
    });

    window.addEventListener('resize', () => this._repositionAll(), { passive: true });
  }

  setMainVideo(video) {
    this._mainVideo = video;
    this._startTrackingLoop();
  }

  _startTrackingLoop() {
    var self = this;
    var video = this._mainVideo;
    if (!video) return;

    function tick(now, meta) {
      var t = meta ? meta.mediaTime : video.currentTime;
      self._repositionAll(t);
      self._updateVisibility(t);
      if (self._visible && self._mainVideo === video) {
        self._rVFCid = rVFC(video, tick);
      }
    }

    this._rVFCid = rVFC(video, tick);
  }

  update(t) {
    if (t == null && this._mainVideo) t = this._mainVideo.currentTime;
    this._repositionAll(t);
    this._updateVisibility(t);
  }

  _repositionAll(t) {
    var self = this;
    Object.entries(this.config).forEach(function(entry) {
      var clipId = entry[0], cfg = entry[1];
      var btn = self.buttons[clipId];
      if (!btn) return;

      var kfs  = cfg.keyframes || [{ t: 0, x: 0.5, y: 0.5 }];
      var pos  = interpolateKeyframes(kfs, t != null ? t : 0);
      var dims = SRC_DIMS[clipId] || { w: 1920, h: 1080 };
      var vp   = sourceNormToViewport(pos.x, pos.y, dims.w, dims.h);
      btn.style.left = vp.x + 'px';
      btn.style.top  = vp.y + 'px';
    });
  }

  _updateVisibility(t) {
    var self = this;
    // Mutual exclusion: if ANY overlay is active, no dot can appear
    if (this.engine.isTakingOverTimeline || this.engine.activeOverlayClipId != null) {
      this.hideAllDots();
      return;
    }

    // Determine which single dot (if any) is within its non-overlapping window
    var activeClipId = null;
    Object.entries(this.config).forEach(function(entry) {
      var clipId = entry[0], cfg = entry[1];
      if (cfg.dotWindow && t >= cfg.dotWindow[0] && t <= cfg.dotWindow[1]) {
        activeClipId = clipId;
      }
    });

    Object.entries(this.buttons).forEach(function(entry) {
      var clipId = entry[0], btn = entry[1];
      var show = (clipId === activeClipId);
      btn.style.opacity       = show ? '1' : '0';
      btn.style.pointerEvents = show ? 'auto' : 'none';
    });
  }

  show() {
    if (this._visible) return;
    this._visible = true;
    this.container.classList.add('snow-hs-layer-visible');
    this._startTrackingLoop();
  }

  hide() {
    if (!this._visible) return;
    this._visible = false;
    this.container.classList.remove('snow-hs-layer-visible');
    this.hideAllDots();
  }

  hideAllDots() {
    Object.values(this.buttons).forEach(b => {
      b.style.opacity = '0';
      b.style.pointerEvents = 'none';
    });
  }
}

/* ============================================================================
   FOREST CLIP ENGINE — Master Controller
   ============================================================================ */
class ForestClipEngine {
  constructor() {
    this.canvas               = null;
    this.hotspots             = null;
    this.player               = null;
    this._sequences           = {};
    this._alignmentData       = null;
    this._activeVideoEl       = null;
    this._visible             = false;
    this._disabled            = false; // ?clips=off kill switch
    this._lastClickedClipId   = null;
    this._lastClickTs         = 0;
    this.activeOverlayClipId  = null;
    this.isTakingOverTimeline = false;
    this.blockBufferSwap      = false;
  }

  _getMainVideo() {
    var v = typeof this._activeVideoEl === 'function'
      ? this._activeVideoEl()
      : this._activeVideoEl;
    if (v && v.isConnected) return v;
    var bA = document.getElementById('video-buffer-a');
    var bB = document.getElementById('video-buffer-b');
    if (bA && getComputedStyle(bA).opacity !== '0' && bA.style.display !== 'none') return bA;
    if (bB && getComputedStyle(bB).opacity !== '0' && bB.style.display !== 'none') return bB;
    return bA || bB || document.querySelector('video');
  }

  async init(mainVideoOrGetter) {
    // 1. Check ?clips=off kill switch
    var params = new URLSearchParams(window.location.search);
    if (params.get('clips') === 'off') {
      console.log('[FOREST] ForestClips completely disabled via ?clips=off');
      this._disabled = true;
      return;
    }

    this._activeVideoEl = mainVideoOrGetter;

    // 2. Setup stage canvas (Empty at IDLE)
    this.canvas = document.createElement('canvas');
    this.canvas.id = 'snow-clip-canvas';
    this.canvas.setAttribute('aria-hidden', 'true');
    this.canvas.setAttribute('data-clip-state', 'IDLE');

    var stage = document.getElementById('cinema-stage');
    if (stage) {
      var overlayContainer = document.getElementById('overlay-container');
      stage.insertBefore(this.canvas, overlayContainer || null);
    } else {
      document.body.appendChild(this.canvas);
    }

    // 3. Hotspot container
    var hsContainer = document.createElement('div');
    hsContainer.id = 'snow-hotspots';
    hsContainer.setAttribute('role', 'group');
    hsContainer.setAttribute('aria-label', 'Snow tree hotspots');
    (stage || document.body).appendChild(hsContainer);

    // 4. Initialize Player & Hotspot layer
    this.player   = new CanvasClipPlayer(this.canvas, () => this._getMainVideo());
    this.hotspots = new SnowHotspotLayer(hsContainer, FOREST_CLIPS_CONFIG, this);

    var video = this._getMainVideo();
    if (video) this.hotspots.setMainVideo(video);

    // 5. Load alignment data & manifest in parallel
    try {
      const [aRes, mRes] = await Promise.all([
        fetch('clipAlignment.json').catch(() => null),
        fetch('clips.manifest.json').catch(() => null)
      ]);
      if (aRes && aRes.ok) this._alignmentData = await aRes.json();
      var manifest = null;
      if (mRes && mRes.ok) manifest = await mRes.json();
    } catch (_) {}

    // 6. Setup FrameSequences
    var FALLBACK = {
      'tree-snow-left':   { dir: 'scene 1.1', count: 72 },
      'tree-snow-center': { dir: 'scene 1.2', count: 78 },
      'tree-snow-right':  { dir: 'scene 1.3', count: 24 }
    };

    var self = this;
    Object.keys(FOREST_CLIPS_CONFIG).forEach(function(clipId) {
      var clipMeta = manifest && manifest.clips && manifest.clips[clipId];
      var urls = [];
      if (clipMeta && clipMeta.frames && clipMeta.frames.length) {
        urls = clipMeta.frames.slice();
      } else {
        var fb = FALLBACK[clipId];
        for (var i = 1; i <= fb.count; i++) {
          urls.push('highquality/' + encodeURIComponent(fb.dir) +
                    '/frame_' + String(i).padStart(3, '0') + '.png');
        }
      }

      var seq = new FrameSequence(urls, (clipMeta && clipMeta.fps) || 24);
      self._sequences[clipId] = seq;
    });

    // 7. Click delegation
    hsContainer.addEventListener('click', function(e) {
      var btn = e.target.closest('.snow-hotspot-btn');
      if (!btn) return;
      var clipId = btn.getAttribute('data-clip-id');
      if (clipId) self._onHotspotClick(clipId);
    });

    console.log('[FOREST] ForestClipEngine v3 initialized successfully');
  }

  activateForScene() {
    if (this._disabled) return;
    this._visible = true;
    if (this.hotspots) this.hotspots.show();
    var video = this._getMainVideo();
    if (video && this.hotspots) this.hotspots.setMainVideo(video);
  }

  deactivateForScene() {
    this._visible = false;
    if (this.hotspots) this.hotspots.hide();
    if (this.player) this.player.stopImmediate();
  }

  setMainVideo(videoEl) {
    this._activeVideoEl = videoEl;
    if (this.hotspots && this._visible) {
      this.hotspots.setMainVideo(videoEl);
    }
  }

  _onHotspotClick(clipId) {
    var now = performance.now();
    // Rapid double-click guard (400ms debounce)
    if (this._lastClickedClipId === clipId && (now - (this._lastClickTs || 0) < 400)) {
      return;
    }
    this._lastClickedClipId = clipId;
    this._lastClickTs = now;

    if (this.isTakingOverTimeline || this.activeOverlayClipId != null) {
      return;
    }
    this.triggerClip(clipId);
  }

  triggerClip(clipId) {
    if (this._disabled || this.isTakingOverTimeline) return Promise.resolve();

    var cfg = FOREST_CLIPS_CONFIG[clipId];
    var seq = this._sequences[clipId];
    if (!cfg || !seq) return Promise.resolve();

    var self = this;

    // Timeline takeover active
    this.isTakingOverTimeline = true;
    this.activeOverlayClipId  = clipId;
    this.blockBufferSwap      = true;

    // Hide all dots immediately
    if (this.hotspots) this.hotspots.hideAllDots();

    // Return a Promise that resolves when the clip completes (or is aborted)
    return (async function() {
      // Decode bitmaps for this clip only
      if (!seq.blobsReady) await seq.fetchBlobs();
      if (!seq.bitmapsReady) await seq.decodeBitmaps();

      var align = self._alignmentData && self._alignmentData.clips
        ? self._alignmentData.clips[cfg.sceneKey]
        : null;

      var cfgWithId = Object.assign({}, cfg, { _id: clipId });

      return new Promise(function(resolve) {
        self.player.startTakeover(clipId, seq, cfgWithId, align, function() {
          var video = self._getMainVideo();
          if (video) {
            video.pause();
            var dur = (window.State && window.State.clipsData && window.State.clipsData.clips['forest-establishing'])
              ? window.State.clipsData.clips['forest-establishing'].endTime
              : 7.28;
            var progress = (video.currentTime / dur) * 0.45;
            var maxScroll = (document.documentElement.scrollHeight - window.innerHeight) || 1;
            window.scrollTo(0, Math.round(progress * maxScroll));
            window.dispatchEvent(new Event('scroll'));
          }

          // Hand control back to scroll scrubbing AFTER scroll position is locked
          self.isTakingOverTimeline = false;
          self.activeOverlayClipId  = null;
          self.blockBufferSwap      = false;

          resolve();
        });
      });
    })();
  }
}

/* ============================================================================
   CSS INJECTION
   ============================================================================ */
function injectForestClipsCSS() {
  if (document.getElementById('forest-clips-css')) return;
  var style = document.createElement('style');
  style.id = 'forest-clips-css';

  style.textContent = [
    '#snow-clip-canvas {',
    '  position:absolute; inset:0;',
    '  width:100%; height:100%;',
    '  pointer-events:none; z-index:5;',
    '}',
    '#snow-clip-canvas[data-clip-state="IDLE"] { display:none; }',
    '#snow-clip-canvas[data-clip-state="PLAYING"] { display:block; }',

    '#snow-hotspots {',
    '  position:absolute; inset:0; pointer-events:none; z-index:20;',
    '}',
    '#snow-hotspots.snow-hs-layer-visible { pointer-events:auto; }',

    '.snow-hotspot-btn {',
    '  position:absolute; width:44px; height:44px;',
    '  transform:translate(-50%,-50%);',
    '  background:transparent; border:none; cursor:pointer; padding:0;',
    '  display:flex; align-items:center; justify-content:center;',
    '  opacity:0; transition:opacity 200ms ease; pointer-events:none;',
    '}',
    '.snow-hotspot-btn:focus-visible {',
    '  outline:2px solid rgba(255,255,255,0.85);',
    '  outline-offset:4px; border-radius:50%;',
    '}',

    '.snow-hs-dot {',
    '  position:absolute; width:14px; height:14px; border-radius:50%;',
    '  background:rgba(255,255,255,0.92);',
    '  box-shadow:0 0 8px rgba(0,0,0,0.45), 0 0 2px rgba(0,0,0,0.3);',
    '  border:1px solid rgba(255,255,255,0.55);',
    '  transition:transform 180ms ease, background 180ms ease;',
    '}',
    '.snow-hotspot-btn:hover .snow-hs-dot { transform:scale(1.3); background:#fff; }',

    '.snow-hs-tooltip {',
    '  position:absolute; bottom:calc(100% + 8px); left:50%;',
    '  transform:translateX(-50%);',
    '  background:rgba(0,0,0,0.75); color:#fff;',
    '  font-size:11px; line-height:1.3; white-space:nowrap;',
    '  padding:3px 9px; border-radius:5px;',
    '  pointer-events:none; opacity:0;',
    '  transition:opacity 130ms ease;',
    '}',
    '.snow-hotspot-btn:hover .snow-hs-tooltip,',
    '.snow-hotspot-btn:focus-visible .snow-hs-tooltip { opacity:1; }'
  ].join('\n');

  document.head.appendChild(style);
}

/* ============================================================================
   PUBLIC API
   ============================================================================ */
var _engine = new ForestClipEngine();

window.ForestClips = {
  init: function(mainVideoOrGetter) {
    injectForestClipsCSS();
    return _engine.init(mainVideoOrGetter).then(function() { return _engine; });
  },
  activate:        function()    { _engine.activateForScene(); },
  deactivate:      function()    { _engine.deactivateForScene(); },
  setMainVideo:    function(el)  { _engine.setMainVideo(el); },
  trigger:         function(id)  { return _engine.triggerClip(id); },
  playClip:        function(id)  { return _engine.triggerClip(id); },
  stopClip:        function()    { if (_engine.player) _engine.player.stopImmediate(); },
  abortClip:       function()    {
    if (_engine.player) _engine.player.abortImmediate();
    // Also reset engine flags in case the onComplete callback wasn't reached yet
    // (abortImmediate fires onComplete itself, which resets these)
  },
  getMainVideo:    function()    { return _engine._getMainVideo(); },
  _getMainVideo:   function()    { return _engine._getMainVideo(); },
  get canvas()               { return _engine.canvas; },
  get player()               { return _engine.player; },
  get hotspots()             { return _engine.hotspots; },
  get _sequences()           { return _engine._sequences; },
  get _alignmentData()       { return _engine._alignmentData; },
  get blockBufferSwap()      { return _engine.blockBufferSwap; },
  get isTakingOverTimeline() { return _engine.isTakingOverTimeline; },
  get activeOverlayClipId()  { return _engine.activeOverlayClipId; },
  get engine()               { return _engine; },
  get config()               { return FOREST_CLIPS_CONFIG; }
};

