import asyncio
import os
import sys
from playwright.async_api import async_playwright

LOCAL_URL = "http://localhost:8000/portfolio/index.html"

async def test_reproduction():
    print("=== RUNNING REPRODUCTION TEST ON LOCALHOST ===")
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        context = await browser.new_context(viewport={"width": 1280, "height": 720})
        page = await context.new_page()

        console_errors = []
        page.on("console", lambda msg: console_errors.append(f"[{msg.type}] {msg.text}") if msg.type in ("error", "warning") else None)
        http_errors = []
        page.on("response", lambda r: http_errors.append(f"{r.status} {r.url}") if r.status >= 400 else None)

        print(f"Navigating to {LOCAL_URL}...")
        await page.goto(LOCAL_URL, wait_until="domcontentloaded")
        await page.wait_for_selector("#cinema-stage")
        await asyncio.sleep(2.0)

        # Bug A Check: Scroll Chapter 1 to Chapter 2
        print("\n[CHECK BUG A] Testing Wheel Scrolling S1_FOREST -> S2_PATH...")
        s_initial = await page.evaluate("() => window.State.currentState")
        for _ in range(20):
            await page.mouse.wheel(0, 100)
            await asyncio.sleep(0.05)
        await asyncio.sleep(1.0)
        s_after_scroll = await page.evaluate("() => window.State.currentState")
        print(f"  Bug A Result: Initial State: {s_initial}, After 20 Wheel Events: {s_after_scroll}")

        # Bug F Check: Certificate Images in Chapter 5
        print("\n[CHECK BUG F] Testing Certificate Images in S9_ICE_CAVE...")
        await page.evaluate("() => window.PortfolioApp.enterState('S9_ICE_CAVE')")
        await asyncio.sleep(1.5)
        cert_stats = await page.evaluate('''() => {
            const panel = document.getElementById('panel-achievements');
            if (!panel) return { exists: false };
            const imgs = Array.from(panel.querySelectorAll('.cert-thumb-img'));
            const broken = imgs.filter(img => !img.complete || img.naturalWidth === 0).map(img => img.src);
            return {
                exists: true,
                totalImgs: imgs.length,
                brokenImgs: broken,
                brokenCount: broken.length
            };
        }''')
        print(f"  Bug F Result: Total cert images: {cert_stats.get('totalImgs', 0)}, Broken: {cert_stats.get('brokenCount', 0)}")
        if cert_stats.get('brokenCount', 0) > 0:
            print(f"  Sample broken image URLs: {cert_stats['brokenImgs'][:3]}")

        # Bug I Check: Back to Peak Button
        print("\n[CHECK BUG I] Testing Back to Peak button from S10_LOOP...")
        await page.evaluate("() => window.PortfolioApp.enterState('S8_HOLD')")
        await asyncio.sleep(1.0)
        await page.evaluate("() => window.PortfolioApp.handlePeakClick('1')")
        await asyncio.sleep(2.0)
        back_btn_visible = await page.is_visible("#btn-scene10-back")
        print(f"  Back to peak button visible: {back_btn_visible}")
        if back_btn_visible:
            await page.click("#btn-scene10-back")
            await asyncio.sleep(1.0)
            state_after_back = await page.evaluate("() => window.State.currentState")
            print(f"  State after Back to Peak clicked: {state_after_back}")

        # Bug J Check: Center Clubs Button in Final Scene
        print("\n[CHECK BUG J] Testing Center Clubs button in S13_FINALE...")
        await page.evaluate("() => window.PortfolioApp.enterState('S13_FINALE')")
        await asyncio.sleep(1.5)
        btn_stage_clubs_visible = await page.is_visible("#btn-stage-clubs")
        print(f"  Center stage clubs button visible: {btn_stage_clubs_visible}")
        if btn_stage_clubs_visible:
            try:
                await page.click("#btn-stage-clubs", timeout=2000)
                await asyncio.sleep(1.0)
                state_after_clubs = await page.evaluate("() => window.State.currentState")
                print(f"  State after clicking center clubs button: {state_after_clubs}")
            except Exception as e:
                print(f"  Clicking center clubs failed: {e}")

        # Bug G Check: Travel to Home -> Cabin Door
        print("\n[CHECK BUG G] Testing Travel to Home flow...")
        await page.evaluate("() => window.PortfolioApp.enterState('S8_HOLD')")
        await asyncio.sleep(1.0)
        try:
            await page.evaluate("() => window.PortfolioApp.handleHouseClick()")
            await asyncio.sleep(3.0)
            house_state = await page.evaluate("() => ({ state: window.State.currentState, inputLocked: window.State.isInputLocked })")
            door_visible = await page.is_visible("#hs-door")
            print(f"  House flow: state={house_state['state']}, inputLocked={house_state['inputLocked']}, door_visible={door_visible}")
        except Exception as e:
            print(f"  House flow failed: {e}")

        print("\nConsole errors encountered:", len(console_errors))
        for err in console_errors:
            print(" ", err)
        print("HTTP errors encountered:", len(http_errors))
        for err in http_errors:
            print(" ", err)

        await browser.close()

if __name__ == "__main__":
    asyncio.run(test_reproduction())
