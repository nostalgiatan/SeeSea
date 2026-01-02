import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from seesea_core import get_static_html_dir

def test_static_files():
    print("Testing static HTML files access...")
    
    try:
        # 获取静态HTML目录
        static_dir = get_static_html_dir()
        print(f"✓ Static HTML directory: {static_dir}")
        
        # 检查目录是否存在
        if static_dir.exists():
            print(f"✓ Static HTML directory exists")
            
            # 列出一些文件
            files = list(static_dir.rglob("*"))[:10]
            print(f"✓ Found {len(files)} files (showing first 10):")
            for file in files:
                if file.is_file():
                    print(f"  - {file.relative_to(static_dir)}")
            
            # 检查是否有_app目录
            app_dir = static_dir / "_app"
            if app_dir.exists():
                print(f"✓ Found _app directory")
            
            print("\n✓ All static files tests passed!")
            return True
        else:
            print(f"✗ Static HTML directory does not exist: {static_dir}")
            return False
        
    except Exception as e:
        print(f"\n✗ Static files test failed: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = test_static_files()
    sys.exit(0 if success else 1)
