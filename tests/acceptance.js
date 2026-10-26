/**
 * tests/acceptance.js
 * In-browser automated acceptance test runner for ForestClips Engine v3.
 * Automatically triggered when ?run_acceptance_test=1 is present in the URL.
 */
(function() {
  'use strict';

  var params = new URLSearchParams(window.location.search);
  if (params.get('run_acceptance_test') !== '1') return;

  console.log('[TEST] Acceptance test mode active. Starting test suite...');

  async function runTestSuite() {
    while (document.body.classList.contains('loading-state') || !window.ForestClips) {
      await new Promise(r => setTimeout(r, 50));
    }
    console.log('[TEST] App and ForestClips are ready. Initializing test runner...');
    await new Promise(r => setTimeout(r, 200));

    var results = {
      timestamp: new Date().toISOString(),
      tests: {},
      summary: { total: 5, passed: 0, failed: 0, status: 'RUNNING' }
    };

    var container = document.createElement('div');
    container.id = 'acceptance-test-container';
    container.style.cssText = 'position:fixed;bottom:10px;left:10px;background:rgba(0,0,0,0.92);color:#00ff88;padding:16px;border:1px solid #00ff88;font-family:monospace;font-size:12px;z-index:999999;max-width:800px;max-height:80vh;overflow:auto;pointer-events:all;box-shadow:0 0 20px rgba(0,255,136,0.3);';
    container.innerHTML = '<h3>[ACCEPTANCE TEST RUNNER]</h3><div id="test-status">Running tests...</div><pre id="test-results"></pre>';
    document.body.appendChild(container);

    var fc = window.ForestClips;
    var app = window.PortfolioApp;
    var v = fc ? fc._getMainVideo() : document.getElementById('video-buffer-a');

    console.log('[TEST] Starting TEST A...');
    // =========================================================================
    // TEST (a): IDLE Canvas Cleanliness & Hotspot Isolation
    // =========================================================================
    try {
      if (v) v.pause();
      var canvas = fc ? fc.canvas : document.getElementById('snow-clip-canvas');
      var ctx = canvas ? canvas.getContext('2d') : null;
      var idleNonZero = 0;
      if (ctx) {
        var idata = ctx.getImageData(0, 0, canvas.width, canvas.height);
        for (var i = 3; i < idata.data.length; i += 4) {
          if (idata.data[i] > 0) idleNonZero++;
        }
      }

      // Check legacy overlays disabled
      var legacyContainer = document.getElementById('overlay-container');
      var legacyActive = legacyContainer ? legacyContainer.querySelectorAll('.media-overlay.playing').length : 0;

      // Check dot 1 presence at t=0.50s
      if (v) v.currentTime = 0.50;
      if (fc && fc.hotspots) fc.hotspots.update(0.50);

      var dot1 = document.querySelector('.snow-hotspot-btn[data-clip-id="tree-snow-left"]');
      var dot2 = document.querySelector('.snow-hotspot-btn[data-clip-id="tree-snow-center"]');
      var dot3 = document.querySelector('.snow-hotspot-btn[data-clip-id="tree-snow-right"]');

      var dot1Vis = dot1 && dot1.style.display !== 'none' && dot1.style.opacity > 0;
      var dot2Vis = dot2 && dot2.style.display !== 'none' && dot2.style.opacity > 0;
      var dot3Vis = dot3 && dot3.style.display !== 'none' && dot3.style.opacity > 0;

      var passA = (idleNonZero === 0) && (legacyActive === 0) && dot1Vis && (!dot2Vis) && (!dot3Vis);
      console.log('[TEST] Test A result:', passA, 'idleNonZero:', idleNonZero, 'legacyActive:', legacyActive, 'dot1:', dot1Vis);

      results.tests['test_a_idle_cleanliness'] = {
        name: 'IDLE Canvas Cleanliness & Mutual Exclusion',
        passed: passA,
        metrics: {
          canvasNonZeroPixelsAtIdle: idleNonZero,
          legacyOverlaysActive: legacyActive,
          tree1DotVisibleAt0_50s: dot1Vis,
          tree2DotVisibleAt0_50s: dot2Vis,
          tree3DotVisibleAt0_50s: dot3Vis,
          canvasState: fc ? fc.player.state : 'UNKNOWN'
        }
      };
      if (passA) results.summary.passed++; else results.summary.failed++;
    } catch (errA) {
      console.error('[TEST] Exception in Test A:', errA);
      results.tests['test_a_idle_cleanliness'] = { name: 'IDLE Canvas Cleanliness', passed: false, error: String(errA) };
      results.summary.failed++;
    }

    // =========================================================================
    // TEST (b): Mid-Clip Tree Region Mask (0 pixels outside mask)
    // =========================================================================
    try {
      var seq = fc._sequences['tree-snow-left'];
      await seq.fetchBlobs();
      await seq.decodeBitmaps();

      var cfg = Object.assign({}, FOREST_CLIPS_CONFIG['tree-snow-left'], { _id: 'tree-snow-left' });
      var align = fc._alignmentData && fc._alignmentData.clips ? fc._alignmentData.clips['1.1'] : null;
      var A = (align && align.frames && align.frames[20]) ? align.frames[20].transform : null;

      // Render frame 20 at opacity 1.0
      fc.player.renderFrame(seq, 20, 1.0, cfg, A);

      var bCanvas = fc.canvas;
      var bCtx = bCanvas.getContext('2d');
      var W = bCanvas.width;
      var H = bCanvas.height;
      var bData = bCtx.getImageData(0, 0, W, H).data;

      var dims = SRC_DIMS[cfg._id] || { w: 1920, h: 1080 };
      var trCover = getCoverTransform(dims.w, dims.h, window.innerWidth, window.innerHeight);
      var tr = cfg.treeRegion; // { x: 0.25, y: 0.00, w: 0.60, h: 0.85 }
      var tx = trCover.drawX + tr.x * trCover.drawW;
      var ty = trCover.drawY + tr.y * trCover.drawH;
      var tw = tr.w * trCover.drawW;
      var th = tr.h * trCover.drawH;

      var minX = Math.floor(tx);
      var maxX = Math.ceil(tx + tw);
      var minY = Math.floor(ty);
      var maxY = Math.ceil(ty + th);

      var nonZeroOutside = 0;
      var nonZeroInside = 0;

      for (var y = 0; y < H; y++) {
        for (var x = 0; x < W; x++) {
          var idx = (y * W + x) * 4;
          var alpha = bData[idx + 3];
          var inside = (x >= minX && x <= maxX && y >= minY && y <= maxY);
          if (inside) {
            if (alpha > 0) nonZeroInside++;
          } else {
            if (alpha > 0) nonZeroOutside++;
          }
        }
      }

      // Reset canvas back to idle
      bCtx.clearRect(0, 0, W, H);
      fc.player.stopImmediate();

      var passB = (nonZeroOutside === 0) && (nonZeroInside > 5000);
      results.tests['test_b_midclip_mask_isolation'] = {
        name: 'Mid-Clip Mask Isolation (0 pixels outside tree region)',
        passed: passB,
        metrics: {
          canvasDimensions: W + 'x' + H,
          treeRegionNormalized: tr,
          treeRegionPixels: { minX: minX, maxX: maxX, minY: minY, maxY: maxY },
          nonZeroPixelsInsideMask: nonZeroInside,
          nonZeroPixelsOutsideMask: nonZeroOutside
        }
      };
      if (passB) results.summary.passed++; else results.summary.failed++;
    } catch (errB) {
      results.tests['test_b_midclip_mask_isolation'] = { name: 'Mid-Clip Mask Isolation', passed: false, error: String(errB) };
      results.summary.failed++;
    }

    // =========================================================================
    // TEST (c): Measured Alignment Data & Residuals
    // =========================================================================
    try {
      var aData = fc._alignmentData;
      var cMetrics = {};
      var passC = !!(aData && aData.clips && aData.clips['1.1'] && aData.clips['1.2'] && aData.clips['1.3']);

      if (passC) {
        ['1.1', '1.2', '1.3'].forEach(function(k) {
          var clipObj = aData.clips[k];
          var s = clipObj.summary || clipObj;
          cMetrics['scene_' + k] = {
            clip_id: s.clip_id,
            scene: s.scene,
            label: s.label,
            frame_count: s.frame_count || s.frameCount,
            avg_residual: s.avg_residual,
            max_residual: s.max_residual,
            slope: s.slope,
            t_0: s.t_0 != null ? s.t_0 : s.t0,
            t_last: s.t_last,
            verdict: s.verdict
          };
        });
      }

      results.tests['test_c_measured_alignment_residuals'] = {
        name: 'Measured Alignment & Residuals Table',
        passed: passC,
        metrics: cMetrics
      };
      if (passC) results.summary.passed++; else results.summary.failed++;
    } catch (errC) {
      results.tests['test_c_measured_alignment_residuals'] = { name: 'Measured Alignment', passed: false, error: String(errC) };
      results.summary.failed++;
    }

    // =========================================================================
    // TEST (d): Timeline Takeover & Hand-Back Re-sync
    // =========================================================================
    try {
      var activeVStart = fc._getMainVideo();
      console.log('[TEST-D-START]',
                  'activeV:', activeVStart ? activeVStart.id : 'null',
                  'vA src:', document.getElementById('video-buffer-a')?.src?.slice(-20),
                  'vA opacity:', getComputedStyle(document.getElementById('video-buffer-a')).opacity,
                  'vB src:', document.getElementById('video-buffer-b')?.src?.slice(-20),
                  'vB opacity:', getComputedStyle(document.getElementById('video-buffer-b')).opacity);

      if (activeVStart) {
        activeVStart.currentTime = 0.50;
      }

      // Start clip — DON'T await yet: capture takeoverActive synchronously first
      var clipDonePromise = fc.playClip('tree-snow-left');

      // isTakingOverTimeline is set synchronously inside triggerClip() before async fetch
      // wait one microtask tick for the async IIFE to set flags
      await new Promise(function(r) { setTimeout(r, 50); });
      var takeoverActive = fc.isTakingOverTimeline === true && fc.blockBufferSwap === true;

      // Now wait for the clip to complete naturally (or safety-timeout)
      await new Promise(function(resolve) {
        var waitInt = setInterval(function() {
          if (!fc.isTakingOverTimeline && fc.player.state === 'IDLE') {
            clearInterval(waitInt);
            resolve();
          }
        }, 100);
        // Safety timeout
        setTimeout(function() {
          clearInterval(waitInt);
          if (fc.player && fc.player.state !== 'IDLE') fc.abortClip();
          resolve();
        }, 12000);
      });

      // Wait for clipDonePromise to settle
      await clipDonePromise.catch(function() {});

      // Allow scroll event listener and browser to settle
      await new Promise(function(resolve) { setTimeout(resolve, 400); });

      var activeV = fc._getMainVideo();
      if (activeV && activeV.seeking) {
        await new Promise(function(r) {
          activeV.addEventListener('seeked', r, { once: true });
          setTimeout(r, 3000);
        });
      }
      var dur = (window.State && window.State.clipsData && window.State.clipsData.clips['forest-establishing'])
        ? window.State.clipsData.clips['forest-establishing'].endTime
        : 7.28;
      var maxScroll = (document.documentElement.scrollHeight - window.innerHeight) || 1;
      var scrollProgress = window.scrollY / maxScroll;
      var expectedTime = (scrollProgress / 0.45) * dur;
      var actualTime = activeV ? activeV.currentTime : 0;
      var deltaSeconds = Math.abs(expectedTime - actualTime);
      var passD = takeoverActive && (deltaSeconds < 0.04);

      console.log('[TEST-D-RESULT]', 'activeV:', activeV ? activeV.id : 'null',
                  'currentTime:', actualTime,
                  'scrollY:', window.scrollY,
                  'expectedTime:', expectedTime,
                  'deltaSeconds:', deltaSeconds,
                  'passD:', passD);

      results.tests['test_d_handback_timeline_resync'] = {
        name: 'Timeline Takeover & Hand-Back Re-sync (<0.04s)',
        passed: passD,
        metrics: {
          takeoverEngaged: takeoverActive,
          postHandbackVideoCurrentTime: actualTime,
          postHandbackExpectedTime: expectedTime,
          postHandbackScrollY: window.scrollY,
          deltaSeconds: deltaSeconds,
          deltaFramesAt25fps: deltaSeconds / 0.04
        }
      };
      if (passD) results.summary.passed++; else results.summary.failed++;
    } catch (errD) {
      results.tests['test_d_handback_timeline_resync'] = { name: 'Timeline Takeover Hand-Back', passed: false, error: String(errD) };
      results.summary.failed++;
    }

    // =========================================================================
    // TEST (e): 10 Repeated Triggers & Console Error Check
    // =========================================================================
    try {
      var caughtErrors = [];
      var runs = 0;

      for (var r = 0; r < 10; r++) {
        try {
          var cid = (r % 2 === 0) ? 'tree-snow-left' : 'tree-snow-center';
          // Start clip, wait briefly, then force-abort (fires onComplete, resets engine flags)
          var clipPromise = fc.playClip(cid);
          await new Promise(function(resolve) { setTimeout(resolve, 80); });
          fc.abortClip();
          // Wait for engine to fully settle
          await clipPromise;
          runs++;
        } catch (re) {
          caughtErrors.push(String(re));
        }
      }

      var passE = (runs === 10) && (caughtErrors.length === 0) && (fc.player.state === 'IDLE');
      results.tests['test_e_repeated_triggers_stability'] = {
        name: '10 Repeated Triggers Stability & Teardown',
        passed: passE,
        metrics: {
          completedRuns: runs,
          caughtErrors: caughtErrors,
          finalState: fc.player.state
        }
      };
      if (passE) results.summary.passed++; else results.summary.failed++;
    } catch (errE) {
      results.tests['test_e_repeated_triggers_stability'] = { name: 'Repeated Triggers', passed: false, error: String(errE) };
      results.summary.failed++;
    }

    // =========================================================================
    // FINAL RESULTS & DOM INJECTION
    // =========================================================================
    var allPassed = (results.summary.passed === results.summary.total);
    results.summary.status = allPassed ? 'ALL_PASSED' : 'SOME_FAILED';

    var resultsElem = document.getElementById('test-results');
    if (resultsElem) {
      resultsElem.textContent = JSON.stringify(results, null, 2);
    }
    var statusElem = document.getElementById('test-status');
    if (statusElem) {
      statusElem.textContent = 'Status: ' + results.summary.status + ' (' + results.summary.passed + '/' + results.summary.total + ' passed)';
      statusElem.style.color = allPassed ? '#00ff88' : '#ff4444';
    }

    document.body.setAttribute('data-test-status', results.summary.status);
    window.__ACCEPTANCE_TEST_RESULTS__ = results;

    console.log('ACCEPTANCE_TEST_RESULTS_JSON:' + JSON.stringify(results));
  }

  runTestSuite();
})();
