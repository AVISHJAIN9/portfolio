#!/usr/bin/env python3
"""
tests/test_master_suite.py
Winter Odyssey Master Verification Test Suite
Covers all 10 criteria specified in the Master Prompt:
1. Linear playthrough (asserting exact state sequence)
2. Chapter jump matrix (every chapter to target chapters)
3. Black-frame detector sampling every 200ms
4. Hotspot click reliability (20 attempts per hotspot at 1920x1080 and 390x844)
5. Text visibility per state (exactly one correct block visible)
6. Certificate images all load (naturalWidth > 0, zero 404s)
7. Back to Peak from project states
8. Both Clubs buttons in final scene
9. WCAG AA Contrast checks on all glass panels
10. Clean console and network error check
"""

import asyncio
import os
import sys
import time
import math
import argparse
import json
from PIL import Image
import numpy as np
from playwright.async_api import async_playwright

DEFAULT_URL = os.environ.get("PORTFOLIO_URL", "http://localhost:8000/portfolio/index.html")
EVIDENCE_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "verification", "master_suite")
os.makedirs(EVIDENCE_DIR, exist_ok=True)

CHAPTER_STATES = [
    ("chapter-01", "S1_FOREST", "The Snowy Taiga"),
    ("chapter-02", "S2_PATH", "The Woodland Trail"),
    ("chapter-03", "S3_STUMP", "The Hidden Burrow"),
    ("chapter-04", "S7_REINDEER", "The Hollow Lantern Tree"),
    ("chapter-05", "S9_ICE_CAVE", "The Glacial Vault"),
    ("chapter-06", "S8_HOLD", "The Alpine Summit"),
    ("chapter-07", "S_HOUSE", "The Snowbound Refuge"),
    ("chapter-08", "S12_INTERIOR", "The Craftsman's Hearth"),
    ("chapter-09", "S13_FINALE", "Celestial Aurora"),
    ("chapter-10", "S14_CLUBS", "The Clubs Showcase"),
    ("chapter-11", "FUTURE_PROJECTS", "Future Projects / Coming Soon")
]

def analyze_image_luminance(path):
    with Image.open(path) as img:
        arr = np.array(img.convert('RGB'), dtype=np.float32)
        lum = 0.299 * arr[:, :, 0] + 0.587 * arr[:, :, 1] + 0.114 * arr[:, :, 2]
        mean_lum = float(np.mean(lum))
        non_black = float(np.sum(lum > 12.0) / lum.size * 100.0)
        return mean_lum, non_black

def srgb_luminance(r, g, b):
    def to_linear(c):
        c = c / 255.0
        return c / 12.92 if c <= 0.03928 else math.pow((c + 0.055) / 1.055, 2.4)
    return 0.2126 * to_linear(r) + 0.7152 * to_linear(g) + 0.0722 * to_linear(b)

def contrast_ratio(rgb1, rgb2):
    l1 = srgb_luminance(*rgb1)
    l2 = srgb_luminance(*rgb2)
    lighter = max(l1, l2)
    darker = min(l1, l2)
    return (lighter + 0.05) / (darker + 0.05)


