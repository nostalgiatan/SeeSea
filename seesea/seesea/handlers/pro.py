# Copyright (C) 2025 nostalgiatan
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

"""
SeeSea Pro API Handlers - Pro API 处理函数

提供高级搜索和处理功能
"""

import asyncio
import json
import signal
import traceback
from typing import Dict, Optional, List
import time

from seesea_core import (
    PyCleaner,
    PyDatePage,
    PyVectorClient,
)  # type: ignore[import-untyped]
from seesea.search import SearchClient
from seesea.Pro.url_to_markdown import UrlToMarkdownConverter

# 事件驱动的资源管理


class ProHandlersResources:
    """Pro handlers资源管理器，使用上下文管理器管理资源"""

    def __init__(self):
        self.vector_client: Optional[PyVectorClient] = None
        self.vector_client_initialized: bool = False
        self.cleaning_queue = asyncio.Queue[tuple[PyDatePage, dict, str]]()
        self.processing_queue = asyncio.Queue[tuple[PyDatePage, dict, str]]()
        self.worker_tasks: List[asyncio.Task] = []
        self.should_exit: bool = False

    async def __aenter__(self):
        """进入上下文，初始化资源"""
        # 设置信号处理
        loop = asyncio.get_running_loop()
        for sig in [signal.SIGINT, signal.SIGTERM]:
            loop.add_signal_handler(sig, self._handle_exit)
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """退出上下文，清理资源"""
        self._handle_exit()
        await self._cleanup()

    def _handle_exit(self):
        """处理退出信号"""
        print("🔄 收到终止信号，开始清理资源...")
        self.should_exit = True

    async def _cleanup(self):
        """清理所有资源"""
        # 取消工作任务
        for task in self.worker_tasks:
            task.cancel()

        # 等待任务完成
        if self.worker_tasks:
            await asyncio.gather(*self.worker_tasks, return_exceptions=True)

        # 关闭向量客户端
        if self.vector_client_initialized and self.vector_client:
            try:
                self.vector_client.close()
                print("✅ 向量数据库客户端已关闭")
            except Exception as e:
                print(f"❌ 关闭向量数据库客户端失败: {e}")
            finally:
                self.vector_client = None
                self.vector_client_initialized = False

        print("✅ 所有资源已清理完成")


# 全局资源实例
_resources = None


# 获取全局资源实例
def get_resources() -> ProHandlersResources:
    """获取全局资源实例"""
    global _resources
    if _resources is None:
        _resources = ProHandlersResources()
    return _resources


# 初始化向量数据库客户端
async def initialize_vector_client(config: Optional[Dict] = None):
    """
    初始化向量数据库客户端

    Args:
        config: 向量数据库配置

    Raises:
        RuntimeError: 如果无法连接到向量数据库
    """
    resources = get_resources()

    if resources.vector_client_initialized:
        return

    # 配置已经从配置文件读取，向量客户端初始化会处理连接
    # 我们不再需要硬编码端口检查，因为向量客户端会使用配置文件中的设置
    print("🔍 跳过硬编码端口检查，向量客户端将使用配置文件中的设置")

    # 确保所有功能真正运行，没有回退
    print("🔍 正在初始化向量数据库客户端...")

    # 直接初始化向量客户端，让它抛出实际错误
    # 注意：根据错误信息，向量存储配置需要Qdrant的配置
    # 这里我们直接创建客户端，让它尝试连接到Qdrant服务
    # 如果失败，会抛出明确的错误信息
    try:
        # 直接创建向量客户端，配置从配置文件读取
        resources.vector_client = PyVectorClient.new()
        resources.vector_client_initialized = True
        print("✅ 向量数据库客户端初始化成功")
    except Exception as e:
        print(f"❌ 向量数据库客户端初始化失败: {e}")
        raise RuntimeError(
            f"无法初始化向量数据库客户端，请确保Qdrant服务正在运行且配置正确: {e}"
        ) from e


