import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from seesea.rss import RssClient

def test_rss_client():
    print("Testing RSS Client...")
    
    try:
        client = RssClient()
        print("✓ RSS Client initialized successfully")
        
        # 测试列出模板
        templates = client.list_templates()
        print(f"✓ Available templates: {templates}")
        
        if templates:
            # 测试从模板添加feeds
            template_name = templates[0]
            print(f"\nTesting template: {template_name}")
            
            count = client.add_from_template(template_name)
            print(f"✓ Added {count} feeds from template '{template_name}'")
        
        print("\n✓ All RSS tests passed!")
        return True
        
    except Exception as e:
        print(f"\n✗ RSS test failed: {e}")
        import traceback
        traceback.print_exc()
        return False

if __name__ == "__main__":
    success = test_rss_client()
    sys.exit(0 if success else 1)