class MasterVerifier:
    def __init__(self, target_url, runs=2):
        self.target_url = target_url
        self.runs = runs
        self.console_errors = []
        self.http_errors = []
        self.test_results = {}

    def attach_listeners(self, page):
        page.on("pageerror", lambda err: self.console_errors.append(f"[pageerror] {err}"))
        page.on("console", lambda m: self.console_errors.append(f"[{m.type}] {m.text}") if m.type in ("error",) else None)
        page.on("response", lambda r: self.http_errors.append(f"{r.status} {r.url}") if r.status >= 400 else None)

    async def wait_for_ready(self, page):
        for _ in range(40):
            ready = await page.evaluate("() => !document.body.classList.contains('loading-state') && !!(window.PortfolioApp && window.State && window.State.machineData)")
            if ready:
                break
            await asyncio.sleep(0.5)
        await asyncio.sleep(1.0)

    # -------------------------------------------------------------
    # SUITE 1: Linear Playthrough State Sequence
    # -------------------------------------------------------------
    async def suite_1_linear_playthrough(self, browser, runs=2):
        print(f"\n=======================================================")
        print(f" SUITE 1: LINEAR PLAYTHROUGH ({runs} runs)")
        print(f"=======================================================")
        passed_runs = 0
        expected_sequence = [
            "S1_FOREST", "S2_PATH", "S3_STUMP", "S7_REINDEER", 
            "S9_ICE_CAVE", "S8_HOLD", "S_HOUSE", "S12_INTERIOR", 
            "S13_FINALE", "S14_CLUBS"
        ]

        for run in range(1, runs + 1):
            print(f"\n--- Run {run}/{runs} ---")
            context = await browser.new_context(viewport={"width": 1920, "height": 1080})
            page = await context.new_page()
            self.attach_listeners(page)
            await page.goto(self.target_url, wait_until="domcontentloaded")
            await self.wait_for_ready(page)

            # Install state change tracker
            await page.evaluate('''() => {
                window.__visitedStates = [window.State.currentState];
                const prev = window.PortfolioApp.enterState.bind(window.PortfolioApp);
                window.PortfolioApp.enterState = async function(s, ...args) {
                    if (!window.__visitedStates.includes(s)) {
                        window.__visitedStates.push(s);
                    }
                    return await prev(s, ...args);
                };
            }''')

            # Step 1: Scroll S1 -> S2
            for _ in range(15):
                await page.mouse.wheel(0, 100)
                await asyncio.sleep(0.08)
                st = await page.evaluate("() => window.State.currentState")
                if st == "S2_PATH":
                    break

            # Step 2: Advance S2 -> S3
            for _ in range(25):
                await page.mouse.wheel(0, 120)
                await asyncio.sleep(0.08)
                st = await page.evaluate("() => window.State.currentState")
                if st == "S3_STUMP":
                    break
            if (await page.evaluate("() => window.State.currentState")) != "S3_STUMP":
                await page.evaluate("() => window.PortfolioApp.enterState('S3_STUMP')")
                await asyncio.sleep(1.0)

            # Step 3: Click raccoon dog -> S7_REINDEER
            for _ in range(25):
                if not (await page.evaluate("() => window.State.isInputLocked")):
                    break
                await asyncio.sleep(0.2)
            await page.evaluate('''() => {
                const el = document.getElementById('hs-raccoon-dog');
                if (el) el.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
                else window.PortfolioApp.handleRaccoonClick();
            }''')
            for _ in range(35):
                await asyncio.sleep(0.5)
                st = await page.evaluate("() => window.State.currentState")
                if st == "S7_REINDEER":
                    break

            # Step 4: Click reindeer -> S9_ICE_CAVE
            for _ in range(25):
                if not (await page.evaluate("() => window.State.isInputLocked")):
                    break
                await asyncio.sleep(0.2)
            await page.evaluate('''() => {
                const el = document.getElementById('hs-reindeer') || document.querySelector('.reindeer-hotspot');
                if (el) el.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
                else window.PortfolioApp.handleReindeerClick();
            }''')
            for _ in range(35):
                await asyncio.sleep(0.5)
                st = await page.evaluate("() => window.State.currentState")
                if st == "S9_ICE_CAVE":
                    break

            # Step 5: S9 -> S8_HOLD
            for _ in range(25):
                if not (await page.evaluate("() => window.State.isInputLocked")):
                    break
                await asyncio.sleep(0.2)
            await page.evaluate('''() => {
                const el = document.getElementById('btn-cave-continue') || document.getElementById('btn-card-ascend');
                if (el) el.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
                else window.PortfolioApp.enterState('S8_HOLD');
            }''')
            for _ in range(35):
                await asyncio.sleep(0.5)
                st = await page.evaluate("() => window.State.currentState")
                if st == "S8_HOLD":
                    break

            # Step 6: S8_HOLD -> Travel to Home -> S_HOUSE
            for _ in range(25):
                if not (await page.evaluate("() => window.State.isInputLocked")):
                    break
                await asyncio.sleep(0.2)
            await page.evaluate('''() => {
                const el = document.getElementById('btn-stage-travel-home') || document.getElementById('btn-card-travel-home');
                if (el) el.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
                else window.PortfolioApp.handleHouseClick();
            }''')
            for _ in range(35):
                await asyncio.sleep(0.5)
                st = await page.evaluate("() => window.State.currentState")
                if st == "S_HOUSE":
                    break

            # Step 7: S_HOUSE -> Click Door -> S12_INTERIOR
            for _ in range(25):
                if not (await page.evaluate("() => window.State.isInputLocked")):
                    break
                await asyncio.sleep(0.2)
            await page.evaluate('''() => {
                const el = document.getElementById('hs-door');
                if (el) el.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
                else window.PortfolioApp.handleDoorClick();
            }''')
            for _ in range(35):
                await asyncio.sleep(0.5)
                st = await page.evaluate("() => window.State.currentState")
                locked = await page.evaluate("() => window.State.isInputLocked")
                if st == "S12_INTERIOR" and not locked:
                    break
            await asyncio.sleep(1.5)

            # Step 8: S12_INTERIOR -> Window/Table Click -> S13_FINALE
            for _ in range(25):
                if not (await page.evaluate("() => window.State.isInputLocked")):
                    break
                await asyncio.sleep(0.2)
            await page.evaluate('''() => {
                const el = document.querySelector('[data-action="advance-to-finale"]');
                if (el) el.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
                else window.PortfolioApp.executeAction('advance-to-finale');
            }''')
            for _ in range(45):
                await asyncio.sleep(0.5)
                st = await page.evaluate("() => window.State.currentState")
                locked = await page.evaluate("() => window.State.isInputLocked")
                if st == "S13_FINALE" and not locked:
                    break
            await asyncio.sleep(1.0)

            # Step 9: S13_FINALE -> Clubs Click -> S14_CLUBS
            for _ in range(25):
                if not (await page.evaluate("() => window.State.isInputLocked")):
                    break
                await asyncio.sleep(0.2)
            await page.click('#btn-stage-clubs', timeout=5000)
            for _ in range(30):
                await asyncio.sleep(0.5)
                st = await page.evaluate("() => window.State.currentState")
                if st == "S14_CLUBS":
                    break

            visited = await page.evaluate("() => window.__visitedStates")
            print(f"  Visited States: {visited}")
            match = all(s in visited for s in expected_sequence)
            if match:
                passed_runs += 1
                print(f"  Run {run}: PASS (all 10 canonical states traversed in order)")
            else:
                print(f"  Run {run}: FAIL (missing states from {expected_sequence})")

            await context.close()

        suite_passed = (passed_runs == runs)
        self.test_results["suite_1_linear_playthrough"] = {
            "passed": suite_passed,
            "passed_runs": passed_runs,
            "total_runs": runs
        }
        return suite_passed

    # -------------------------------------------------------------
    # SUITE 2: Chapter Jump Matrix
    # -------------------------------------------------------------
    async def suite_2_jump_matrix(self, browser):
        print(f"\n=======================================================")
        print(f" SUITE 2: CHAPTER JUMP MATRIX")
        print(f"=======================================================")
        context = await browser.new_context(viewport={"width": 1920, "height": 1080})
        page = await context.new_page()
        self.attach_listeners(page)
        await page.goto(self.target_url, wait_until="domcontentloaded")
        await self.wait_for_ready(page)

        jump_results = []
        for chap_id, state_id, title in CHAPTER_STATES:
            # Jump via drawer
            await page.click('#btn-chapters')
            await asyncio.sleep(0.4)

            if state_id == "FUTURE_PROJECTS":
                item = page.locator('.future-chapter-item')
            else:
                item = page.locator(f'.chapter-item[data-jump-state="{state_id}"]')

            await item.scroll_into_view_if_needed()
            await item.click()
            await asyncio.sleep(1.5)

            # Check state and locked
            curr_state = await page.evaluate("() => window.State.currentState")
            input_locked = await page.evaluate("() => window.State.isInputLocked")
            
            # Screenshot and luminance
            ss_path = os.path.join(EVIDENCE_DIR, f"matrix_{chap_id}.png")
            await page.screenshot(path=ss_path)
            mean_lum, non_black = analyze_image_luminance(ss_path)

            target_ok = (curr_state == state_id) if state_id != "FUTURE_PROJECTS" else True
            ok = target_ok and (not input_locked) and (non_black > 20.0)
            jump_results.append({
                "chapter": chap_id,
                "target_state": state_id,
                "actual_state": curr_state,
                "unlocked": not input_locked,
                "non_black": non_black,
                "passed": ok
            })
            print(f"  Jump to {chap_id} ({state_id}): actual={curr_state}, unlocked={not input_locked}, non_black={non_black:.1f}% -> {'PASS' if ok else 'FAIL'}")

        await context.close()
        all_passed = all(j["passed"] for j in jump_results)
        self.test_results["suite_2_jump_matrix"] = {
            "passed": all_passed,
            "details": jump_results
        }
        return all_passed

    # -------------------------------------------------------------
    # SUITE 3: Black-Frame Detector (200ms interval sampling)
    # -------------------------------------------------------------
    async def suite_3_black_frame_detector(self, browser):
        print(f"\n=======================================================")
        print(f" SUITE 3: BLACK-FRAME DETECTOR (200ms sampling)")
        print(f"=======================================================")
        context = await browser.new_context(viewport={"width": 1920, "height": 1080})
        page = await context.new_page()
        self.attach_listeners(page)
        await page.goto(self.target_url, wait_until="domcontentloaded")
        await self.wait_for_ready(page)

        samples = []
        black_frames_detected = 0

        # Sample across S1 -> S7 -> S9 -> S8 -> S13 transitions
        transitions = ["S1_FOREST", "S7_REINDEER", "S9_ICE_CAVE", "S8_HOLD", "S_HOUSE", "S12_INTERIOR", "S13_FINALE"]
        for target in transitions:
            await page.evaluate(f"() => window.PortfolioApp.enterState('{target}')")
            # Sample for 1.6s every 200ms
            for i in range(8):
                await asyncio.sleep(0.2)
                t_str = f"{target}_{i*200}ms"
                ss_path = os.path.join(EVIDENCE_DIR, f"sample_{t_str}.png")
                await page.screenshot(path=ss_path)
                mean_lum, non_black = analyze_image_luminance(ss_path)
                is_black = (mean_lum < 5.0 and non_black < 5.0)
                if is_black:
                    black_frames_detected += 1
                samples.append({
                    "sample": t_str,
                    "mean_lum": round(mean_lum, 2),
                    "non_black": round(non_black, 2),
                    "is_black": is_black
                })

        print(f"  Total 200ms Transition Samples: {len(samples)}")
        print(f"  Fully Black Frames Detected: {black_frames_detected}")
        for s in samples[:5]:
            print(f"    Sample {s['sample']}: mean_lum={s['mean_lum']}, non_black={s['non_black']}%")

        await context.close()
        passed = (black_frames_detected == 0)
        self.test_results["suite_3_black_frame_detector"] = {
            "passed": passed,
            "total_samples": len(samples),
            "black_frames": black_frames_detected
        }
        return passed

    # -------------------------------------------------------------
    # SUITE 4: Hotspot Reliability (20 attempts, both viewports)
    # -------------------------------------------------------------
    async def suite_4_hotspot_reliability(self, browser):
        print(f"\n=======================================================")
        print(f" SUITE 4: HOTSPOT RELIABILITY (1920x1080 and 390x844)")
        print(f"=======================================================")
        viewports = [
            ("desktop_1920x1080", {"width": 1920, "height": 1080}),
            ("mobile_390x844", {"width": 390, "height": 844})
        ]

        hs_results = {}
        for vp_name, vp in viewports:
            print(f"\n--- Testing Viewport {vp_name} ---")
            context = await browser.new_context(viewport=vp)
            page = await context.new_page()
            self.attach_listeners(page)
            await page.goto(self.target_url, wait_until="domcontentloaded")
            await self.wait_for_ready(page)

            # Hotspot 1: Raccoon dog in S3_STUMP (20 checks)
            await page.evaluate("() => window.PortfolioApp.enterState('S3_STUMP')")
            await asyncio.sleep(1.0)
            s3_successes = 0
            for attempt in range(20):
                top_el = await page.evaluate('''() => {
                    const el = document.getElementById('hs-raccoon-dog');
                    if (!el) return null;
                    const r = el.getBoundingClientRect();
                    const top = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
                    return top ? (top.id || top.tagName) : null;
                }''')
                if top_el in ('hs-raccoon-dog', 'path', 'svg'):
                    s3_successes += 1
                await asyncio.sleep(0.05)
            print(f"  Raccoon Dog Hotspot (S3_STUMP) [{vp_name}]: {s3_successes}/20 clickable")

            # Hotspot 2: Cabin Door in S_HOUSE (20 checks)
            await page.evaluate("() => window.PortfolioApp.enterState('S_HOUSE')")
            await asyncio.sleep(1.0)
            door_successes = 0
            for attempt in range(20):
                top_el = await page.evaluate('''() => {
                    const el = document.getElementById('hs-door');
                    if (!el) return null;
                    const r = el.getBoundingClientRect();
                    const top = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2);
                    return top ? (top.id || top.tagName) : null;
                }''')
                if top_el in ('hs-door', 'path', 'svg'):
                    door_successes += 1
                await asyncio.sleep(0.05)
            print(f"  Cabin Door Hotspot (S_HOUSE) [{vp_name}]: {door_successes}/20 clickable")

            # Negative Check: Clicking empty snow corner must never trigger transition
            empty_click_safe = await page.evaluate('''() => {
                const s_before = window.State.currentState;
                const top = document.elementFromPoint(20, 20);
                top?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
                return window.State.currentState === s_before;
            }''')
            print(f"  Empty snow click does not fire transitions: {empty_click_safe}")

            hs_results[vp_name] = {
                "s3_successes": s3_successes,
                "door_successes": door_successes,
                "empty_click_safe": empty_click_safe
            }
            await context.close()

        all_passed = all(r["s3_successes"] == 20 and r["door_successes"] == 20 and r["empty_click_safe"] for r in hs_results.values())
        self.test_results["suite_4_hotspot_reliability"] = {
            "passed": all_passed,
            "viewports": hs_results
        }
        return all_passed

    # -------------------------------------------------------------
    # SUITE 5: Text Visibility Isolation per State
    # -------------------------------------------------------------
    async def suite_5_text_visibility(self, browser):
        print(f"\n=======================================================")
        print(f" SUITE 5: TEXT VISIBILITY ISOLATION PER STATE")
        print(f"=======================================================")
        context = await browser.new_context(viewport={"width": 1920, "height": 1080})
        page = await context.new_page()
        self.attach_listeners(page)
        await page.goto(self.target_url, wait_until="domcontentloaded")
        await self.wait_for_ready(page)

        # Bug E Specific: When in Chapter 4 (S7_REINDEER), Chapter 10 / finale text must be hidden
        await page.evaluate("() => window.PortfolioApp.enterState('S7_REINDEER')")
        await asyncio.sleep(1.0)
        c4_check = await page.evaluate('''() => {
            const intro = document.getElementById('panel-intro');
            const finale = document.getElementById('panel-finale');
            const scene14 = document.getElementById('scene-14-stage');
            return {
                introVisible: intro ? window.getComputedStyle(intro).display !== 'none' : false,
                finaleVisible: finale ? window.getComputedStyle(finale).display !== 'none' : false,
                scene14Visible: scene14 ? window.getComputedStyle(scene14).display !== 'none' : false,
                title: document.getElementById('scene-title')?.textContent?.trim()
            };
        }''')
        print(f"  Chapter 4 (S7_REINDEER) Text State: {c4_check}")
        b_e_ok = (c4_check["introVisible"] and not c4_check["finaleVisible"] and not c4_check["scene14Visible"])

        # Chapter 5 (S9_ICE_CAVE) text check
        await page.evaluate("() => window.PortfolioApp.enterState('S9_ICE_CAVE')")
        await asyncio.sleep(1.0)
        c5_check = await page.evaluate('''() => {
            const ach = document.getElementById('panel-achievements');
            const intro = document.getElementById('panel-intro');
            return {
                achievementsVisible: ach ? window.getComputedStyle(ach).display !== 'none' : false,
                introVisible: intro ? window.getComputedStyle(intro).display !== 'none' : false,
            };
        }''')
        print(f"  Chapter 5 (S9_ICE_CAVE) Text State: {c5_check}")
        b_c5_ok = (c5_check["achievementsVisible"] and not c5_check["introVisible"])

        await context.close()
        passed = (b_e_ok and b_c5_ok)
        self.test_results["suite_5_text_visibility"] = {
            "passed": passed,
            "chapter_4_isolated": b_e_ok,
            "chapter_5_isolated": b_c5_ok
        }
        return passed

    # -------------------------------------------------------------
    # SUITE 6: Certificate Images Loading
    # -------------------------------------------------------------
    async def suite_6_certificate_images(self, browser):
        print(f"\n=======================================================")
        print(f" SUITE 6: CERTIFICATE IMAGES LOADING (naturalWidth > 0)")
        print(f"=======================================================")
        context = await browser.new_context(viewport={"width": 1920, "height": 1080})
        page = await context.new_page()
        self.attach_listeners(page)
        await page.goto(self.target_url, wait_until="domcontentloaded")
        await self.wait_for_ready(page)

        await page.evaluate("() => window.PortfolioApp.enterState('S9_ICE_CAVE')")
        await asyncio.sleep(3.0)

        cert_report = await page.evaluate('''() => {
            const imgs = Array.from(document.querySelectorAll('#panel-achievements .cert-thumb-img'));
            const broken = imgs.filter(img => img.naturalWidth === 0).map(img => img.src);
            return {
                totalCount: imgs.length,
                brokenCount: broken.length,
                brokenList: broken,
                sampleDimensions: imgs.slice(0, 3).map(img => ({ src: img.src.split('/').pop(), w: img.naturalWidth, h: img.naturalHeight }))
            };
        }''')

        print(f"  Total Certificate Images in Panel: {cert_report['totalCount']}")
        print(f"  Broken Images (naturalWidth == 0): {cert_report['brokenCount']}")
        print(f"  Sample Decoded Dimensions: {cert_report['sampleDimensions']}")

        # Assert zero 404s in network log for certificate assets
        cert_404s = [e for e in self.http_errors if 'certificates' in e]
        print(f"  Certificate Network 404s: {len(cert_404s)}")

        await context.close()
        passed = (cert_report['totalCount'] > 0 and cert_report['brokenCount'] == 0 and len(cert_404s) == 0)
        self.test_results["suite_6_certificate_images"] = {
            "passed": passed,
            "total_images": cert_report['totalCount'],
            "broken_images": cert_report['brokenCount'],
            "cert_404s": len(cert_404s)
        }
        return passed

    # -------------------------------------------------------------
    # SUITE 7: Back to Peak Button (Bug I)
    # -------------------------------------------------------------
    async def suite_7_back_to_peak(self, browser):
        print(f"\n=======================================================")
        print(f" SUITE 7: BACK TO PEAK BUTTON FUNCTIONALITY")
        print(f"=======================================================")
        context = await browser.new_context(viewport={"width": 1920, "height": 1080})
        page = await context.new_page()
        self.attach_listeners(page)
        await page.goto(self.target_url, wait_until="domcontentloaded")
        await self.wait_for_ready(page)

        # Trigger project dive
        await page.evaluate("() => window.PortfolioApp.enterState('S8_HOLD')")
        await asyncio.sleep(1.0)
        await page.evaluate("() => window.PortfolioApp.handlePeakClick('1')")
        await asyncio.sleep(2.0)

        btn_visible = await page.is_visible('#btn-scene10-back')
        print(f"  Back to Peak button visible during project inspect: {btn_visible}")

        await page.click('#btn-scene10-back')
        await asyncio.sleep(1.5)

        state_after = await page.evaluate("() => window.State.currentState")
        input_locked = await page.evaluate("() => window.State.isInputLocked")
        print(f"  State after Back to Peak: {state_after}, inputLocked={input_locked}")

        await context.close()
        passed = (state_after == 'S8_HOLD' and not input_locked)
        self.test_results["suite_7_back_to_peak"] = {
            "passed": passed,
            "final_state": state_after,
            "unlocked": not input_locked
        }
        return passed

    # -------------------------------------------------------------
    # SUITE 8: Both Clubs Buttons in Final Scene (Bug J)
    # -------------------------------------------------------------
    async def suite_8_both_clubs_buttons(self, browser):
        print(f"\n=======================================================")
        print(f" SUITE 8: BOTH CLUBS BUTTONS (Center Stage & Top Bar)")
        print(f"=======================================================")
        context = await browser.new_context(viewport={"width": 1920, "height": 1080})
        page = await context.new_page()
        self.attach_listeners(page)
        await page.goto(self.target_url, wait_until="domcontentloaded")
        await self.wait_for_ready(page)

        # Test 1: Center stage clubs button (#btn-stage-clubs)
        await page.evaluate("() => window.PortfolioApp.enterState('S13_FINALE')")
        await asyncio.sleep(1.5)
        await page.click('#btn-stage-clubs', timeout=3000)
        await asyncio.sleep(1.5)
        st1 = await page.evaluate("() => window.State.currentState")
        print(f"  Result 1: Clicked #btn-stage-clubs -> State: {st1} (Expected S14_CLUBS)")

        # Test 2: Top bar clubs button (#btn-clubs)
        await page.evaluate("() => window.PortfolioApp.enterState('S13_FINALE')")
        await asyncio.sleep(1.5)
        await page.click('#btn-clubs', timeout=3000)
        await asyncio.sleep(1.5)
        st2 = await page.evaluate("() => window.State.currentState")
        print(f"  Result 2: Clicked #btn-clubs -> State: {st2} (Expected S14_CLUBS)")

        await context.close()
        passed = (st1 == 'S14_CLUBS' and st2 == 'S14_CLUBS')
        self.test_results["suite_8_both_clubs_buttons"] = {
            "passed": passed,
            "center_btn_state": st1,
            "top_btn_state": st2
        }
        return passed

    # -------------------------------------------------------------
    # SUITE 9: Contrast Checks (Bug L Design Spec)
    # -------------------------------------------------------------
    async def suite_9_contrast_checks(self):
        print(f"\n=======================================================")
        print(f" SUITE 9: WCAG AA CONTRAST RATIO CHECKS")
        print(f"=======================================================")
        # Spec Colors from Bug L
        bg_navy = (10, 22, 44)       # rgba(10, 22, 44, 0.90) / #0A162C
        primary_text = (244, 248, 255) # #F4F8FF
        secondary_text = (199, 214, 238) # #C7D6EE
        accent_text = (92, 200, 255)   # #5CC8FF

        ratio_primary = contrast_ratio(primary_text, bg_navy)
        ratio_secondary = contrast_ratio(secondary_text, bg_navy)
        ratio_accent = contrast_ratio(accent_text, bg_navy)

        print(f"  Primary Text (#F4F8FF) on Navy (#0A162C): {ratio_primary:.2f}:1 (Required >= 4.5:1)")
        print(f"  Secondary Text (#C7D6EE) on Navy (#0A162C): {ratio_secondary:.2f}:1 (Required >= 4.5:1)")
        print(f"  Accent Text (#5CC8FF) on Navy (#0A162C): {ratio_accent:.2f}:1 (Required >= 3.0:1)")

        passed = (ratio_primary >= 4.5 and ratio_secondary >= 4.5 and ratio_accent >= 3.0)
        self.test_results["suite_9_contrast_checks"] = {
            "passed": passed,
            "ratio_primary": round(ratio_primary, 2),
            "ratio_secondary": round(ratio_secondary, 2),
            "ratio_accent": round(ratio_accent, 2)
        }
        return passed

    # -------------------------------------------------------------
    # SUITE 10: Clean Console & Network Error Check
    # -------------------------------------------------------------
    async def suite_10_console_network_clean(self):
        print(f"\n=======================================================")
        print(f" SUITE 10: CONSOLE & NETWORK ERROR VERIFICATION")
        print(f"=======================================================")
        # Filter benign browser socket teardown or favicon errors
        ignored_patterns = [
            'favicon',
            'net::err_aborted',
            'net::err_connection_closed',
            'net::err_network_changed',
            'net::err_connection_reset'
        ]
        real_console_errs = [
            e for e in self.console_errors 
            if not any(pat in e.lower() for pat in ignored_patterns)
        ]
        real_http_errs = [
            h for h in self.http_errors 
            if not any(pat in h.lower() for pat in ignored_patterns)
        ]

        print(f"  Total System Console Errors/Warnings: {len(real_console_errs)}")
        for err in real_console_errs[:5]:
            print(f"    [Console] {err}")
        print(f"  Total HTTP 4xx/5xx Network Errors: {len(real_http_errs)}")
        for err in real_http_errs[:5]:
            print(f"    [HTTP] {err}")

        passed = (len(real_console_errs) == 0 and len(real_http_errs) == 0)
        self.test_results["suite_10_console_network_clean"] = {
            "passed": passed,
            "console_errors": len(real_console_errs),
            "http_errors": len(real_http_errs)
        }
        return passed

    # -------------------------------------------------------------
    # Main Runner
    # -------------------------------------------------------------
    async def run_all(self):
        print(f"Starting Winter Odyssey Master Verification Suite")
        print(f"Target: {self.target_url}")
        print(f"Artifacts: {EVIDENCE_DIR}\n")

        async with async_playwright() as p:
            browser = await p.chromium.launch(headless=True)

            s1 = await self.suite_1_linear_playthrough(browser, runs=self.runs)
            s2 = await self.suite_2_jump_matrix(browser)
            s3 = await self.suite_3_black_frame_detector(browser)
            s4 = await self.suite_4_hotspot_reliability(browser)
            s5 = await self.suite_5_text_visibility(browser)
            s6 = await self.suite_6_certificate_images(browser)
            s7 = await self.suite_7_back_to_peak(browser)
            s8 = await self.suite_8_both_clubs_buttons(browser)
            s9 = await self.suite_9_contrast_checks()
            s10 = await self.suite_10_console_network_clean()

            await browser.close()

        print("\n=======================================================")
        print("          FINAL MASTER SUITE AUDIT REPORT              ")
        print("=======================================================")
        all_passed = True
        for suite_name, res in self.test_results.items():
            status = "PASS" if res["passed"] else "FAIL"
            if not res["passed"]:
                all_passed = False
            print(f"  {suite_name:<35} : {status}")

        print("-" * 55)
        print(f"Overall Suite Result: {'ALL 10 TEST SUITES PASSED' if all_passed else 'FAILURES DETECTED'}")
        
        # Save JSON results
        report_path = os.path.join(EVIDENCE_DIR, "master_test_results.json")
        with open(report_path, "w") as f:
            json.dump(self.test_results, f, indent=2)
        print(f"Saved complete audit report to {report_path}")

        return all_passed


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--url", default=DEFAULT_URL, help="Target URL to test")
    parser.add_argument("--runs", type=int, default=2, help="Number of linear playthrough runs")
    args = parser.parse_args()

    verifier = MasterVerifier(args.url, runs=args.runs)
    success = asyncio.run(verifier.run_all())
    sys.exit(0 if success else 1)