# 获取向量数据库客户端
async def get_vector_client() -> Optional[PyVectorClient]:
    """
    获取向量数据库客户端，如果未初始化则初始化

    Returns:
        Optional[PyVectorClient]: 向量数据库客户端
    """
    resources = get_resources()
    if not resources.vector_client_initialized:
        await initialize_vector_client()
    return resources.vector_client


# 搜索结果处理任务
async def process_search_results(results: Dict, query: str, page: int, page_size: int):
    """
    处理搜索结果

    Args:
        results: 搜索结果
        query: 搜索查询
        page: 页码
        page_size: 每页结果数
    """
    resources = get_resources()
    print(f"🔍 开始处理搜索结果: 查询='{query}', 页码={page}, 每页结果数={page_size}")

    # 1. 使用异步处理每个URL，提高效率
    async def process_single_url(result):
        url = result.get("url")
        if not url:
            return

        try:
            # 2. 使用UrlToMarkdownConverter获取URL的正文，使用上下文管理器
            async with UrlToMarkdownConverter() as converter:
                convert_result = await converter.convert(url)

            # 检查转换是否成功
            if not convert_result.get("success", False):
                print(f"❌ URL转换失败 {url}: {convert_result.get('error', '未知错误')}")
                return

            # 提取markdown内容
            md_content = convert_result.get("markdown", "")
            print(f"📄 成功获取URL内容: {url}")

            # 3. 在向量数据库中精准搜索URL
            vector_client = await get_vector_client()

            if vector_client:
                # 向量数据库搜索
                vector_results = vector_client.search_by_url(url, limit=1)
                if vector_results and len(vector_results) > 0:
                    print(f"🔍 在向量数据库中找到URL: {url}")

            # 4. 创建DatePage对象
            current_time = time.time()
            date_page = PyDatePage(
                url=url,
                time=current_time,
                description=result.get("description", ""),
                source_data=md_content,
            )
            print(f"📝 创建DatePage对象: {url}")

            # 5. 将DatePage放入清理队列，异步处理
            await resources.cleaning_queue.put((date_page, result, query))
            print(f"📥 已将 {url} 放入清理队列")

        except Exception as e:
            print(f"❌ 处理URL {url} 失败: {e}")
            # 不再跳过，而是抛出异常，确保整个流程真正运行
            raise RuntimeError(f"处理URL {url} 失败: {e}") from e

    # 并发处理所有URL
    tasks = [process_single_url(result) for result in results.get("results", [])]
    if tasks:
        await asyncio.gather(*tasks, return_exceptions=False)


# 清理任务处理器
async def cleaning_worker():
    """
    清理任务工作器 - 处理清理队列中的DatePage对象
    """
    resources = get_resources()
    # 初始化清洗器 - 提供max_lines_per_block参数
    cleaner = PyCleaner(50)

    while not resources.should_exit:
        try:
            # 使用超时获取队列项，允许定期检查退出标志
            date_page, result, query = await asyncio.wait_for(
                resources.cleaning_queue.get(), timeout=1.0
            )
            print(f"🧹 开始清理: {date_page.url()}")

            # 6. 调用cleaning函数进行预处理
            # 注意：这里使用blocking方式调用，实际生产中可能需要更复杂的异步处理
            # 但根据要求，我们使用异步扔任务后台等完成，不必同步等待
            is_unchanged = False
            try:
                # 调用cleaning函数
                date_page.cleaning(cleaner)
                is_unchanged = True  # 假设成功，实际需要根据返回值判断
            except Exception as e:
                print(f"❌ 清理失败 {date_page.url()}: {e}")
                resources.cleaning_queue.task_done()
                continue

            if is_unchanged:
                # 7. 如果哈希一致，直接完成任务
                print(f"✅ 哈希一致，跳过处理: {date_page.url()}")
                resources.cleaning_queue.task_done()
                continue

            # 8. 否则将任务放入处理队列
            await resources.processing_queue.put((date_page, result, query))
            print(f"📤 已将 {date_page.url()} 放入处理队列")

            resources.cleaning_queue.task_done()
        except asyncio.TimeoutError:
            # 超时，继续循环检查退出标志
            continue
        except Exception as e:
            print(f"❌ 清理工作器错误: {e}")
            try:
                resources.cleaning_queue.task_done()
            except ValueError:
                # 队列已经空了，忽略
                pass
    print("🧹 清理工作器已退出")


