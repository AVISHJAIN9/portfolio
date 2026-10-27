import asyncio
import os
import sys
import time
import math
from playwright.async_api import async_playwright

OUTPUT_DIR = os.path.abspath("verification/issue_fixes")
os.makedirs(OUTPUT_DIR, exist_ok=True)

PANELS = [
    ("panel-intro", "About Me & Contact"),
    ("panel-achievements", "Achievements"),
    ("panel-projects", "Projects Showcase"),
    ("panel-skills", "Skills & Architecture"),
    ("panel-blog", "Blog"),
    ("panel-upcoming", "Upcoming Events"),
    ("panel-future-projects", "Future Projects"),
    ("icicle-panel", "Icicle Detail Panel")
]

async def open_panel_by_id(page, pid):
    if pid in ["panel-blog", "panel-upcoming", "panel-future-projects"]:
        await page.evaluate(f'window.GlassPortfolio._openSubPanel("{pid}")')
    elif pid == "icicle-panel":
        await page.evaluate('window.PortfolioApp.openIcicleDetail("icicle-1")')
    else:
        await page.evaluate(f'''() => {{
            const el = document.getElementById("{pid}");
            el.style.display = "flex";
            el.classList.add("panel-visible");
            document.body.classList.add("glass-panel-active");
        }}''')
    await page.wait_for_timeout(400)

async def close_panel_by_id(page, pid):
    if pid in ["panel-blog", "panel-upcoming", "panel-future-projects"]:
        await page.evaluate(f'''() => {{
            const sp = document.getElementById("{pid}");
            if (sp) {{
                sp.classList.remove("panel-visible");
                sp.style.display = "none";
            }}
            document.body.classList.remove("glass-panel-active");
        }}''')
    elif pid == "icicle-panel":
        await page.evaluate('window.PortfolioApp._closeIciclePanel()')
    else:
        await page.evaluate(f'''() => {{
            const el = document.getElementById("{pid}");
            if (el) {{
                el.classList.remove("panel-visible");
                el.style.display = "none";
            }}
            document.body.classList.remove("glass-panel-active");
        }}''')
    await page.wait_for_timeout(200)

async def test_panel_sharpness(page, scene_name):
    print(f"\n=======================================================")
    print(f" TESTING ISSUE 1: TEXT SHARPNESS & COMPUTED STYLES ({scene_name})")
    print(f"=======================================================")

    results = {}
    for pid, title in PANELS:
        await open_panel_by_id(page, pid)
        
        # Verify computed styles on panel, ancestors, and text descendants
        eval_res = await page.evaluate(f'''() => {{
            const panel = document.getElementById("{pid}");
            if (!panel) return {{ error: "not found" }};

            const panelCS = window.getComputedStyle(panel);
            const panelFilter = panelCS.filter;
            const panelBackdrop = panelCS.backdropFilter || panelCS.webkitBackdropFilter;

            // Ancestor check
            const ancestorErrors = [];
            let curr = panel.parentElement;
            while (curr && curr !== document.documentElement) {{
                const cs = window.getComputedStyle(curr);
                if (cs.filter && cs.filter !== "none") {{
                    ancestorErrors.push({{ tag: curr.tagName, id: curr.id, cls: curr.className, filter: cs.filter }});
                }}
                curr = curr.parentElement;
            }}

            // Descendant check (all text-bearing elements)
            const textDescendantErrors = [];
            let textElementsCount = 0;
            const all = panel.querySelectorAll("*");
            all.forEach(el => {{
                const hasDirectText = Array.from(el.childNodes).some(n => n.nodeType === 3 && n.textContent.trim().length > 0);
                if (hasDirectText) {{
                    textElementsCount++;
                    const cs = window.getComputedStyle(el);
                    const f = cs.filter;
                    const bf = cs.backdropFilter || cs.webkitBackdropFilter;
                    if (f && f !== "none") {{
                        textDescendantErrors.push({{ tag: el.tagName, id: el.id, cls: el.className, filter: f }});
                    }}
                    if (bf && bf !== "none") {{
                        textDescendantErrors.push({{ tag: el.tagName, id: el.id, cls: el.className, backdropFilter: bf }});
                    }}
                }}
            }});

            // Verify .panel-scrim
            const scrim = document.querySelector(".panel-scrim");
            const scrimCS = scrim ? window.getComputedStyle(scrim) : null;
            const scrimFilter = scrimCS ? scrimCS.filter : "none";
            const scrimBackdrop = scrimCS ? (scrimCS.backdropFilter || scrimCS.webkitBackdropFilter) : "none";

            return {{
                panelFilter,
                panelBackdrop,
                ancestorErrors,
                textElementsCount,
                textDescendantErrorsCount: textDescendantErrors.length,
                textDescendantErrors: textDescendantErrors.slice(0, 5),
                scrimFilter,
                scrimBackdrop
            }};
        }}''')

        # Take screenshot
        shot_path = os.path.join(OUTPUT_DIR, f"{scene_name}_{pid}.png")
        await page.screenshot(path=shot_path)
        print(f"[{scene_name}] Panel '{title}' ({pid}):")
        print(f"    panel filter: {eval_res.get('panelFilter')}")
        print(f"    panel backdrop-filter: {eval_res.get('panelBackdrop')}")
        print(f"    ancestor filter errors: {len(eval_res.get('ancestorErrors', []))}")
        print(f"    text descendants checked: {eval_res.get('textElementsCount')}, errors: {eval_res.get('textDescendantErrorsCount')}")
        print(f"    scrim filter: {eval_res.get('scrimFilter')}, scrim backdrop: {eval_res.get('scrimBackdrop')}")
        print(f"    screenshot: {shot_path}")

        assert eval_res.get("panelFilter") == "none", f"Panel {pid} filter must be 'none' but got {eval_res.get('panelFilter')}"
        assert len(eval_res.get("ancestorErrors", [])) == 0, f"Ancestors of {pid} have active filters!"
        assert eval_res.get("textDescendantErrorsCount") == 0, f"Text elements inside {pid} have filter or backdrop-filter!"
        assert eval_res.get("scrimFilter") == "none", f".panel-scrim filter must be 'none'"
        assert eval_res.get("scrimBackdrop") == "none", f".panel-scrim backdropFilter must be 'none'"

        await close_panel_by_id(page, pid)
        results[pid] = eval_res

    return results

