#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
命令行工具：测试LocalLLM处理文本的功能

用法：
python test_local_llm.py <model_path>

示例：
python test_local_llm.py /path/to/model.gguf
"""

import sys
from seesea.Pro.llm import LocalLLM


def main():
    """主函数，处理命令行参数并执行测试流程"""
    # 检查命令行参数
    if len(sys.argv) != 2:
        print("用法: python test_local_llm.py <model_path>")
        print("示例: python test_local_llm.py /path/to/model.gguf")
        sys.exit(1)

    model_path = sys.argv[1]
    print(f"\n正在测试本地LLM，模型路径: {model_path}\n")

    try:
        # 创建LocalLLM实例（自动配置）
        print("=== 1. 创建LocalLLM实例（自动配置） ===")
        llm = LocalLLM(model_path)

        # 打印模型信息和自动配置
        model_info = llm.get_model_info()
        print(f"模型名称: {model_info['model_name']}")
        print(f"系统资源: {model_info['system_resources']}")
        print(f"自动配置: {model_info['current_config']}")

        # 测试文本生成
        print("\n=== 2. 测试文本生成功能 ===")
        prompt = "你好，能介绍一下你自己吗？"
        print(f"提示词: {prompt}")

        generated_text = llm.generate_text(prompt, max_tokens=500, temperature=0.7, top_p=0.95)
        print(f"生成结果: {generated_text}")

        # 测试文本嵌入
        print("\n=== 3. 测试文本嵌入功能 ===")
        text = "这是一个测试文本，用于生成嵌入向量。"
        embedding = llm.generate_embedding(text)
        print(f"文本: {text}")
        print(f"嵌入向量长度: {len(embedding)}")
        print(f"嵌入向量前5个值: {embedding[:5]}")

        # 测试动态配置调整
        print("\n=== 4. 测试动态配置调整 ===")
        print("当前配置 - GPU层数: {}".format(llm.get_current_config()["n_gpu_layers"]))
        print("当前配置 - 线程数: {}".format(llm.get_current_config()["n_threads"]))
        print("当前配置 - 上下文大小: {}".format(llm.get_current_config()["n_ctx"]))

        # 更新配置
        print("\n更新配置: GPU层数=10, 线程数=4, 上下文大小=4096")
        llm.update_config(n_gpu_layers=10, n_threads=4, n_ctx=4096)

        print("更新后配置 - GPU层数: {}".format(llm.get_current_config()["n_gpu_layers"]))
        print("更新后配置 - 线程数: {}".format(llm.get_current_config()["n_threads"]))
        print("更新后配置 - 上下文大小: {}".format(llm.get_current_config()["n_ctx"]))

        # 使用新配置测试文本生成
        print("\n使用新配置测试文本生成:")
        prompt = "请解释一下机器学习的基本概念。"
        print(f"提示词: {prompt}")

        generated_text2 = llm.generate_text(prompt, max_tokens=500, temperature=0.7, top_p=0.95)
        print(f"生成结果: {generated_text2}")

        # 重置配置
        print("\n=== 5. 测试配置重置 ===")
        print("重置配置到初始状态")
        llm.reset_config()

        print("重置后配置 - GPU层数: {}".format(llm.get_current_config()["n_gpu_layers"]))
        print("重置后配置 - 线程数: {}".format(llm.get_current_config()["n_threads"]))
        print("重置后配置 - 上下文大小: {}".format(llm.get_current_config()["n_ctx"]))

        # 测试批量嵌入生成
        print("\n=== 6. 测试批量嵌入生成 ===")
        texts = ["这是第一个测试文本", "这是第二个测试文本", "这是第三个测试文本"]

        embeddings = llm.batch_generate_embedding(texts)
        print(f"输入文本数量: {len(texts)}")
        print(f"生成嵌入数量: {len(embeddings)}")
        for i, (text, embedding) in enumerate(zip(texts, embeddings)):
            print(f"文本 {i+1} 嵌入长度: {len(embedding)}")

        print("\n✅ 所有测试完成！")
        return 0

    except Exception as e:
        print(f"❌ 测试失败: {e}")
        import traceback

        traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())