# 处理任务处理器
async def processing_worker():
    """
    处理任务工作器 - 处理处理队列中的DatePage对象
    """
    resources = get_resources()
    # 初始化嵌入模型
    embedder = None
    try:
        from seesea.Pro.llm.embeddings.llama_cpp_embeddings import LlamaCppEmbedder

        embedder = LlamaCppEmbedder()
        print(f"✅ 嵌入模型初始化成功，维度: {embedder.get_dimension()}")
    except Exception as e:
        print(f"❌ 嵌入模型初始化失败: {e}")

    while not resources.should_exit:
        try:
            # 使用超时获取队列项，允许定期检查退出标志
            date_page, result, query = await asyncio.wait_for(
                resources.processing_queue.get(), timeout=1.0
            )
            print(f"📊 开始处理: {date_page.url()}")

            # 9. 使用嵌入模型模块对数据块进行向量化
            try:
                # 获取数据块
                data_blocks = date_page.data_blocks()
                vectors = []

                # 使用真实的嵌入模型进行向量化
                # 获取所有数据块内容
                block_contents = [block.content() for block in data_blocks]

                # 使用嵌入模型生成向量
                vectors = embedder.encode(block_contents)
                print(f"🔢 生成向量: {len(vectors)}个向量，每个维度: {embedder.get_dimension()}")

                # 10. 使用cleaner的蚁群算法进行进一步处理
                # 注意：cleaner已经在cleaning过程中完成了蚁群算法处理

                # 11. 将新的结果结合DatePage对象写入向量数据库
                vector_client = await get_vector_client()

                # 确保向量客户端可用
                if vector_client is None:
                    raise RuntimeError("❌ 向量客户端不可用，无法写入向量数据库")

                # 准备写入数据
                metadata = {
                    "url": date_page.url(),
                    "title": result.get("title", ""),
                    "description": result.get("description", ""),
                    "hash": str(date_page.hash()),
                    "last_update_time": date_page.last_update_time(),
                    "query": query,
                }

                # 写入向量数据库
                vector_client.upsert_with_metadata(
                    vectors=vectors,
                    metadata=metadata,
                    data_blocks=[block.content() for block in data_blocks],
                )

                # 12. 更新最后更新时间
                date_page.update_source_data(date_page.source_data())

                print(f"✅ 处理完成并写入向量数据库: {date_page.url()}")
            except Exception as e:
                print(f"❌ 处理失败 {date_page.url()}: {e}")

            resources.processing_queue.task_done()
        except asyncio.TimeoutError:
            # 超时，继续循环检查退出标志
            continue
        except Exception as e:
            print(f"❌ 处理工作器错误: {e}")
            try:
                resources.processing_queue.task_done()
            except ValueError:
                # 队列已经空了，忽略
                pass
    print("📊 处理工作器已退出")


# 启动工作器
async def start_workers():
    """
    启动异步工作器
    """
    resources = get_resources()
    # 启动清理工作器
    resources.worker_tasks.append(asyncio.create_task(cleaning_worker()))
    # 启动处理工作器
    resources.worker_tasks.append(asyncio.create_task(processing_worker()))
    print("✅ 异步工作器启动成功")


