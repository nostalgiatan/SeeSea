"""
简单的测试脚本，验证核心功能
"""

def test_basic_functionality():
    """测试基本功能（不使用真实模型）"""
    print("=== 测试基本功能 ===\n")
    
    # 测试Rust扩展是否可以导入
    print("1. 测试Rust扩展导入...")
    try:
        from tf_rust import VectorStore
        print("   ✓ tf_rust模块导入成功")
    except ImportError as e:
        print(f"   ✗ 无法导入tf_rust: {e}")
        print("   请运行: maturin develop")
        return False
    
    # 测试创建VectorStore
    print("\n2. 测试创建VectorStore...")
    try:
        store = VectorStore(768)  # 768维向量
        print(f"   ✓ VectorStore创建成功 (dimension=768)")
    except Exception as e:
        print(f"   ✗ 创建失败: {e}")
        return False
    
    # 测试使用预计算向量添加数据
    print("\n3. 测试添加向量...")
    try:
        # 创建模拟向量 (768维)
        mock_vector = [0.1] * 768
        
        store.set_vector(
            "test1",
            mock_vector,
            "测试标题1",
            "https://example.com/1"
        )
        print("   ✓ 向量添加成功")
        
        # 检查数量
        count = store.len()
        print(f"   ✓ 当前文档数量: {count}")
        
        assert count == 1, f"Expected 1 document, got {count}"
    except Exception as e:
        print(f"   ✗ 添加失败: {e}")
        return False
    
    # 测试搜索
    print("\n4. 测试向量搜索...")
    try:
        query_vector = [0.1] * 768
        results = store.search(query_vector, 5)
        print(f"   ✓ 搜索成功，返回 {len(results)} 个结果")
        
        if len(results) > 0:
            result = results[0]
            print(f"   - ID: {result['id']}")
            print(f"   - Score: {result['score']:.4f}")
            print(f"   - Title: {result['title']}")
            print(f"   - URL: {result['url']}")
            print(f"   - Content字段存在: {'content' in result}")
            
            assert 'content' not in result, "Content should NOT be stored!"
            print("   ✓ 确认：content未存储（内存优化成功！）")
    except Exception as e:
        print(f"   ✗ 搜索失败: {e}")
        return False
    
    # 测试获取元数据
    print("\n5. 测试获取元数据...")
    try:
        metadata = store.get_metadata("test1")
        if metadata:
            print(f"   ✓ 元数据获取成功")
            print(f"   - Title: {metadata.get('title', 'N/A')}")
            print(f"   - URL: {metadata.get('url', 'N/A')}")
            print(f"   - Content字段存在: {'content' in metadata}")
            
            assert 'content' not in metadata, "Content should NOT be in metadata!"
            print("   ✓ 确认：元数据中无content（内存优化成功！）")
        else:
            print("   ✗ 元数据获取失败")
            return False
    except Exception as e:
        print(f"   ✗ 获取元数据失败: {e}")
        return False
    
    # 测试删除
    print("\n6. 测试删除文档...")
    try:
        store.rm("test1")
        count = store.len()
        print(f"   ✓ 删除成功，剩余文档: {count}")
        
        assert count == 0, f"Expected 0 documents, got {count}"
    except Exception as e:
        print(f"   ✗ 删除失败: {e}")
        return False
    
    print("\n=== 所有测试通过！ ===")
    return True


def test_callback_mechanism():
    """测试Python回调机制"""
    print("\n\n=== 测试Python回调机制 ===\n")
    
    try:
        from tf_rust import VectorStore
        
        store = VectorStore(768)
        
        # 创建一个简单的回调函数
        call_count = [0]  # 使用列表来跟踪调用次数
        
        def mock_embedder(text: str):
            """模拟嵌入函数"""
            call_count[0] += 1
            print(f"   回调被调用 (第{call_count[0]}次): text长度={len(text)}")
            # 返回一个简单的向量
            return [0.1 + i * 0.001 for i in range(768)]
        
        # 使用回调添加文档
        print("1. 使用回调添加文档...")
        content = "这是一段测试内容，将被向量化后丢弃。" * 10
        
        store.set(
            "callback_test",
            content,
            "回调测试",
            "https://example.com/callback",
            mock_embedder
        )
        
        print(f"   ✓ 文档添加成功")
        print(f"   ✓ 回调函数被调用了 {call_count[0]} 次")
        print(f"   ✓ 内容长度: {len(content)} 字符")
        print(f"   ✓ 内容已被丢弃（未存储）")
        
        # 验证文档存在
        print("\n2. 验证文档存在...")
        metadata = store.get_metadata("callback_test")
        assert metadata is not None, "Document should exist"
        assert 'content' not in metadata, "Content should not be stored"
        print("   ✓ 文档存在")
        print("   ✓ 内容未存储（内存优化成功！）")
        
        # 清理
        store.rm("callback_test")
        
        print("\n=== 回调机制测试通过！ ===")
        return True
        
    except Exception as e:
        print(f"\n✗ 回调机制测试失败: {e}")
        import traceback
        traceback.print_exc()
        return False


if __name__ == "__main__":
    success = test_basic_functionality()
    
    if success:
        success = test_callback_mechanism()
    
    if success:
        print("\n" + "="*50)
        print("🎉 所有测试成功！系统工作正常！")
        print("="*50)
    else:
        print("\n" + "="*50)
        print("❌ 部分测试失败")
        print("="*50)
        exit(1)