def is_black_frame(pixels_data):
    # pixels_data is a list of RGBA bytes from a downsampled screenshot or evaluate
    # Computes mean luminance: 0.299 R + 0.587 G + 0.114 B
    total_luma = 0
    num_pixels = len(pixels_data) // 4
    if num_pixels == 0:
        return True
    
    non_dark = 0
    for i in range(0, len(pixels_data), 4):
        r = pixels_data[i]
        g = pixels_data[i+1]
        b = pixels_data[i+2]
        luma = 0.299 * r + 0.587 * g + 0.114 * b
        total_luma += luma
        if luma > 15:
            non_dark += 1
            
    mean_luma = total_luma / num_pixels
    ratio_bright = non_dark / num_pixels
    # Fully black frame has mean_luma < 3 and ratio_bright < 0.01
    return mean_luma < 4.0 and ratio_bright < 0.02

async def run_autoadvance_trial(browser, run_idx, width, height):
    print(f"\n--- TRIAL {run_idx}: Resolution {width}x{height} ---")
    page = await browser.new_page(viewport={"width": width, "height": height})
    console_errors = []
    page.on("console", lambda msg: console_errors.append(msg.text) if msg.type == "error" else None)

    t0 = time.time()
    await page.goto("http://localhost:8000/index.html")

    state_history = []
    black_frame_count = 0
    samples_count = 0

    # Total duration for S1 (7.28s) + S2 (6.44s) = 13.72s. With margin + 3s = 16.72s
    # We sample every 200ms
    s3_entered_time = None

    while True:
        elapsed = time.time() - t0
        st = await page.evaluate("window.State ? window.State.currentState : null")
        locked = await page.evaluate("window.State ? window.State.isInputLocked : null")
        
        # Check active video frame luminance via canvas sample in browser
        luma_info = await page.evaluate('''() => {
            const v = (window.PortfolioApp && window.PortfolioApp.player) ? window.PortfolioApp.player.activeVideo : null;
            if (!v || v.videoWidth === 0) return { meanLuma: 50, isBlack: false };
            const c = document.createElement('canvas');
            c.width = 64;
            c.height = 36;
            const ctx = c.getContext('2d');
            ctx.drawImage(v, 0, 0, 64, 36);
            const imgData = ctx.getImageData(0, 0, 64, 36).data;
            let sum = 0;
            let nonBlack = 0;
            const total = imgData.length / 4;
            for (let i = 0; i < imgData.length; i += 4) {
                const lum = 0.299 * imgData[i] + 0.587 * imgData[i+1] + 0.114 * imgData[i+2];
                sum += lum;
                if (lum > 15) nonBlack++;
            }
            const meanLuma = sum / total;
            return { meanLuma, nonBlackRatio: nonBlack / total, isBlack: (meanLuma < 4.0 && (nonBlack / total) < 0.02) };
        }''')

        samples_count += 1
        if luma_info.get("isBlack"):
            black_frame_count += 1
            print(f"  [BLACK FRAME DETECTED] at elapsed={elapsed:.2f}s: {luma_info}")

        if not state_history or state_history[-1]["state"] != st:
            state_history.append({"time": elapsed, "state": st, "locked": locked})
            print(f"  Transition -> {st} at {elapsed:.2f}s (locked={locked})")

        if st == "S3_STUMP" and s3_entered_time is None:
            s3_entered_time = elapsed
            print(f"  --> REACHED S3_STUMP at {s3_entered_time:.2f}s (target <= 17.0s)")

        if s3_entered_time is not None:
            # Once in S3, wait 10 full seconds to verify it stops and doesn't advance
            if (elapsed - s3_entered_time) >= 10.0:
                break

        if elapsed > 28.0:
            # Safety timeout
            break

        await asyncio.sleep(0.2)

    # Final checks for trial
    final_state = await page.evaluate("window.State.currentState")
    final_locked = await page.evaluate("window.State.isInputLocked")
    progress_val = await page.evaluate("document.getElementById('timeline-fill') ? document.getElementById('timeline-fill').style.width : ''")
    ch3_active = await page.evaluate("document.querySelector('.chapter-item[data-jump-state=\"S3_STUMP\"].active') !== null")

    print(f"Trial {run_idx} Results:")
    print(f"  State sequence: {[s['state'] for s in state_history]}")
    print(f"  S3 entered at: {s3_entered_time:.2f}s")
    print(f"  Final state after 10s wait: {final_state} (locked={final_locked})")
    print(f"  Timeline progress width: {progress_val}")
    print(f"  Chapter 3 menu highlight active: {ch3_active}")
    print(f"  Black frame count: {black_frame_count}/{samples_count}")
    print(f"  Console errors: {console_errors}")

    assert s3_entered_time is not None, "Failed to reach S3_STUMP"
    assert s3_entered_time <= 17.0, f"S3 reached too late ({s3_entered_time:.2f}s > 17.0s)"
    assert final_state == "S3_STUMP", f"State did not stay in S3_STUMP! final={final_state}"
    assert final_locked == False, f"Input remained locked in S3_STUMP!"
    assert black_frame_count == 0, f"Detected {black_frame_count} black frames during sequence!"
    assert len(console_errors) == 0, f"Encountered console errors: {console_errors}"

    await page.close()
    return True