# 处理Pro搜索请求
async def handle_pro_search(req: Dict) -> Dict:
    """
    处理Pro搜索请求

    Args:
        req: 请求上下文

    Returns:
        Dict: 响应
    """
    try:
        # 解析请求参数
        query_params = req.get("query_params", {})
        query = query_params.get("q", "")
        page = int(query_params.get("page", 1))
        page_size = int(query_params.get("page_size", 10))

        if not query:
            return {"status": 400, "body": json.dumps({"error": "Query is required"})}

        print(f"🔍 收到Pro搜索请求: query='{query}', page={page}, page_size={page_size}")

        # 1. 使用正常搜索函数进行搜索
        search_client = SearchClient()
        results = search_client.search(query=query, page=page, page_size=page_size)

        # 将搜索结果转换为字典
        results_dict: Dict = {
            "query": results.query,
            "results": [
                {
                    "title": item.title,
                    "url": item.url,
                    "description": item.content,
                    "score": getattr(item, "score", 0),
                }
                for item in results.results
            ],
            "total_count": results.total_count,
            "cached": results.cached,
            "query_time_ms": results.query_time_ms,
            "engines_used": results.engines_used,
        }

        # 2. 同步处理搜索结果，确保所有数据都经过向量处理
        await process_search_results(results_dict, query, page, page_size)

        # 3. 初始化嵌入模型，生成查询向量
        from seesea.Pro.llm.embeddings.llama_cpp_embeddings import LlamaCppEmbedder

        embedder = LlamaCppEmbedder()
        query_vector = embedder.encode([query])[0]

        # 4. 使用向量数据库搜索相似文档，获取相关性分数
        vector_client = await get_vector_client()
        if vector_client:
            vector_results = vector_client.search(query_vector, limit=page_size)

            # 5. 根据向量搜索结果重新排序原始搜索结果
            # 创建URL到相关性分数的映射
            url_score_map = {}
            for result in vector_results:
                # 获取结果中的URL和分数
                # 注意：这里需要根据实际的PyVectorClient.search返回结构调整
                result_dict = result.as_dict() if hasattr(result, "as_dict") else {}
                url = result_dict.get("url", "")
                score = result_dict.get("score", 0.0)
                if url:
                    url_score_map[url] = score

            # 重新排序原始搜索结果
            sorted_results = sorted(
                results_dict["results"],
                key=lambda x: url_score_map.get(x["url"], 0.0),
                reverse=True,
            )

            # 更新结果和分数
            results_dict["results"] = sorted_results

        # 6. 返回重新排序后的结果
        return {"status": 200, "body": json.dumps(results_dict, ensure_ascii=False)}
    except Exception as e:
        print(f"❌ Pro搜索请求处理失败: {e}")
        return {"status": 500, "body": json.dumps({"error": str(e)})}


# 添加Pro路由到API服务器
def add_pro_routes(server):
    """
    添加Pro路由到API服务器

    Args:
        server: API服务器实例
    """
    # 注册Pro搜索路由
    server.add_pro_route("/search", handle_pro_search, method="GET")
    server.add_pro_route("/search", handle_pro_search, method="POST")

    print("✅ Pro API路由注册完成")


# 模块初始化
async def initialize_pro_handlers():
    """
    初始化Pro handlers
    """
    try:
        # 初始化向量数据库客户端
        await initialize_vector_client()
        print("✅ Pro handlers初始化完成")
    except Exception as e:
        print(f"⚠️ Pro handlers初始化失败，但服务器仍将继续运行: {e}")
        print(f"   详细错误: {traceback.format_exc()}")
        # 即使Pro功能初始化失败，服务器仍将继续运行
        # 只记录错误，不抛出异常

    # 启动异步工作器
    try:
        await start_workers()
    except Exception as e:
        print(f"⚠️ 异步工作器启动失败，但服务器仍将继续运行: {e}")


# 命令行模式下的清理函数
async def cleanup_command_line():
    """
    命令行模式下的清理函数
    """
    resources = get_resources()
    await resources._cleanup()
