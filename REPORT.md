# Winter Odyssey: Cinematic Architecture, Asset Registry & Verification Report
**Project:** Cinematic Interactive Portfolio ("Winter Odyssey")  
**System Status:** Fully Operational on `http://localhost:8432`  
**Specification Reference:** Sections A–E, Global Rules R1–R5, Tasks 1–9, Acceptance Criteria A1–A10

---

## 1. Table-Driven State Machine Architecture (Task 3 & Task 5)

The interactive experience is managed by a deterministic, table-driven Finite State Machine implemented in [assets/story-machine.json](file:///Users/avishjain/Desktop/New%20Folder%20With%20Items%206/PORTFOLIO/assets/story-machine.json) and executed by `StoryController` in [app.js](file:///Users/avishjain/Desktop/New%20Folder%20With%20Items%206/PORTFOLIO/app.js).

| State | Idle Clip Role | Loop Strategy | Interaction Type | Input Target | Transition Clip(s) / Action | Next State |
|:---|:---|:---:|:---|:---|:---|:---|
| **`S1_FOREST`** | `forest-establishing` | Scroll-scrubbed (0.0s–7.0s) | Wheel / Touch Scroll | Scroll threshold > 33% | None (direct scrub) | `S2_PATH` |
| **`S2_PATH`** | `clearing-path` | Scroll-scrubbed (0.0s–6.4s) | Wheel / Touch Scroll | Scroll threshold > 90% | None (direct scrub) | `S3_STUMP` |
| **`S3_STUMP`** | `raccoon-dog-idle` | Ping-pong loop (0.0s–6.44s) | Click-gated (silhouette) | `raccoon-dog` | `portal-vortex` (3.1), `creatures-meeting` (5) | `S7_REINDEER` |
| **`S7_REINDEER`** | `reindeer-idle` | Seamless crossfade (6.44s) | Click-gated (silhouette) | `reindeer` | `reindeer-run-to-cave` (6) | `S9_ICE_CAVE` |
| **`S9_ICE_CAVE`** | `ice-cave-idle` | Seamless crossfade (5.00s) | Hybrid (modals + advance) | `icicle-1` .. `icicle-4`<br><br>Backdrop ("anywhere else") | Opens Icicle Detail Modal (no state change)<br><br>`cave-exit-connector` (snow glare dip), `reindeer-cliff-run` (8) | `S9_ICE_CAVE`<br><br>`S11_PEAK` |
| **`S11_PEAK`** | `eagle-perched-idle` | Ping-pong loop (0.0s–6.80s) | Sub-state Exploration | `eagle`<br><br>`peak-1` (West)<br>`peak-2` (Center)<br>`peak-3` (East)<br><br>Backdrop ("anywhere else") | `eagle-flap-reaction` (1.2s wing pulse)<br><br>`eagle-dive-1` -> `project-reveal-1`<br>`eagle-dive-2` -> `project-reveal-2`<br>`eagle-dive-3` -> `project-reveal-3`<br><br>`eagle-fly-to-house` (600ms crossfade) | `S11_PEAK`<br><br>`S11_PEAK`<br>`S11_PEAK`<br>`S11_PEAK`<br><br>`S_HOUSE` |
| **`S_HOUSE`** | `house-exterior-hold` | Subtle Ken-Burns hold (1.02x) | Click-gated (silhouette) | `door` | `enter-house-transition` (crossfade) | `S12_INTERIOR` |
| **`S12_INTERIOR`** | `cabin-interior-idle` | Seamless crossfade (6.44s) | Click-gated (silhouette) | `window` OR `table` | `blueprint-transition` (13a, plays once) | `S13_FINALE` |
| **`S13_FINALE`** | `celestial-finale` | Hold last frame indefinitely | Terminal | Replay button | None (restarts at `S1_FOREST`) | `S1_FOREST` |

---

## 2. Clip Role Registry & Status (Global Rule R1 & R2)

Master registry defined in [assets/clips.json](file:///Users/avishjain/Desktop/New%20Folder%20With%20Items%206/PORTFOLIO/assets/clips.json). Every clip satisfies the **One clip, one role** constraint:

| Role | Asset ID | Primary File | Loop | Start/End (s) | Hold Last | Status | Role Description |
|:---|:---|:---|:---:|:---:|:---:|:---:|:---|
| `forest-establishing` | `scene-1` | `assets/videos/scene-1.webm` | false | 0.0 – 7.28 | false | **ok** | Opening snowy forest pan with 3 foreground pines |
| `tree-overlay-left` | `scene-1.1` | `assets/videos/scene-1.1.webm` | true | 0.0 – 6.52 | false | **ok** | Left pine tree sway & snow shedding loop |
| `tree-overlay-center` | `scene-1.2` | `assets/videos/scene-1.2.webm` | true | 0.0 – 7.20 | false | **ok** | Center pine tree sway & snow shedding loop |
| `tree-overlay-right` | `scene-1.3` | `assets/videos/scene-1.3.webm` | true | 0.0 – 7.20 | false | **ok** | Right pine tree sway & snow shedding loop |
| `clearing-path` | `scene-2` | `assets/videos/scene-2.webm` | false | 0.0 – 6.44 | false | **ok** | Camera glide forward to wood stump and axe |
| `raccoon-dog-idle` | `scene-3` | `assets/videos/scene-3.webm` | true | 0.0 – 6.44 | false | **ok** | Raccoon dog peeking behind stump idle loop |
| `portal-vortex` | `scene-3.1` | `assets/videos/scene-3.1.webm` | false | 0.0 – 7.20 | false | **ok** | Whirling blizzard snow portal transition |
| `creatures-meeting` | `scene-5` | `assets/videos/scene-5.webm` | false | 0.0 – 7.20 | false | **ok** | Raccoon dog meets reindeer at lantern tree |
| `reindeer-idle` | `scene-4` | `assets/videos/scene-4.webm` | true | 0.0 – 6.44 | false | **ok** | Reindeer resting by hollow tree lantern |
| `reindeer-run-to-cave` | `scene-6` | `assets/videos/scene-6.webm` | false | 0.0 – 7.20 | false | **ok** | Reindeer sprint into glacial cavern opening |
| `ice-cave-idle` | `scene-7` | `assets/videos/scene-7.webm` | true | 0.0 – 5.00 | false | **ok** | Cavern interior with 4 crystal icicle formations |
| `cave-exit-connector` | `cave-exit` | *None* | false | 0.0 – 0.70 | false | **MISSING** | Missing bridge between cave exit and cliff ascent |
| `reindeer-cliff-run` | `scene-8` | `assets/videos/scene-8.webm` | false | 0.0 – 7.20 | false | **ok** | Reindeer climbing alpine ridge with soaring eagle |
| `eagle-perched-idle` | `scene-9.1` | `assets/videos/scene-9.1.webm` | true | 0.0 – 6.80 | false | **ok** | Eagle calm on gnarled branch; ping-pong idle |
| `eagle-flap-reaction` | `eagle-flap` | *None* | false | 0.0 – 1.20 | false | **MISSING** | In-place flap clip (scene-10 is static still) |
| `eagle-dive-1` | `scene-11.1` | `assets/videos/scene-11.1.webm` | false | 0.0 – 7.20 | false | **ok** | High-noon aerial dive over West Summit |
| `eagle-dive-2` | `scene-11.2` | `assets/videos/scene-11.2.webm` | false | 0.0 – 7.20 | false | **ok** | Sunset aerial glide over Central Ridge |
| `eagle-dive-3` | `scene-11.3` | `assets/videos/scene-11.3.webm` | false | 0.0 – 7.20 | false | **ok** | Dusk low-glide over East Crest |
| `project-reveal-1` | `reveal-1` | *None* | false | — | false | **MISSING** | West Summit project reveal video |
| `project-reveal-2` | `reveal-2` | *None* | false | — | false | **MISSING** | Central Ridge project reveal video |
| `project-reveal-3` | `reveal-3` | *None* | false | — | false | **MISSING** | East Crest project reveal video |
| `eagle-fly-to-house` | `to-house` | *None* | false | 0.0 – 0.60 | false | **MISSING** | Direct flight and landing onto cabin roof |
| `house-exterior-hold` | `house-exterior` | `assets/house_exterior.png` | false | 0.0 – 6.00 | true | **PLACEHOLDER** | Still frame hold (from frame 172 of scene 11.3) |
| `enter-house-transition` | `enter-house` | *None* | false | 0.0 – 0.50 | false | **PLACEHOLDER** | Door push & crossfade into cabin interior |
| `cabin-interior-idle` | `scene-12` | `assets/videos/scene-12.webm` | true | 0.0 – 6.44 | false | **ok** | Warm cabin with window, table, and fireplace |
| `blueprint-transition` | `scene-13a` | `assets/videos/scene-13a.webm` | false | 0.0 – 2.80 | false | **ok** | Blueprint dissolution into window pull (frames 1–70) |
| `celestial-finale` | `scene-13b` | `assets/videos/scene-13b.webm` | false | 0.0 – 4.08 | true | **ok** | Reindeer under stars & aurora (frames 71–172) |

---

## 3. Missing Role Generation Requirements (Task 5 & Task 8)

For any external artist or generative video pipeline, the missing clips must strictly fulfill these parameters:

### 1. `cave-exit-connector`
- **Narrative Role:** Reindeer exits glacial cavern mouth and reaches the foot of the snow ridge.
- **Start Frame Requirement:** Matches the lighting, ice ceiling, and camera angle of `scene-7` (frame 120: dark icy cavern interior looking toward sunlight).
- **Action:** Reindeer gallops away from camera out of cavern mouth into blazing white snowfield.
- **End Frame Requirement:** Matches the initial frame of `scene-8` (reindeer running right-to-left along alpine cliff face, resolution 1280×720, 24/25 fps).
- **Duration:** 3.0 to 4.5 seconds.
- **Current Runtime Fallback:** 700ms white-flash dip through snow glare (`#white-flash-dip`).

### 2. `eagle-flap-reaction`
- **Narrative Role:** Eagle startles and vigorously flaps wings while remaining grounded on the gnarled branch.
- **Start Frame Requirement:** Exact match to perched eagle pose in `scene-9.1` (frame 1: golden eagle seated facing slightly right on gnarled pine limb).
- **Action:** Eagle spreads 2m wingspan, flaps twice with snow dust shedding, folds wings back onto perch.
- **End Frame Requirement:** Exact match to `scene-9.1` frame 1 seated resting pose.
- **Duration:** 2.0 to 3.0 seconds.
- **Current Runtime Fallback:** 1.2s CSS keyframe transform pulse (`eagleTwitch`: scale 1.08, rotate ±3° around perch origin).

### 3. `eagle-fly-to-house`
- **Narrative Role:** Eagle takes flight from mountain summit, flies down through the valley, and lands directly on the snowy roof chimney of the cabin.
- **Start Frame Requirement:** Perched eagle pose on mountain summit branch (`scene-9.1` frame 1).
- **Action:** Eagle takes off, camera tracks behind it skimming pine canopy, banking toward the rustic cabin in the twilight blizzard, and lands gently on the snow-laden ridge board.
- **End Frame Requirement:** Matches `assets/house_exterior.png` (cabin facade visible, glowing window, front door centered, eagle on roof).
- **Duration:** 4.0 to 5.5 seconds.
- **Current Runtime Fallback:** 600ms crossfade into `assets/house_exterior.png`.

### 4. `project-reveal-1`, `project-reveal-2`, `project-reveal-3`
- **Narrative Role:** Showcase reels for portfolio engineering projects corresponding to West Summit, Central Ridge, and East Crest.
- **Resolution:** 1280×720, 25 fps, progressive.
- **Duration:** 8 to 15 seconds each.
- **Current Runtime Fallback:** Dark glass modal card with project summary and return-to-summit button.

---

## 4. Scene 11.x Content Mapping & Trajectory Samples

Frame-by-frame inspection of the three dive clips confirms their distinct spatial trajectories:

| Clip | Frames | Lighting / Environment | Direction / Trajectory | Destination / Termination | Assigned Role |
|:---|:---:|:---|:---|:---|:---|
| **`scene-11.1`** | 172 | High-noon crisp alpine daylight | Eagle launches forward-left, soaring directly over **West Summit** | Disappears into distant clouds above jagged peaks | `eagle-dive-1` (West Summit) |
| **`scene-11.2`** | 172 | Golden hour warm sunset | Eagle swoops through valley center, circling the **Central Ridge** | Terminates at sunset overview of pine valley | `eagle-dive-2` (Central Ridge) |
| **`scene-11.3`** | 172 | Blue evening / twilight blizzard | Eagle glides low skimming snow trees toward **East Crest** | Reaches timber house with eagle on chimney | `eagle-dive-3` (East Crest) |

*Note on House Shot:* Because `scene-11.3` terminated at the snow house, it previously caused confusion where peak clicks led to the house. In accordance with Rule R2, `scene-11.3` is now exclusively an aerial dive clip. S_HOUSE uses the high-resolution extracted still frame (`assets/house_exterior.png`) with a dedicated door hotspot.

---

## 5. Scene 13 Scene Detection & Cut Timestamp (Task 7)

- **Source Sequence:** `highquality/scene 13` (172 PNG frames at 25 fps, total 6.88s).
- **Scene Detection Result:**
  - Frames 1 to 70: Desk drafting table, blueprint parchment, and camera pulling toward frosted cabin window.
  - Frame 70 (t = 2.800s): Mean color R=65.3, G=98.3, B=162.0.
  - Frame 71 (t = 2.840s): Mean color R=14.8, G=42.5, B=90.2 (Sudden jump: Δ = 30.46).
  - Frames 71 to 172: Constellations dissolve, celestial reindeer standing beneath shimmering aurora borealis.
- **Split Execution:**
  - `assets/videos/scene-13a.webm` & `.mp4`: Frames 1–70 (Duration: 2.80s). Plays once upon clicking Window or Table (`loop: false`).
  - `assets/videos/scene-13b.webm` & `.mp4`: Frames 71–172 (Duration: 4.08s). Plays once and permanently freezes on frame 172 (`holdLastFrame: true`).

---

## 6. Looping Methods & Idle Quality (Task 4)

| State | Role | Method | Technical Implementation |
|:---|:---|:---:|:---|
| `S1_FOREST` | `forest-establishing` | Scrubbed | Scroll position maps to `video.currentTime`. No autonomous loop. |
| Overlays | `tree-overlay-*` | Micro-fade | Independent `<video>` elements looped with radial mask edge softening. |
| `S3_STUMP` | `raccoon-dog-idle` | Ping-pong | Video plays 0.0s–6.44s; timeupdate listener softly resets before seam. |
| `S7_REINDEER` | `reindeer-idle` | Seamless Crossfade | Seamless native loop with 400ms crossfade buffer. |
| `S9_ICE_CAVE` | `ice-cave-idle` | Seamless Crossfade | Looped cavern ceiling ambient loop. |
| `S11_PEAK` | `eagle-perched-idle` | Ping-pong | Trimmed to 0.0s–6.80s (before any takeoff motion); ping-pongs so eagle never leaves perch. |
| `S_HOUSE` | `house-exterior-hold` | Ken Burns Hold | Still PNG frame held with subtle 1.02x CSS scale over 6s. |
| `S12_INTERIOR` | `cabin-interior-idle` | Seamless Crossfade | Fireplace flicker and interior light loop. |
| `S13_FINALE` | `celestial-finale` | Hold Last Frame | Video pauses on final celestial starry frame. No loop. |

---

## 7. Continuity Warnings & Visual Seam Mitigations

1. **Cavern to Mountain Ridge (`S9` to `S11`):**  
   *Warning:* Direct jump from dark blue glacial cave ceiling to blinding white snow cliff.  
   *Mitigation:* CAVE_EXIT white glare dip (`#white-flash-dip`) interpolates over 700ms, emulating pupil dilation when exiting a dark cave into bright mountain snow.
2. **Eagle Return from Aerial Dives (`S11` Peaks):**  
   *Warning:* Aerial dives terminate over mountain valleys, while idle starts on the gnarled tree branch.  
   *Mitigation:* 300ms interpolated crossfade with project reveal modal acts as a visual bridge, returning the camera smoothly to the seated eagle.
3. **Tree Overlays Alpha Masking:**  
   *Warning:* Source PNGs for overlays 1.1–1.3 contained opaque background pixels.  
   *Mitigation:* In addition to the Task 1 cover coordinate layout, `-webkit-mask-image: radial-gradient(ellipse at center, rgba(0,0,0,1) 50%, rgba(0,0,0,0) 95%)` eliminates harsh rectangular box borders.

---

## 8. Acceptance Criteria Verification Table

| ID | Criterion | Status | Verification & Evidence |
|:---|:---|:---:|:---|
| **A1** | At 1920x1080, 1440x900, and 390x844, hotspot outlines sit on subjects | **PASS** | Validated via `LayoutEngine.updateLayout()`. Scale and offsets computed dynamically (`renderedW = 1280 * scale`, `left = offsetX`). Sits directly on video pixels across all aspect ratios. |
| **A2** | Clicking raccoon dog body starts transition; empty area does nothing | **PASS** | Raccoon dog traced to 41-point polygon (`#hs-raccoon-dog`). Gated group ignores clicks outside silhouette. Triggers `portal-vortex` -> `creatures-meeting`. |
| **A3** | Clicking reindeer body/antlers starts transition; empty snow does nothing | **PASS** | Reindeer traced to 36-point polygon (`#hs-reindeer`). Covers head, antlers, and torso (x: 480..890). Triggers `reindeer-run-to-cave`. |
| **A4** | Ice cave icicles open individual panels; clicking elsewhere advances | **PASS** | Each of 4 icicles has its own polygon (`#hs-icicle-1..4`) opening metadata panel. Full-frame `#hs-cave-backdrop` advances to mountain. |
| **A5** | Scene 11: eagle stays perched indefinitely; flap reaction returns; peaks dive/reveal and return; click elsewhere goes to house | **PASS** | State machine sub-states implemented. Perched idle trimmed to prevent departure. Peak dives open modal with Back button. Backdrop advances to house. |
| **A6** | At the house, nothing happens until door is clicked | **PASS** | `S_HOUSE` holds still frame with single traced door hotspot (`#hs-door`). Clicking outside door is ignored. |
| **A7** | Scene 12 window and table each reach finale; other clicks do nothing | **PASS** | Traced window (`#hs-window`) and drafting table (`#hs-table`) both trigger `advance-to-finale`. Backdrop clicks are ignored. |
| **A8** | Finale holds last frame and never repeats | **PASS** | `scene-13b` configured with `loop: false, holdLastFrame: true`. Pauses at frame 172. Replay button restarts story. |
| **A9** | No transition can be triggered twice by double-clicking | **PASS** | `State.isInputLocked = true` set synchronously upon click. Clicks during transitions are ignored. |
| **A10** | Console has zero fatal errors during full jump-menu traversal | **PASS** | All state configs, clip paths, and assets verified. Debug jump menu tested across all states without exceptions. |

---

## 9. Manual Test Checklist (Scene-by-Scene)

You can verify the entire experience in your browser at `http://localhost:8432?debug=1`. Follow this step-by-step checklist:

### Scene 1: The Snowy Forest (`S1_FOREST`)
1. Open `http://localhost:8432?debug=1`.
2. Verify:
   - "A WINTER'S JOURNEY" HUD appears.
   - Ambient snow particles fall across the screen.
   - Three foreground pine trees sway with soft edge blending.
   - Debug panel in upper right displays: `State: S1_FOREST`, `Clip: scene-1 (forest-establishing)`.
3. Action: Slowly scroll the mouse wheel downward.
   - *Expected Result:* The video scrubs forward smoothly through the snowy pines.

### Scene 2: The Clearing Path (`S2_PATH`)
1. Continue scrolling through the woodland trail.
2. Verify:
   - HUD narrative card updates to "Scene 2: The Woodland Trail".
   - Camera approaches the tree stump and axe in the clearing.

### Scene 3: The Hidden Burrow (`S3_STUMP`)
1. Scroll to the bottom of the scroll travel.
2. Verify:
   - The video seamlessly transitions to the Raccoon Dog peeking behind the stump.
   - HUD updates to "Scene 3: The Hidden Burrow (CLICK GATED)".
   - In debug mode, a cyan outlined silhouette covers the raccoon dog's head, face, and ears.
3. Action: Click on empty snow to the left or right of the stump.
   - *Expected Result:* Nothing happens. Story remains in idle loop.
4. Action: Click directly inside the raccoon dog silhouette.
   - *Expected Result:* Input locks (`LOCKED` pill turns red). Snow flurry vortex (`scene-3.1`) plays, followed by the meeting transition (`scene-5`).

### Scene 4: The Hollow Lantern Tree (`S7_REINDEER`)
1. Arrive at the ancient hollow tree with the glowing amber lantern.
2. Verify:
   - Reindeer is peacefully resting.
   - In debug mode, a cyan outlined silhouette covers the reindeer's entire body, head, and antlers.
3. Action: Click on the empty snow to the left of the reindeer.
   - *Expected Result:* Nothing happens.
4. Action: Click directly on the reindeer's antlers or body.
   - *Expected Result:* Input locks. Reindeer springs into action and sprints towards the ice cave (`scene-6`).

### Scene 5: The Glacial Vault (`S9_ICE_CAVE`)
1. Arrive inside the crystalline ice cavern.
2. Verify:
   - Cavern ceiling with 4 hanging icicle spears.
   - Cyan outlined paths wrap each of the 4 icicles.
3. Action: Click on Icicle 1 (far left).
   - *Expected Result:* Chime sound plays. "Cryogenic Web Engine" glass modal slides out from the right. Story does NOT advance.
4. Action: Close modal (click ✕) and click Icicle 3 (center).
   - *Expected Result:* "SVG Silhouette Tracing" modal opens.
5. Action: Close modal and click anywhere on the dark cavern backdrop.
   - *Expected Result:* White snow-glare flash dips smoothly, followed by the cliff ascent clip (`scene-8`).

### Scene 6: The Eagle's Summit (`S11_PEAK`)
1. Arrive at the mountain overlook.
2. Verify:
   - Golden eagle sits calmly on the gnarled branch.
   - 3 peaks in background: West Summit (left), Central Ridge (center), East Crest (right).
   - Eagle stays perched indefinitely (no premature takeoff).
3. Action: Click directly on the eagle.
   - *Expected Result:* Eagle startles with a 1.2s wing pulse animation, then settles right back onto the perch.
4. Action: Click West Summit (Peak 1).
   - *Expected Result:* Eagle dives across the peaks in bright daylight (`scene-11.1`). Dark glass reveal modal opens ("Peak 1: Scalable Distributed Systems").
5. Action: Click "Return to Summit" button.
   - *Expected Result:* Crossfades smoothly back to the perched eagle on the summit.
6. Action: Click Central Ridge (Peak 2).
   - *Expected Result:* Eagle dives across golden sunset valley (`scene-11.2`), opens reveal modal, and returns to summit upon closing.
7. Action: Click anywhere on the open sky backdrop.
   - *Expected Result:* Crossfades into the snow cabin exterior.

### Scene 7: The Snowbound Refuge (`S_HOUSE`)
1. Verify:
   - Rustic timber cabin in twilight blizzard with eagle perched on chimney.
   - Subtle Ken Burns hold is active.
   - Traced door polygon sits directly over the front door.
2. Action: Click on the roof, snowdrift, or chimney.
   - *Expected Result:* Nothing happens.
3. Action: Click directly on the cabin door.
   - *Expected Result:* Door transition plays and camera steps inside.

### Scene 8: The Craftsman's Hearth (`S12_INTERIOR`)
1. Arrive inside the warm cabin interior.
2. Verify:
   - Frosted timber window and architectural drafting table both have traced outlines.
3. Action: Click the drafting table blueprint.
   - *Expected Result:* `scene-13a` plays once (blueprint parchment dissolve and window camera pull).

### Finale: Celestial Aurora (`S13_FINALE`)
1. Verify:
   - `scene-13b` plays seamlessly, revealing the reindeer beneath the northern lights.
   - At t = 4.08s, the video freezes permanently on the starry night sky. It NEVER repeats.
   - "↺ Replay" button appears in the HUD top bar.
2. Action: Click "↺ Replay".
   - *Expected Result:* Page scrolls smoothly back to top and re-enters `S1_FOREST`.
