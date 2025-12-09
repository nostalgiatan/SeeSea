from playwright.sync_api import sync_playwright
import time

# 使用Playwright同步API
with sync_playwright() as p:
    # 启动浏览器
    browser = p.chromium.launch(headless=True)
    page = browser.new_page()

    # 访问前端页面
    page.goto("http://localhost:5173/")

    # 等待页面加载完成
    page.wait_for_load_state("networkidle")

    # 等待2秒，确保页面完全渲染
    time.sleep(2)

    # 截图并保存
    page.screenshot(path="frontend_screenshot.png", full_page=True)

    # 关闭浏览器
    browser.close()

    print("截图已保存为 frontend_screenshot.png")
