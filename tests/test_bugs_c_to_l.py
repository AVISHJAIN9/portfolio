import asyncio
import os
import sys
from playwright.async_api import async_playwright

LOCAL_URL = "http://localhost:8000/portfolio/index.html"

async def test_c_to_l():
    print("=== TESTING BUGS C, D, E, H, K, L ===")
    async with async_playwright() as p:
        browser = await p.chromium.launch(headless=True)
        page = await browser.new_page(viewport={"width": 1920, "height": 1080})

        print(f"Navigating to {LOCAL_URL}...")
        await page.goto(LOCAL_URL, wait_until="domcontentloaded")
        await page.wait_for_selector("#cinema-stage")
        await asyncio.sleep(2.0)

        # Check Bug E: Text in Chapter 4 (S7_REINDEER)
        print("\n[CHECK BUG E] Testing text blocks in S7_REINDEER (Chapter 4)...")
        await page.evaluate("() => window.PortfolioApp.enterState('S7_REINDEER')")
        await asyncio.sleep(1.0)
        e_text = await page.evaluate('''() => {
            const cardTitle = document.getElementById('scene-title')?.textContent;
            const cardDesc = document.getElementById('scene-desc')?.textContent;
            const introPanel = document.getElementById('panel-intro');
            const finalePanel = document.getElementById('panel-finale');
            return {
                cardTitle,
                cardDesc,
                introVisible: introPanel ? window.getComputedStyle(introPanel).display !== 'none' : false,
                finaleVisible: finalePanel ? window.getComputedStyle(finalePanel).display !== 'none' : false,
            };
        }''')
        print("  Bug E Text info in S7_REINDEER:", e_text)

        # Check Bug C: Hotspot bounding box and elementFromPoint in S3_STUMP
        print("\n[CHECK BUG C] Testing Raccoon Dog hotspot in S3_STUMP...")
        await page.evaluate("() => window.PortfolioApp.enterState('S3_STUMP')")
        await asyncio.sleep(1.0)
        hs_info = await page.evaluate('''() => {
            const hs = document.getElementById('hs-raccoon-dog');
            if (!hs) return { exists: false };
            const rect = hs.getBoundingClientRect();
            const topEl = document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2);
            return {
                exists: true,
                rect: { x: rect.x, y: rect.y, w: rect.width, h: rect.height },
                topElementTag: topEl?.tagName,
                topElementId: topEl?.id,
                topElementClass: topEl?.className
            };
        }''')
        print("  Bug C Hotspot info in S3_STUMP:", hs_info)

        # Check Bug H: S_HOUSE door click
        print("\n[CHECK BUG H] Testing Door Click in S_HOUSE...")
        await page.evaluate("() => window.PortfolioApp.enterState('S_HOUSE')")
        await asyncio.sleep(1.0)
        door_info = await page.evaluate('''() => {
            const door = document.getElementById('hs-door');
            if (!door) return { exists: false };
            const rect = door.getBoundingClientRect();
            const topEl = document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2);
            return {
                exists: true,
                rect: { x: rect.x, y: rect.y, w: rect.width, h: rect.height },
                topElementTag: topEl?.tagName,
                topElementId: topEl?.id
            };
        }''')
        print("  Bug H Door hotspot info in S_HOUSE:", door_info)

        # Check Bug K: Chapter jump list mapping
        print("\n[CHECK BUG K] Checking chapters drawer list mapping...")
        await page.click('#btn-chapters')
        await asyncio.sleep(0.5)
        chaps = await page.evaluate('''() => {
            const items = Array.from(document.querySelectorAll('.chapter-item'));
            return items.map((it, idx) => ({
                idx: idx + 1,
                state: it.getAttribute('data-jump-state'),
                text: it.querySelector('.chapter-name')?.textContent?.trim()
            }));
        }''')
        print("  Chapters rendered in menu:")
        for c in chaps:
            print(f"    {c['idx']}: state={c['state']}, text={c['text']}")

        await browser.close()

if __name__ == "__main__":
    asyncio.run(test_c_to_l())