async def main():
    async with async_playwright() as p:
        browser = await p.chromium.launch()

        # ==========================================
        # 1. ISSUE 1: Panel Sharpness Tests & Screenshots
        # ==========================================
        page = await browser.new_page(viewport={"width": 1920, "height": 1080})
        await page.goto("http://localhost:8000/index.html")
        await page.wait_for_timeout(1000)

        # Snow scene (S1_FOREST)
        await test_panel_sharpness(page, "snow_scene")

        # Cabin scene (S12_INTERIOR)
        print("\nSwitching to Cabin scene (S12_INTERIOR)...")
        await page.evaluate("() => window.PortfolioApp.enterState('S12_INTERIOR')")
        await page.wait_for_timeout(2000)
        await test_panel_sharpness(page, "cabin_scene")

        # Finale reference scene (S13_FINALE)
        print("\nVerifying Finale Scene cards (S13_FINALE)...")
        await page.evaluate("() => window.PortfolioApp.enterState('S13_FINALE')")
        await page.wait_for_timeout(1500)
        finale_shot = os.path.join(OUTPUT_DIR, "finale_scene_cards.png")
        await page.screenshot(path=finale_shot)
        print(f"Finale reference screenshot saved to: {finale_shot}")

        await page.close()

        # ==========================================
        # 2. ISSUE 2: Auto-Advance S1 -> S2 -> S3 -> Stop (5 non-flaky runs)
        # ==========================================
        print(f"\n=======================================================")
        print(f" TESTING ISSUE 2: AUTO-ADVANCE FLOW (5 TRIALS)")
        print(f"=======================================================")
        
        # 3 runs at 1920x1080, 2 runs at 390x844 (mobile)
        configs = [
            (1920, 1080),
            (1920, 1080),
            (1920, 1080),
            (390, 844),
            (390, 844)
        ]

        for idx, (w, h) in enumerate(configs, 1):
            success = await run_autoadvance_trial(browser, idx, w, h)
            assert success == True, f"Trial {idx} failed!"

        await browser.close()
        print("\nALL VERIFICATIONS PASSED WITH ZERO ERRORS!")

if __name__ == "__main__":
    asyncio.run(main())
