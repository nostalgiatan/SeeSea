#!/usr/bin/env python3
"""
检查 seesea-core 项目的编译错误
"""

import subprocess
import sys
import os

def run_cargo_check():
    """运行 cargo check 并捕获输出"""
    try:
        # 切换到项目目录
        os.chdir("d:\\SeeSea-1\\seesea-core")
        
        # 运行 cargo check
        result = subprocess.run(
            ["cargo", "check"],
            capture_output=True,
            text=True,
            timeout=120  # 2分钟超时
        )
        
        print("=== CARGO CHECK 输出 ===")
        if result.stdout:
            print("STDOUT:")
            print(result.stdout)
        
        if result.stderr:
            print("STDERR:")
            print(result.stderr)
        
        print(f"=== 返回码: {result.returncode} ===")
        
        if result.returncode == 0:
            print("✅ 编译检查通过！")
        else:
            print("❌ 编译检查失败，发现错误")
            
        return result.returncode == 0
        
    except subprocess.TimeoutExpired:
        print("❌ 命令超时（超过2分钟）")
        return False
    except Exception as e:
        print(f"❌ 运行 cargo check 时出错: {e}")
        return False

if __name__ == "__main__":
    print("开始检查 seesea-core 项目编译状态...")
    success = run_cargo_check()
    sys.exit(0 if success else 1)